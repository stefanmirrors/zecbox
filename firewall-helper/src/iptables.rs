//! iptables chain management for Shield Mode on Linux.
//! Creates/flushes rules in the ZECBOX_SHIELD chain.

use std::process::Command;

const CHAIN_NAME: &str = "ZECBOX_SHIELD";
const ZCASH_PORT: u16 = 8233;

/// Generate the iptables rules as a string (for testing).
pub fn generate_rules(redir_port: u16) -> String {
    format!(
        "-A {chain} -p tcp --dport {port} -j REDIRECT --to-port {redir}\n",
        chain = CHAIN_NAME,
        port = ZCASH_PORT,
        redir = redir_port,
    )
}

fn run_iptables(args: &[&str]) -> Result<(), String> {
    let output = Command::new("iptables")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run iptables: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("iptables {} failed: {}", args.join(" "), stderr.trim()))
    } else {
        Ok(())
    }
}

/// Create the ZECBOX_SHIELD chain and redirect Zcash P2P traffic to the transparent proxy.
pub fn enable(redir_port: u16) -> Result<(), String> {
    // Create chain (ignore error if it already exists)
    let _ = run_iptables(&["-t", "nat", "-N", CHAIN_NAME]);

    // Flush any existing rules in our chain
    run_iptables(&["-t", "nat", "-F", CHAIN_NAME])?;

    // Add redirect rule
    run_iptables(&[
        "-t", "nat", "-A", CHAIN_NAME,
        "-p", "tcp", "--dport", &ZCASH_PORT.to_string(),
        "-j", "REDIRECT", "--to-port", &redir_port.to_string(),
    ])?;

    // Jump from OUTPUT to our chain (remove first to avoid duplicates)
    let _ = run_iptables(&[
        "-t", "nat", "-D", "OUTPUT",
        "-p", "tcp", "--dport", &ZCASH_PORT.to_string(),
        "-j", CHAIN_NAME,
    ]);
    run_iptables(&[
        "-t", "nat", "-A", "OUTPUT",
        "-p", "tcp", "--dport", &ZCASH_PORT.to_string(),
        "-j", CHAIN_NAME,
    ])?;

    log::info!("iptables chain {} loaded with redirect to port {}", CHAIN_NAME, redir_port);
    Ok(())
}

/// Flush and remove the ZECBOX_SHIELD chain.
pub fn disable() -> Result<(), String> {
    // Remove the jump rule from OUTPUT
    let _ = run_iptables(&[
        "-t", "nat", "-D", "OUTPUT",
        "-p", "tcp", "--dport", &ZCASH_PORT.to_string(),
        "-j", CHAIN_NAME,
    ]);

    // Flush and delete the chain
    let _ = run_iptables(&["-t", "nat", "-F", CHAIN_NAME]);
    let _ = run_iptables(&["-t", "nat", "-X", CHAIN_NAME]);

    log::info!("iptables chain {} flushed and removed", CHAIN_NAME);
    Ok(())
}

/// Check if the ZECBOX_SHIELD chain has rules loaded.
pub fn is_enabled() -> bool {
    let output = Command::new("iptables")
        .args(["-t", "nat", "-L", CHAIN_NAME, "-n"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // The chain exists and has at least one rule (beyond the header lines)
            stdout.lines().count() > 2
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_rules() {
        let rules = generate_rules(9040);
        assert!(rules.contains("ZECBOX_SHIELD"));
        assert!(rules.contains("8233"));
        assert!(rules.contains("9040"));
        assert!(rules.contains("REDIRECT"));
    }
}
