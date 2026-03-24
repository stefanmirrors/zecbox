//! Sidecar process management: spawn, monitor, restart, PID tracking.

pub mod zaino;
pub mod zebrad;

use std::path::Path;

/// Check if a process with the given PID has a name matching `expected`.
/// Uses `ps` to query the process command name. Returns false if the
/// process doesn't exist or doesn't match.
pub fn is_process_named(pid: u32, expected: &str) -> bool {
    std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|name| name.trim().ends_with(expected))
        .unwrap_or(false)
}

/// Write a PID file with restrictive permissions (0600).
pub fn write_pid_file(data_dir: &Path, filename: &str, pid: u32) -> Result<(), std::io::Error> {
    let pid_path = data_dir.join(filename);
    std::fs::write(&pid_path, pid.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&pid_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}
