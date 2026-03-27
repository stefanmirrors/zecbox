//! PF (packet filter) anchor management for Shield Mode.
//! Loads/flushes rules in the `com.zecbox.shield` anchor.

use std::collections::BTreeSet;
use std::ffi::CStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

const ANCHOR_NAME: &str = "com.zecbox.shield";
/// Root-owned directory for temp PF config files (avoids symlink attacks in /tmp)
const SECURE_TMPDIR: &str = "/var/run/com.zecbox";

fn ensure_secure_tmpdir() {
    let _ = fs::create_dir_all(SECURE_TMPDIR);
    let _ = fs::set_permissions(SECURE_TMPDIR, fs::Permissions::from_mode(0o700));
}

/// Dynamically enumerate active, non-loopback network interfaces via getifaddrs().
fn get_active_interfaces() -> Vec<String> {
    let mut ifaces = BTreeSet::new();
    unsafe {
        let mut ifaddr_ptr: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifaddr_ptr) != 0 {
            log::warn!("getifaddrs() failed, falling back to common interfaces");
            return vec![
                "en0", "en1", "en2", "en3", "en4", "en5",
                "utun0", "utun1", "utun2", "utun3",
            ].into_iter().map(String::from).collect();
        }

        let mut current = ifaddr_ptr;
        while !current.is_null() {
            let flags = (*current).ifa_flags;
            let up = (flags & libc::IFF_UP as u32) != 0;
            let running = (flags & libc::IFF_RUNNING as u32) != 0;
            let loopback = (flags & libc::IFF_LOOPBACK as u32) != 0;

            if up && running && !loopback {
                if let Ok(name) = CStr::from_ptr((*current).ifa_name).to_str() {
                    ifaces.insert(name.to_string());
                }
            }
            current = (*current).ifa_next;
        }
        libc::freeifaddrs(ifaddr_ptr);
    }
    ifaces.into_iter().collect()
}

/// Generate PF rules that redirect outbound Zcash P2P traffic through the transparent proxy.
fn generate_rules(redir_port: u16) -> String {
    let interfaces = get_active_interfaces();
    log::info!("Generating PF rules for interfaces: {:?}", interfaces);

    let mut rules = String::new();

    // Redirect rule on loopback (translation rule — goes in anchor's rdr section)
    rules.push_str(&format!(
        "rdr on lo0 proto tcp from any to any port 8233 -> 127.0.0.1 port {}\n",
        redir_port
    ));

    // Route-to rules for each active interface (forces traffic through lo0 for rdr to catch)
    for iface in &interfaces {
        rules.push_str(&format!(
            "pass out on {} route-to (lo0 127.0.0.1) proto tcp from any to any port 8233 no state\n",
            iface
        ));
    }

    // Catch-all: block any port 8233 traffic that wasn't matched by a route-to rule above.
    // This prevents clearnet leaks if a new interface appears after rules were generated.
    rules.push_str("block out proto tcp from any to any port 8233\n");

    rules
}

/// Ensure our anchor is referenced in the main PF config.
/// We read /etc/pf.conf directly (the authoritative source) and insert our
/// anchor declarations in the correct positions, preserving all existing rules
/// including scrub-anchor, dummynet-anchor, nat-anchor, and load anchor lines
/// that pfctl -s queries would not return.
fn ensure_anchor_registered() -> Result<(), String> {
    let rdr_anchor_line = format!("rdr-anchor \"{}\"", ANCHOR_NAME);
    let anchor_line = format!("anchor \"{}\"", ANCHOR_NAME);

    // Read the actual system PF config
    let pf_conf = fs::read_to_string("/etc/pf.conf")
        .map_err(|e| format!("Failed to read /etc/pf.conf: {}", e))?;

    // Check if already registered
    if pf_conf.contains(&rdr_anchor_line) && pf_conf.contains(&anchor_line) {
        return Ok(());
    }

    // Insert our anchors at the correct positions in the existing config.
    // PF ordering: scrub → nat → rdr → dummynet → anchor → load anchor
    // We insert rdr-anchor after the last rdr-anchor line,
    // and anchor after the last anchor line (but before load anchor).
    let mut lines: Vec<String> = pf_conf.lines().map(|l| l.to_string()).collect();
    let mut rdr_inserted = pf_conf.contains(&rdr_anchor_line);
    let mut anchor_inserted = pf_conf.contains(&anchor_line);

    if !rdr_inserted {
        // Find the last rdr-anchor line and insert after it
        let mut insert_pos = None;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("rdr-anchor") || trimmed.starts_with("rdr ") {
                insert_pos = Some(i + 1);
            }
        }
        // If no rdr-anchor found, insert before the first anchor line
        if insert_pos.is_none() {
            for (i, line) in lines.iter().enumerate() {
                if line.trim().starts_with("anchor") {
                    insert_pos = Some(i);
                    break;
                }
            }
        }
        if let Some(pos) = insert_pos {
            lines.insert(pos, rdr_anchor_line);
            rdr_inserted = true;
        }
    }

    if !anchor_inserted {
        // Find the last "anchor" line (not rdr-anchor/nat-anchor/etc) and insert after it
        let mut insert_pos = None;
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("anchor ") && !trimmed.starts_with("anchor \"com.zecbox") {
                insert_pos = Some(i + 1);
            }
        }
        // If no anchor found, append before load anchor or at end
        if insert_pos.is_none() {
            for (i, line) in lines.iter().enumerate() {
                if line.trim().starts_with("load anchor") {
                    insert_pos = Some(i);
                    break;
                }
            }
        }
        if let Some(pos) = insert_pos {
            lines.insert(pos, anchor_line);
            anchor_inserted = true;
        }
    }

    if !rdr_inserted || !anchor_inserted {
        return Err("Could not determine where to insert PF anchor declarations".into());
    }

    let new_conf = lines.join("\n") + "\n";

    ensure_secure_tmpdir();
    let main_path = format!("{}/pf-main.conf", SECURE_TMPDIR);
    fs::write(&main_path, &new_conf)
        .map_err(|e| format!("Failed to write PF config: {}", e))?;

    let output = Command::new("pfctl")
        .args(["-f", &main_path])
        .output()
        .map_err(|e| format!("Failed to load PF rules: {}", e))?;

    let _ = fs::remove_file(&main_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let real_errors: Vec<&str> = stderr
            .lines()
            .filter(|l| {
                !l.contains("pf enabled")
                    && !l.contains("pf already enabled")
                    && !l.contains("ALTQ")
                    && !l.trim().is_empty()
            })
            .collect();
        if !real_errors.is_empty() {
            return Err(format!("Failed to register anchor: {}", real_errors.join("\n")));
        }
    }

    log::info!("PF anchor {} registered in main ruleset", ANCHOR_NAME);
    Ok(())
}

/// Load PF anchor rules to redirect Zcash P2P traffic through the transparent proxy.
pub fn enable(redir_port: u16) -> Result<(), String> {
    let rules = generate_rules(redir_port);

    // Write rules to a secure temp file
    ensure_secure_tmpdir();
    let rules_path = format!("{}/pf-shield.conf", SECURE_TMPDIR);
    fs::write(&rules_path, &rules)
        .map_err(|e| format!("Failed to write PF rules: {}", e))?;

    // Ensure PF is enabled
    let _ = Command::new("pfctl").args(["-e"]).output();

    // Register our anchor in the main PF config
    ensure_anchor_registered()?;

    // Load rules into our anchor
    let output = Command::new("pfctl")
        .args(["-a", ANCHOR_NAME, "-f", &rules_path])
        .output()
        .map_err(|e| format!("Failed to run pfctl: {}", e))?;

    // Clean up temp file
    let _ = fs::remove_file(&rules_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let real_errors: Vec<&str> = stderr
            .lines()
            .filter(|l| !l.contains("pf enabled") && !l.contains("pf already enabled") && !l.contains("ALTQ") && !l.trim().is_empty())
            .collect();
        if !real_errors.is_empty() {
            return Err(format!("pfctl failed: {}", real_errors.join("\n")));
        }
    }

    log::info!("PF anchor {} loaded with redirect to port {}", ANCHOR_NAME, redir_port);
    Ok(())
}

/// Flush all rules from the PF anchor.
pub fn disable() -> Result<(), String> {
    let output = Command::new("pfctl")
        .args(["-a", ANCHOR_NAME, "-F", "all"])
        .output()
        .map_err(|e| format!("Failed to run pfctl: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let real_errors: Vec<&str> = stderr
            .lines()
            .filter(|l| !l.contains("pf enabled") && !l.trim().is_empty())
            .collect();
        if !real_errors.is_empty() {
            return Err(format!("pfctl flush failed: {}", real_errors.join("\n")));
        }
    }

    log::info!("PF anchor {} flushed", ANCHOR_NAME);
    Ok(())
}

/// Check if the PF anchor has rules loaded.
pub fn is_enabled() -> bool {
    let output = Command::new("pfctl")
        .args(["-a", ANCHOR_NAME, "-s", "rules"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            !stdout.trim().is_empty()
        }
        Err(_) => false,
    }
}
