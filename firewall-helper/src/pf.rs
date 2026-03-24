//! PF (packet filter) anchor management for Shield Mode.
//! Loads/flushes rules in the `com.zecbox.shield` anchor.

use std::fs;
use std::process::Command;

const ANCHOR_NAME: &str = "com.zecbox.shield";

/// Generate PF rules that redirect outbound Zcash P2P traffic through the transparent proxy.
fn generate_rules(redir_port: u16) -> String {
    // rdr on lo0: redirects loopback-routed traffic to our transparent proxy.
    // pass out route-to lo0: forces outbound port 8233 TCP through loopback
    //   so the rdr rule catches it.
    // We enumerate common macOS interfaces since `!lo0` negation may not work
    //   on Apple's PF fork. Each rule forces matching outbound traffic through lo0.
    let interfaces = ["en0", "en1", "en2", "en3", "en4", "en5", "utun0", "utun1", "utun2", "utun3"];

    let mut rules = String::new();

    // Redirect rule on loopback
    rules.push_str(&format!(
        "rdr on lo0 proto tcp from any to any port 8233 -> 127.0.0.1 port {}\n",
        redir_port
    ));

    // Route-to rules for each interface
    for iface in &interfaces {
        rules.push_str(&format!(
            "pass out on {} route-to (lo0 127.0.0.1) proto tcp from any to any port 8233 no state\n",
            iface
        ));
    }

    rules
}

/// Ensure our anchor is referenced in the main PF config.
/// macOS PF requires anchors to be declared in the main ruleset.
fn ensure_anchor_registered() -> Result<(), String> {
    // Check if our anchor is already in the main rules
    let output = Command::new("pfctl")
        .args(["-s", "rules"])
        .output()
        .map_err(|e| format!("Failed to query PF rules: {}", e))?;

    let rules = String::from_utf8_lossy(&output.stdout);
    let rdr_anchor = format!("rdr-anchor \"{}\"", ANCHOR_NAME);
    let anchor = format!("anchor \"{}\"", ANCHOR_NAME);

    if rules.contains(&rdr_anchor) && rules.contains(&anchor) {
        return Ok(());
    }

    // Load a main config that includes our anchor alongside existing rules
    let main_rules = format!(
        "{}\n{}\n{}\n",
        rdr_anchor,
        anchor,
        rules.trim()
    );

    let main_path = "/tmp/zecbox-pf-main.conf";
    fs::write(main_path, &main_rules)
        .map_err(|e| format!("Failed to write main PF rules: {}", e))?;

    let output = Command::new("pfctl")
        .args(["-f", main_path])
        .output()
        .map_err(|e| format!("Failed to load main PF rules: {}", e))?;

    let _ = fs::remove_file(main_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let real_errors: Vec<&str> = stderr
            .lines()
            .filter(|l| {
                !l.contains("pf enabled")
                    && !l.contains("pf already enabled")
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

    // Write rules to a temp file
    let rules_path = "/tmp/zecbox-pf-shield.conf";
    fs::write(rules_path, &rules)
        .map_err(|e| format!("Failed to write PF rules: {}", e))?;

    // Ensure PF is enabled
    let _ = Command::new("pfctl").args(["-e"]).output();

    // Register our anchor in the main PF config
    ensure_anchor_registered()?;

    // Load rules into our anchor
    let output = Command::new("pfctl")
        .args(["-a", ANCHOR_NAME, "-f", rules_path])
        .output()
        .map_err(|e| format!("Failed to run pfctl: {}", e))?;

    // Clean up temp file
    let _ = fs::remove_file(rules_path);

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // pfctl prints "pf enabled" to stderr even on success, filter that out
        let real_errors: Vec<&str> = stderr
            .lines()
            .filter(|l| !l.contains("pf enabled") && !l.contains("pf already enabled") && !l.trim().is_empty())
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
    // Flush all rules in our anchor
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
