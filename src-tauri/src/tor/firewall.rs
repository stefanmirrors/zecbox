//! Client for communicating with the ZecBox Firewall Helper daemon.
//! The helper manages firewall rules and a transparent SOCKS5 redirector.
//! macOS: PF rules via LaunchDaemon
//! Linux: iptables rules via systemd service
//! Windows: not yet supported (Shield Mode coming soon)

#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::process::Command;
#[cfg(unix)]
use std::time::Duration;

use tauri::AppHandle;

#[cfg(unix)]
const SOCKET_PATH: &str = "/var/run/com.zecbox.firewall.sock";

#[cfg(unix)]
/// Escape a string for safe use inside a single-quoted shell argument.
fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(unix)]
/// Must match HELPER_VERSION in firewall-helper/src/main.rs.
const REQUIRED_HELPER_VERSION: &str = "2";
#[cfg(unix)]
const HELPER_BIN_NAME: &str = "zecbox-firewall-helper";

#[cfg(target_os = "macos")]
const HELPER_INSTALL_PATH: &str = "/Library/PrivilegedHelperTools/com.zecbox.firewall-helper";
#[cfg(target_os = "macos")]
const PLIST_INSTALL_PATH: &str = "/Library/LaunchDaemons/com.zecbox.firewall.plist";
#[cfg(target_os = "macos")]
const PLIST_LABEL: &str = "com.zecbox.firewall";

#[cfg(target_os = "linux")]
const HELPER_INSTALL_PATH: &str = "/usr/local/bin/zecbox-firewall-helper";
#[cfg(target_os = "linux")]
const SERVICE_INSTALL_PATH: &str = "/etc/systemd/system/com.zecbox.firewall.service";
#[cfg(target_os = "linux")]
const SERVICE_NAME: &str = "com.zecbox.firewall";

// ===== Public API =====

/// Check if the firewall helper daemon is installed, reachable, and up to date.
#[cfg(unix)]
pub fn is_helper_installed() -> bool {
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
            if reader.read_line(&mut line).is_ok() && line.contains("\"ok\":true") {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) {
                    let version = v["version"].as_str().unwrap_or("1");
                    if version == REQUIRED_HELPER_VERSION {
                        return true;
                    }
                    log::info!(
                        "Firewall helper version mismatch: got {}, need {}",
                        version, REQUIRED_HELPER_VERSION
                    );
                    return false;
                }
                return false;
            }
        }
    }

    false
}

#[cfg(windows)]
pub fn is_helper_installed() -> bool {
    if let Ok(response) = windows_send_command_raw("status") {
        if response.contains("\"ok\":true") {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&response) {
                let version = v["version"].as_str().unwrap_or("1");
                if version == REQUIRED_HELPER_VERSION_WIN {
                    return true;
                }
                log::info!(
                    "Firewall helper version mismatch: got {}, need {}",
                    version, REQUIRED_HELPER_VERSION_WIN
                );
            }
        }
    }
    false
}

/// Install the firewall helper daemon. Requires admin password (one-time).
#[cfg(unix)]
pub fn install_helper(app_handle: &AppHandle) -> Result<(), String> {
    let helper_src = resolve_helper_binary_path(app_handle);
    if !helper_src.exists() {
        return Err(format!(
            "Firewall helper binary not found at {:?}",
            helper_src
        ));
    }

    #[cfg(target_os = "macos")]
    install_helper_macos(&helper_src)?;

    #[cfg(target_os = "linux")]
    install_helper_linux(&helper_src)?;

    // Wait briefly for daemon to start
    std::thread::sleep(Duration::from_secs(1));

    if !is_helper_installed() {
        return Err("Helper installed but daemon not responding. Try restarting.".into());
    }

    log::info!("Firewall helper installed successfully");
    Ok(())
}

#[cfg(windows)]
pub fn install_helper(app_handle: &AppHandle) -> Result<(), String> {
    let helper_src = crate::platform::resolve_sidecar_path(app_handle, HELPER_BIN_NAME_WIN);
    if !helper_src.exists() {
        return Err(format!(
            "Firewall helper binary not found at {:?}",
            helper_src
        ));
    }

    install_helper_windows(&helper_src)?;

    // Wait briefly for service to start
    std::thread::sleep(std::time::Duration::from_secs(2));

    if !is_helper_installed() {
        return Err("Helper installed but service not responding. Try restarting.".into());
    }

    log::info!("Firewall helper installed successfully (Windows Service)");
    Ok(())
}

/// Send enable command to the firewall helper.
#[cfg(unix)]
pub fn enable_firewall() -> Result<(), String> {
    send_command("enable")
}

#[cfg(windows)]
pub fn enable_firewall() -> Result<(), String> {
    windows_send_command("enable")
}

/// Send disable command to the firewall helper.
#[cfg(unix)]
pub fn disable_firewall() -> Result<(), String> {
    send_command("disable")
}

#[cfg(windows)]
pub fn disable_firewall() -> Result<(), String> {
    windows_send_command("disable")
}

/// Query firewall status from the helper.
#[cfg(unix)]
pub fn firewall_status() -> Result<(bool, bool), String> {
    let response = send_command_raw("status")?;
    let v: serde_json::Value =
        serde_json::from_str(&response).map_err(|e| format!("Invalid status response: {}", e))?;

    let enabled = v["enabled"].as_bool().unwrap_or(false);
    let redirector = v["redirector_running"].as_bool().unwrap_or(false);
    Ok((enabled, redirector))
}

#[cfg(windows)]
pub fn firewall_status() -> Result<(bool, bool), String> {
    let response = windows_send_command_raw("status")?;
    let v: serde_json::Value =
        serde_json::from_str(&response).map_err(|e| format!("Invalid status response: {}", e))?;

    let enabled = v["enabled"].as_bool().unwrap_or(false);
    let redirector = v["redirector_running"].as_bool().unwrap_or(false);
    Ok((enabled, redirector))
}

// ===== Unix-only internals =====

#[cfg(unix)]
fn send_command(cmd: &str) -> Result<(), String> {
    let response = send_command_raw(cmd)?;
    let v: serde_json::Value =
        serde_json::from_str(&response).map_err(|_| "Firewall helper returned an unexpected response. Try restarting zecbox.".to_string())?;

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

#[cfg(unix)]
fn send_command_raw(cmd: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(SOCKET_PATH)
        .map_err(|e| format!("Cannot connect to firewall helper at {}: {}", SOCKET_PATH, e))?;

    // Set timeouts on the raw stream BEFORE wrapping in BufReader
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Failed to set read timeout: {}", e))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(5)))
        .map_err(|e| format!("Failed to set write timeout: {}", e))?;

    let msg = format!("{{\"cmd\":\"{}\"}}\n", cmd);
    stream
        .write_all(msg.as_bytes())
        .map_err(|e| format!("Failed to send command to firewall helper: {}", e))?;

    // Read response with the timeout set on the underlying stream
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut {
                "Firewall helper did not respond within 5 seconds. It may need to be reinstalled.".to_string()
            } else {
                format!("Failed to read response from firewall helper: {}", e)
            }
        })?;

    if line.is_empty() {
        return Err("Firewall helper returned empty response. It may need to be reinstalled.".into());
    }

    Ok(line)
}

#[cfg(unix)]
fn resolve_helper_binary_path(app_handle: &AppHandle) -> std::path::PathBuf {
    crate::platform::resolve_sidecar_path(app_handle, HELPER_BIN_NAME)
}

#[cfg(target_os = "macos")]
fn install_helper_macos(helper_src: &std::path::Path) -> Result<(), String> {
    let plist_content = generate_plist();

    let plist_tmp = format!(
        "{}/com.zecbox.firewall.{}.plist",
        std::env::temp_dir().display(),
        std::process::id()
    );
    std::fs::write(&plist_tmp, &plist_content)
        .map_err(|e| format!("Failed to write plist: {}", e))?;

    let esc_helper_src = shell_escape(&helper_src.display().to_string());
    let esc_install_path = shell_escape(HELPER_INSTALL_PATH);
    let esc_plist_tmp = shell_escape(&plist_tmp);
    let esc_plist_install = shell_escape(PLIST_INSTALL_PATH);
    let script = format!(
        r#"
mkdir -p /Library/PrivilegedHelperTools
cp {src} {dst}
chown root:wheel {dst}
chmod 755 {dst}
cp {plist_src} {plist_dst}
chown root:wheel {plist_dst}
chmod 644 {plist_dst}
launchctl bootout system/{label_raw} 2>/dev/null || true
launchctl bootstrap system {plist_dst}
"#,
        src = esc_helper_src,
        dst = esc_install_path,
        plist_src = esc_plist_tmp,
        plist_dst = esc_plist_install,
        label_raw = PLIST_LABEL,
    );

    let script_tmp = format!(
        "{}/com.zecbox.install.{}.sh",
        std::env::temp_dir().display(),
        std::process::id()
    );
    std::fs::write(&script_tmp, &script)
        .map_err(|e| format!("Failed to write install script: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script_tmp, std::fs::Permissions::from_mode(0o700));
    }

    let output = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "do shell script \"{}\" with administrator privileges",
            script_tmp.replace('\\', "\\\\").replace('"', "\\\"")
        ))
        .output()
        .map_err(|e| format!("Failed to run osascript: {}", e))?;

    let _ = std::fs::remove_file(&plist_tmp);
    let _ = std::fs::remove_file(&script_tmp);

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

    Ok(())
}

#[cfg(target_os = "linux")]
fn install_helper_linux(helper_src: &std::path::Path) -> Result<(), String> {
    let service_content = generate_systemd_service();

    let service_tmp = format!(
        "{}/com.zecbox.firewall.{}.service",
        std::env::temp_dir().display(),
        std::process::id()
    );
    std::fs::write(&service_tmp, &service_content)
        .map_err(|e| format!("Failed to write service file: {}", e))?;

    let esc_helper_src = shell_escape(&helper_src.display().to_string());
    let esc_install_path = shell_escape(HELPER_INSTALL_PATH);
    let esc_service_tmp = shell_escape(&service_tmp);
    let esc_service_install = shell_escape(SERVICE_INSTALL_PATH);
    let script = format!(
        r#"
cp {src} {dst}
chown root:root {dst}
chmod 755 {dst}
cp {svc_src} {svc_dst}
chown root:root {svc_dst}
chmod 644 {svc_dst}
systemctl daemon-reload
systemctl enable {svc_name}
systemctl restart {svc_name}
"#,
        src = esc_helper_src,
        dst = esc_install_path,
        svc_src = esc_service_tmp,
        svc_dst = esc_service_install,
        svc_name = SERVICE_NAME,
    );

    let script_tmp = format!(
        "{}/com.zecbox.install.{}.sh",
        std::env::temp_dir().display(),
        std::process::id()
    );
    std::fs::write(&script_tmp, &script)
        .map_err(|e| format!("Failed to write install script: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&script_tmp, std::fs::Permissions::from_mode(0o700));
    }

    // Try pkexec first (graphical), fall back to informing user about sudo
    let output = Command::new("pkexec")
        .arg("/bin/sh")
        .arg(&script_tmp)
        .output();

    let _ = std::fs::remove_file(&service_tmp);
    let _ = std::fs::remove_file(&script_tmp);

    match output {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr);
            if stderr.contains("dismissed") || stderr.contains("Not authorized") {
                Err("Installation canceled by user".into())
            } else {
                Err(format!(
                    "System helper installation failed. Details: {}",
                    stderr.lines().next().unwrap_or("Unknown error")
                ))
            }
        }
        Err(_) => {
            Err("pkexec not found. Install polkit or run the install script manually with sudo.".into())
        }
    }
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "linux")]
fn generate_systemd_service() -> String {
    format!(
        "[Unit]\n\
         Description=ZecBox Firewall Helper\n\
         After=network.target\n\
         \n\
         [Service]\n\
         Type=simple\n\
         ExecStart={}\n\
         Restart=always\n\
         RestartSec=5\n\
         StandardOutput=journal\n\
         StandardError=journal\n\
         \n\
         [Install]\n\
         WantedBy=multi-user.target\n",
        HELPER_INSTALL_PATH
    )
}

// ===== Windows-only internals =====

#[cfg(target_os = "windows")]
const PIPE_NAME: &str = r"\\.\pipe\com.zecbox.firewall";
#[cfg(target_os = "windows")]
const REQUIRED_HELPER_VERSION_WIN: &str = "2";
#[cfg(target_os = "windows")]
const HELPER_BIN_NAME_WIN: &str = "zecbox-firewall-helper";
#[cfg(target_os = "windows")]
const HELPER_INSTALL_DIR: &str = r"C:\Program Files\ZecBox";
#[cfg(target_os = "windows")]
const SERVICE_NAME: &str = "ZecBoxFirewall";

#[cfg(target_os = "windows")]
fn windows_send_command(cmd: &str) -> Result<(), String> {
    let response = windows_send_command_raw(cmd)?;
    let v: serde_json::Value = serde_json::from_str(&response)
        .map_err(|_| "Firewall helper returned an unexpected response. Try restarting zecbox.".to_string())?;

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

#[cfg(target_os = "windows")]
fn windows_send_command_raw(cmd: &str) -> Result<String, String> {
    use std::fs::OpenOptions;
    use std::io::{BufRead, BufReader, Write};
    use std::time::Duration;

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)
        .map_err(|e| format!("Cannot connect to firewall helper at {}: {}", PIPE_NAME, e))?;

    let msg = format!("{{\"cmd\":\"{}\"}}\n", cmd);
    file.write_all(msg.as_bytes())
        .map_err(|e| format!("Failed to send command to firewall helper: {}", e))?;
    file.flush()
        .map_err(|e| format!("Failed to flush command: {}", e))?;

    let mut reader = BufReader::new(&file);
    let mut line = String::new();
    reader
        .read_line(&mut line)
        .map_err(|e| format!("Failed to read response from firewall helper: {}", e))?;

    if line.is_empty() {
        return Err("Firewall helper returned empty response. It may need to be reinstalled.".into());
    }

    Ok(line)
}

#[cfg(target_os = "windows")]
fn install_helper_windows(helper_src: &std::path::Path) -> Result<(), String> {
    use std::process::Command;

    let helper_dst = format!(r"{}\zecbox-firewall-helper.exe", HELPER_INSTALL_DIR);

    // Build a PowerShell script that:
    // 1. Creates the install directory
    // 2. Copies the helper binary
    // 3. Stops the old service if it exists
    // 4. Removes the old service if it exists
    // 5. Creates and starts the new service
    // WinDivert DLL and driver should be next to the helper binary
    let helper_dir = helper_src.parent().unwrap_or(std::path::Path::new("."));
    let windivert_dll = helper_dir.join("WinDivert.dll");
    let windivert_sys = helper_dir.join("WinDivert64.sys");

    let mut copy_windivert = String::new();
    if windivert_dll.exists() {
        copy_windivert.push_str(&format!(
            "Copy-Item -Force '{}' '{install_dir}\\WinDivert.dll'\n",
            windivert_dll.display(),
            install_dir = HELPER_INSTALL_DIR,
        ));
    }
    if windivert_sys.exists() {
        copy_windivert.push_str(&format!(
            "Copy-Item -Force '{}' '{install_dir}\\WinDivert64.sys'\n",
            windivert_sys.display(),
            install_dir = HELPER_INSTALL_DIR,
        ));
    }

    let script = format!(
        r#"
New-Item -ItemType Directory -Force -Path '{install_dir}' | Out-Null
Copy-Item -Force '{src}' '{dst}'
{copy_wd}$svc = Get-Service -Name '{svc_name}' -ErrorAction SilentlyContinue
if ($svc) {{
    Stop-Service -Name '{svc_name}' -Force -ErrorAction SilentlyContinue
    sc.exe delete '{svc_name}' | Out-Null
    Start-Sleep -Seconds 1
}}
sc.exe create '{svc_name}' binPath= '{dst}' DisplayName= 'ZecBox Firewall Helper' start= demand | Out-Null
sc.exe start '{svc_name}' | Out-Null
"#,
        install_dir = HELPER_INSTALL_DIR,
        src = helper_src.display(),
        dst = helper_dst,
        copy_wd = copy_windivert,
        svc_name = SERVICE_NAME,
    );

    let script_tmp = format!(
        r"{}\com.zecbox.install.{}.ps1",
        std::env::temp_dir().display(),
        std::process::id()
    );
    std::fs::write(&script_tmp, &script)
        .map_err(|e| format!("Failed to write install script: {}", e))?;

    // Elevate via PowerShell Start-Process with -Verb RunAs (triggers UAC)
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File','{}' -Verb RunAs -Wait",
                script_tmp.replace('\'', "''")
            ),
        ])
        .output()
        .map_err(|e| format!("Failed to run installer: {}", e))?;

    let _ = std::fs::remove_file(&script_tmp);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("canceled") || stderr.contains("elevation") {
            return Err("Installation canceled by user".into());
        }
        return Err(format!(
            "System helper installation failed. You may need to grant administrator access. Details: {}",
            stderr.lines().next().unwrap_or("Unknown error")
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    #[test]
    fn test_generate_systemd_service() {
        let svc = super::generate_systemd_service();
        assert!(svc.contains("[Unit]"));
        assert!(svc.contains("[Service]"));
        assert!(svc.contains("[Install]"));
        assert!(svc.contains("zecbox-firewall-helper"));
    }
}
