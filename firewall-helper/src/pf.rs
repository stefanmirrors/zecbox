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

/// Classify a PF rule line as translation (rdr/nat/binat) or filter (everything else).
fn is_translation_rule(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("rdr")
        || trimmed.starts_with("nat")
        || trimmed.starts_with("binat")
        || trimmed.starts_with("rdr-anchor")
        || trimmed.starts_with("nat-anchor")
}

/// Ensure our anchor is referenced in the main PF config.
/// macOS PF requires anchors to be declared in the main ruleset.
/// PF enforces strict rule ordering: translation rules (rdr/nat) before filter rules (pass/block/anchor).
fn ensure_anchor_registered() -> Result<(), String> {
    let rdr_anchor = format!("rdr-anchor \"{}\"", ANCHOR_NAME);
    let anchor = format!("anchor \"{}\"", ANCHOR_NAME);

    // Query existing NAT/translation rules and filter rules separately
    let nat_output = Command::new("pfctl")
        .args(["-s", "nat"])
        .output()
        .map_err(|e| format!("Failed to query PF NAT rules: {}", e))?;
    let nat_rules = String::from_utf8_lossy(&nat_output.stdout);

    let filter_output = Command::new("pfctl")
        .args(["-s", "rules"])
        .output()
        .map_err(|e| format!("Failed to query PF filter rules: {}", e))?;
    let filter_rules = String::from_utf8_lossy(&filter_output.stdout);

    // Check if already registered in both sections
    if nat_rules.contains(&rdr_anchor) && filter_rules.contains(&anchor) {
        return Ok(());
    }

    // Build config in correct PF order:
    // 1. Translation rules (rdr, nat, binat, rdr-anchor, nat-anchor)
    // 2. Filter rules (pass, block, anchor)
    let mut translation_lines = Vec::new();
    let mut filter_lines = Vec::new();

    // Collect existing translation rules
    for line in nat_rules.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.contains(ANCHOR_NAME) {
            translation_lines.push(line.to_string());
        }
    }

    // Collect existing filter rules, separating any misplaced translation rules
    for line in filter_rules.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.contains(ANCHOR_NAME) {
            continue;
        }
        if is_translation_rule(trimmed) {
            translation_lines.push(line.to_string());
        } else {
            filter_lines.push(line.to_string());
        }
    }

    // Build the final config
    let mut main_rules = String::new();

    // Translation section: existing + our rdr-anchor
    for line in &translation_lines {
        main_rules.push_str(line);
        main_rules.push('\n');
    }
    main_rules.push_str(&rdr_anchor);
    main_rules.push('\n');

    // Filter section: our anchor + existing
    main_rules.push_str(&anchor);
    main_rules.push('\n');
    for line in &filter_lines {
        main_rules.push_str(line);
        main_rules.push('\n');
    }

    ensure_secure_tmpdir();
    let main_path = format!("{}/pf-main.conf", SECURE_TMPDIR);
    fs::write(&main_path, &main_rules)
        .map_err(|e| format!("Failed to write main PF rules: {}", e))?;

    let output = Command::new("pfctl")
        .args(["-f", &main_path])
        .output()
        .map_err(|e| format!("Failed to load main PF rules: {}", e))?;

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
