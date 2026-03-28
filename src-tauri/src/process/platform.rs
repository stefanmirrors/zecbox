//! Cross-platform process management helpers.
//! Unix: SIGTERM/SIGKILL via nix crate.
//! Windows: TerminateProcess via windows-sys.

use std::time::Duration;

/// Check if a process with the given PID is still alive.
#[cfg(unix)]
pub fn is_process_alive(pid: u32) -> bool {
    nix::sys::signal::kill(nix::unistd::Pid::from_raw(pid as i32), None).is_ok()
}

#[cfg(windows)]
pub fn is_process_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut exit_code: u32 = 0;
        let alive = GetExitCodeProcess(handle, &mut exit_code) != 0
            && exit_code == 259; // STILL_ACTIVE
        CloseHandle(handle);
        alive
    }
}

/// Send a graceful termination signal (SIGTERM on Unix, no-op on Windows).
#[cfg(unix)]
pub fn send_term(pid: u32) {
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGTERM,
    );
}

#[cfg(windows)]
pub fn send_term(_pid: u32) {
    // No portable SIGTERM equivalent for sidecar processes on Windows.
    // The graceful_stop function handles the timeout+kill pattern.
}

/// Force-kill a process by PID.
#[cfg(unix)]
pub fn force_kill(pid: u32) {
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(pid as i32),
        nix::sys::signal::Signal::SIGKILL,
    );
}

#[cfg(windows)]
pub fn force_kill(pid: u32) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};

    unsafe {
        let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
        if !handle.is_null() {
            TerminateProcess(handle, 1);
            CloseHandle(handle);
        }
    }
}

/// Gracefully stop a child process: SIGTERM then wait, SIGKILL on timeout (Unix),
/// or wait then TerminateProcess (Windows).
pub async fn graceful_stop(child: &mut tokio::process::Child, timeout: Duration) {
    if let Some(pid) = child.id() {
        send_term(pid);

        let wait_result = tokio::time::timeout(timeout, child.wait()).await;

        if wait_result.is_err() {
            log::warn!("Process {} did not exit in {:?}, force killing", pid, timeout);
            let _ = child.kill().await;
        }
    } else {
        let _ = child.kill().await;
    }
}
