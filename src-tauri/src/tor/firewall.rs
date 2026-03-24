//! Client for communicating with the ZecBox Firewall Helper daemon.
//! The helper manages PF rules and a transparent SOCKS5 redirector.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

use tauri::{AppHandle, Manager};

const SOCKET_PATH: &str = "/var/run/com.zecbox.firewall.sock";
const HELPER_BIN_NAME: &str = "zecbox-firewall-helper";
const HELPER_INSTALL_PATH: &str = "/Library/PrivilegedHelperTools/com.zecbox.firewall-helper";
const PLIST_INSTALL_PATH: &str = "/Library/LaunchDaemons/com.zecbox.firewall.plist";
const PLIST_LABEL: &str = "com.zecbox.firewall";

/// Check if the firewall helper daemon is installed and reachable.
pub fn is_helper_installed() -> bool {
    // Check if the socket exists and is connectable
    if let Ok(mut stream) = UnixStream::connect(SOCKET_PATH) {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .ok();
        if stream.write_all(b"{\"cmd\":\"status\"}\n").is_ok() {
            let mut reader = BufReader::new(&stream);
            let mut line = String::new();
            if reader.read_line(&mut line).is_ok() {
                return line.contains("\"ok\":true");
            }
        }
    }

    // Fallback: check if the binary and plist exist
    Path::new(HELPER_INSTALL_PATH).exists() && Path::new(PLIST_INSTALL_PATH).exists()
}

/// Install the firewall helper daemon. Requires admin password (one-time).
pub fn install_helper(app_handle: &AppHandle) -> Result<(), String> {
    // Resolve the helper binary path (bundled with the app)
    let helper_src = resolve_helper_binary_path(app_handle);
    if !helper_src.exists() {
        return Err(format!(
            "Firewall helper binary not found at {:?}",
            helper_src
        ));
    }

    let plist_content = generate_plist();

    // Write plist to a temp file
    let plist_tmp = "/tmp/com.zecbox.firewall.plist";
    std::fs::write(plist_tmp, &plist_content)
        .map_err(|e| format!("Failed to write plist: {}", e))?;

    // Build the installation script
    let script = format!(
        r#"
mkdir -p /Library/PrivilegedHelperTools
cp '{}' '{}'
chown root:wheel '{}'
chmod 755 '{}'
cp '{}' '{}'
chown root:wheel '{}'
chmod 644 '{}'
launchctl bootout system/{} 2>/dev/null || true
launchctl bootstrap system '{}'
"#,
        helper_src.display(),
        HELPER_INSTALL_PATH,
        HELPER_INSTALL_PATH,
        HELPER_INSTALL_PATH,
        plist_tmp,
        PLIST_INSTALL_PATH,
        PLIST_INSTALL_PATH,
        PLIST_INSTALL_PATH,
        PLIST_LABEL,
        PLIST_INSTALL_PATH,
    );

    // Execute with admin privileges via osascript
    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "do shell script \"{}\" with administrator privileges",
            script.replace('\\', "\\\\").replace('"', "\\\"")
        ))
        .output()
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    // Clean up temp plist
    let _ = std::fs::remove_file(plist_tmp);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("User canceled") || stderr.contains("(-128)") {
            return Err("Installation canceled by user".into());
        }
        return Err(format!(
            "System helper installation failed. You may need to grant administrator access. Details: {}",
            stderr.lines().next().unwrap_or("Unknown error")
        ));
    }

    // Wait briefly for daemon to start
    std::thread::sleep(Duration::from_secs(1));

    if !is_helper_installed() {
        return Err("Helper installed but daemon not responding. Try restarting.".into());
    }

    log::info!("Firewall helper installed successfully");
    Ok(())
}

/// Send enable command to the firewall helper.
pub fn enable_firewall() -> Result<(), String> {
    send_command("enable")
}

/// Send disable command to the firewall helper.
pub fn disable_firewall() -> Result<(), String> {
    send_command("disable")
}

/// Query firewall status from the helper.
pub fn firewall_status() -> Result<(bool, bool), String> {
    let response = send_command_raw("status")?;
    let v: serde_json::Value =
        serde_json::from_str(&response).map_err(|e| format!("Invalid status response: {}", e))?;

    let enabled = v["enabled"].as_bool().unwrap_or(false);
    let redirector = v["redirector_running"].as_bool().unwrap_or(false);
    Ok((enabled, redirector))
}

fn send_command(cmd: &str) -> Result<(), String> {
    let response = send_command_raw(cmd)?;
    let v: serde_json::Value =
        serde_json::from_str(&response).map_err(|_| "Firewall helper returned an unexpected response. Try restarting ZecBox.".to_string())?;

    if v["ok"].as_bool() == Some(true) {
        Ok(())
    } else {
        let err = v["error"]
            .as_str()
            .unwrap_or("Unknown error")
            .to_string();
        Err(err)
    }
}

fn send_command_raw(cmd: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(SOCKET_PATH)
        .map_err(|e| format!("Cannot connect to firewall helper at {}: {}", SOCKET_PATH, e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .ok();
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .ok();

    let msg = format!("{{\"cmd\":\"{}\"}}\n", cmd);
    stream
        .write_all(msg.as_bytes())
        .map_err(|e| format!("Failed to send command: {}", e))?;

    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read response: {}", e))?;

    Ok(line)
}

fn resolve_helper_binary_path(app_handle: &AppHandle) -> std::path::PathBuf {
    let target_triple = "aarch64-apple-darwin";
    let binary_name_with_triple = format!("{}-{}", HELPER_BIN_NAME, target_triple);

    // Dev mode: look in src-tauri/binaries/
    if cfg!(debug_assertions) {
        let dev_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(&binary_name_with_triple);
        if dev_path.exists() {
            return dev_path;
        }
    }

    // Production: alongside the main executable
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()));

    if let Some(ref dir) = exe_dir {
        let prod_path = dir.join(HELPER_BIN_NAME);
        if prod_path.exists() {
            return prod_path;
        }
        let prod_path = dir.join(&binary_name_with_triple);
        if prod_path.exists() {
            return prod_path;
        }
    }

    if let Ok(resource_dir) = app_handle.path().resource_dir() {
        let prod_path = resource_dir.join(HELPER_BIN_NAME);
        if prod_path.exists() {
            return prod_path;
        }
    }

    exe_dir
        .unwrap_or_default()
        .join(HELPER_BIN_NAME)
}

fn generate_plist() -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>Program</key>
    <string>{}</string>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>/var/log/com.zecbox.firewall.log</string>
    <key>StandardErrorPath</key>
    <string>/var/log/com.zecbox.firewall.log</string>
</dict>
</plist>
"#,
        PLIST_LABEL, HELPER_INSTALL_PATH
    )
}
