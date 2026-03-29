//! Sidecar process management: spawn, monitor, restart, PID tracking.

pub mod platform;
pub mod wireguard;
pub mod zaino;
pub mod zebrad;

use std::path::Path;

/// Check if a process with the given PID has a name matching `expected`.
/// Returns false if the process doesn't exist or doesn't match.
#[cfg(unix)]
pub fn is_process_named(pid: u32, expected: &str) -> bool {
    std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "comm="])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|name| name.trim().ends_with(expected))
        .unwrap_or(false)
}

/// Check if a process with the given PID has a name matching `expected`.
/// Uses CreateToolhelp32Snapshot to enumerate processes on Windows.
#[cfg(windows)]
pub fn is_process_named(pid: u32, expected: &str) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return false;
        }

        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        if Process32FirstW(snapshot, &mut entry) == 0 {
            CloseHandle(snapshot);
            return false;
        }

        loop {
            if entry.th32ProcessID == pid {
                let name_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = String::from_utf16_lossy(&entry.szExeFile[..name_len]);
                CloseHandle(snapshot);
                let expected_exe = format!("{}.exe", expected);
                return name.ends_with(expected) || name.ends_with(&expected_exe);
            }
            if Process32NextW(snapshot, &mut entry) == 0 {
                break;
            }
        }

        CloseHandle(snapshot);
    }
    false
}

/// Write a PID file with restrictive permissions (0600 on Unix).
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
