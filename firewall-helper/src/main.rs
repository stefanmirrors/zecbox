//! ZecBox Firewall Helper Daemon
//!
//! Privileged helper that runs as root.
//! macOS: LaunchDaemon, manages PF firewall rules
//! Linux: systemd service, manages iptables rules
//!
//! Communication: Unix socket at /var/run/com.zecbox.firewall.sock
//! Commands (JSON):
//!   {"cmd":"enable"}  — Load firewall rules + start redirector
//!   {"cmd":"disable"} — Flush firewall rules + stop redirector
//!   {"cmd":"status"}  — Return current state

#[cfg(target_os = "macos")]
mod pf;
#[cfg(target_os = "linux")]
mod iptables;
#[cfg(not(target_os = "windows"))]
mod redirector;
mod socks5;
#[cfg(target_os = "windows")]
mod windivert_fw;
#[cfg(target_os = "windows")]
mod service;

#[cfg(not(target_os = "windows"))]
use std::fs;
#[cfg(not(target_os = "windows"))]
use std::os::unix::fs::PermissionsExt;
#[cfg(not(target_os = "windows"))]
use std::os::unix::io::AsRawFd;
#[cfg(not(target_os = "windows"))]
use std::path::Path;
#[cfg(not(target_os = "windows"))]
use std::sync::Arc;

#[cfg(not(target_os = "windows"))]
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "windows"))]
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
#[cfg(not(target_os = "windows"))]
use tokio::net::UnixListener;
#[cfg(not(target_os = "windows"))]
use tokio::sync::{watch, Mutex};
#[cfg(not(target_os = "windows"))]
use tokio::task::JoinHandle;

#[cfg(not(target_os = "windows"))]
const SOCKET_PATH: &str = "/var/run/com.zecbox.firewall.sock";
#[cfg(not(target_os = "windows"))]
/// Bump this whenever the helper protocol or behavior changes.
/// The app checks this to detect outdated helpers and prompt for reinstallation.
const HELPER_VERSION: &str = "2";
#[cfg(not(target_os = "windows"))]
const REDIR_LISTEN: &str = "127.0.0.1:9040";
#[cfg(not(target_os = "windows"))]
const REDIR_PORT: u16 = 9040;
#[cfg(not(target_os = "windows"))]
const SOCKS_ADDR: &str = "127.0.0.1:9150";

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Deserialize)]
struct Command {
    cmd: String,
}

#[cfg(not(target_os = "windows"))]
#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirector_running: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
}

#[cfg(not(target_os = "windows"))]
impl Response {
    fn success() -> Self {
        Response {
            ok: true,
            error: None,
            enabled: None,
            redirector_running: None,
            version: None,
        }
    }

    fn error(msg: String) -> Self {
        Response {
            ok: false,
            error: Some(msg),
            enabled: None,
            redirector_running: None,
            version: None,
        }
    }

    fn status(enabled: bool, redirector_running: bool) -> Self {
        Response {
            ok: true,
            error: None,
            enabled: Some(enabled),
            redirector_running: Some(redirector_running),
            version: Some(HELPER_VERSION.to_string()),
        }
    }
}

#[cfg(not(target_os = "windows"))]
struct DaemonState {
    enabled: bool,
    shutdown_tx: Option<watch::Sender<bool>>,
    redirector_handle: Option<JoinHandle<()>>,
}

#[cfg(not(target_os = "windows"))]
impl DaemonState {
    fn new() -> Self {
        DaemonState {
            enabled: false,
            shutdown_tx: None,
            redirector_handle: None,
        }
    }
}

#[cfg(not(target_os = "windows"))]
async fn handle_enable(state: &mut DaemonState) -> Response {
    if state.enabled {
        return Response::success();
    }

    // Start the transparent redirector
    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let socks_addr = SOCKS_ADDR.to_string();
    let listen_addr = REDIR_LISTEN.to_string();

    let handle = tokio::spawn(async move {
        if let Err(e) = redirector::run(&listen_addr, socks_addr, shutdown_rx).await {
            log::error!("Redirector error: {}", e);
        }
    });

    // Give the redirector a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    // Load firewall rules
    #[cfg(target_os = "macos")]
    let fw_result = pf::enable(REDIR_PORT);
    #[cfg(target_os = "linux")]
    let fw_result = iptables::enable(REDIR_PORT);

    if let Err(e) = fw_result {
        let _ = shutdown_tx.send(true);
        handle.abort();
        return Response::error(format!("Failed to load firewall rules: {}", e));
    }

    state.enabled = true;
    state.shutdown_tx = Some(shutdown_tx);
    state.redirector_handle = Some(handle);

    log::info!("Shield firewall enabled");
    Response::success()
}

#[cfg(not(target_os = "windows"))]
async fn handle_disable(state: &mut DaemonState) -> Response {
    if !state.enabled {
        return Response::success();
    }

    // Flush firewall rules first
    #[cfg(target_os = "macos")]
    if let Err(e) = pf::disable() {
        log::error!("Failed to flush firewall rules: {}", e);
    }
    #[cfg(target_os = "linux")]
    if let Err(e) = iptables::disable() {
        log::error!("Failed to flush firewall rules: {}", e);
    }

    // Stop redirector
    if let Some(tx) = state.shutdown_tx.take() {
        let _ = tx.send(true);
    }
    if let Some(handle) = state.redirector_handle.take() {
        handle.abort();
    }

    state.enabled = false;
    log::info!("Shield firewall disabled");
    Response::success()
}

#[cfg(not(target_os = "windows"))]
fn handle_status(state: &DaemonState) -> Response {
    let redirector_running = state
        .redirector_handle
        .as_ref()
        .map(|h| !h.is_finished())
        .unwrap_or(false);

    Response::status(state.enabled, redirector_running)
}

#[cfg(target_os = "windows")]
fn main() {
    service::service_main();
}

#[cfg(not(target_os = "windows"))]
#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .init();

    log::info!("ZecBox Firewall Helper starting");

    // Clean up stale socket
    if Path::new(SOCKET_PATH).exists() {
        let _ = fs::remove_file(SOCKET_PATH);
    }

    let listener = match UnixListener::bind(SOCKET_PATH) {
        Ok(l) => l,
        Err(e) => {
            log::error!("Failed to bind Unix socket at {}: {}", SOCKET_PATH, e);
            std::process::exit(1);
        }
    };

    // Restrict socket permissions
    if let Err(e) = fs::set_permissions(SOCKET_PATH, fs::Permissions::from_mode(0o660)) {
        log::warn!("Failed to set socket permissions: {}", e);
    }

    // Set socket group so the unprivileged ZecBox app can connect
    let app_gid = get_app_group_gid();
    unsafe {
        let c_path = std::ffi::CString::new(SOCKET_PATH).unwrap();
        libc::chown(c_path.as_ptr(), 0, app_gid);
    }

    log::info!("Listening on {}", SOCKET_PATH);

    let state = Arc::new(Mutex::new(DaemonState::new()));

    // Handle SIGTERM for graceful shutdown
    let state_clone = Arc::clone(&state);
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        log::info!("Received shutdown signal, cleaning up");
        let mut s = state_clone.lock().await;
        let _ = handle_disable(&mut s).await;
        let _ = fs::remove_file(SOCKET_PATH);
        std::process::exit(0);
    });

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                if let Err(e) = verify_peer_credentials(&stream) {
                    log::warn!("Rejected connection: {}", e);
                    continue;
                }

                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    let (reader, mut writer) = stream.into_split();
                    let mut lines = BufReader::new(reader).lines();

                    while let Ok(Some(line)) = lines.next_line().await {
                        let response = match serde_json::from_str::<Command>(&line) {
                            Ok(cmd) => {
                                let mut s = state.lock().await;
                                match cmd.cmd.as_str() {
                                    "enable" => handle_enable(&mut s).await,
                                    "disable" => handle_disable(&mut s).await,
                                    "status" => handle_status(&s),
                                    other => Response::error(format!("Unknown command: {}", other)),
                                }
                            }
                            Err(e) => Response::error(format!("Invalid JSON: {}", e)),
                        };

                        let mut resp_json = serde_json::to_string(&response).unwrap();
                        resp_json.push('\n');
                        if writer.write_all(resp_json.as_bytes()).await.is_err() {
                            break;
                        }
                    }
                });
            }
            Err(e) => {
                log::error!("Accept error: {}", e);
            }
        }
    }
}

// --- Platform-specific group and credential functions (Unix only) ---

#[cfg(not(target_os = "windows"))]
/// Get the GID of the group that app users belong to.
fn get_app_group_gid() -> u32 {
    #[cfg(target_os = "macos")]
    {
        // macOS: "staff" group (GID 20) includes all interactive users
        resolve_group_gid("staff").unwrap_or(20)
    }
    #[cfg(target_os = "linux")]
    {
        // Linux: try "sudo" group (Debian/Ubuntu), then "wheel" (Fedora/Arch)
        resolve_group_gid("sudo")
            .or_else(|| resolve_group_gid("wheel"))
            .unwrap_or(0) // fallback to root
    }
}

#[cfg(not(target_os = "windows"))]
fn resolve_group_gid(name: &str) -> Option<u32> {
    unsafe {
        let c_name = std::ffi::CString::new(name).ok()?;
        let grp = libc::getgrnam(c_name.as_ptr());
        if grp.is_null() {
            None
        } else {
            Some((*grp).gr_gid)
        }
    }
}

#[cfg(not(target_os = "windows"))]
/// Verify the connecting peer is root or a member of the app group.
fn verify_peer_credentials(stream: &tokio::net::UnixStream) -> Result<(), String> {
    let raw_fd = stream.as_raw_fd();

    #[cfg(target_os = "macos")]
    let (uid, gid) = {
        let mut uid: libc::uid_t = 0;
        let mut gid: libc::gid_t = 0;
        let ret = unsafe { libc::getpeereid(raw_fd, &mut uid, &mut gid) };
        if ret != 0 {
            return Err(format!("getpeereid failed: {}", std::io::Error::last_os_error()));
        }
        (uid, gid)
    };

    #[cfg(target_os = "linux")]
    let (uid, gid) = {
        let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
        let ret = unsafe {
            libc::getsockopt(
                raw_fd,
                libc::SOL_SOCKET,
                libc::SO_PEERCRED,
                &mut cred as *mut libc::ucred as *mut libc::c_void,
                &mut len,
            )
        };
        if ret != 0 {
            return Err(format!("getsockopt SO_PEERCRED failed: {}", std::io::Error::last_os_error()));
        }
        (cred.uid, cred.gid)
    };

    // Allow root
    if uid == 0 {
        return Ok(());
    }

    // Allow members of the app group
    let app_gid = get_app_group_gid();
    if gid == app_gid {
        return Ok(());
    }

    // Check supplementary groups
    if is_uid_in_group(uid, app_gid) {
        return Ok(());
    }

    Err(format!(
        "Unauthorized: uid={} gid={} is not root or app group member",
        uid, gid
    ))
}

#[cfg(not(target_os = "windows"))]
fn is_uid_in_group(uid: libc::uid_t, target_gid: libc::gid_t) -> bool {
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return false;
        }

        let mut ngroups: libc::c_int = 32;

        // On macOS, getgrouplist uses i32 for groups; on Linux, it uses gid_t (u32)
        #[cfg(target_os = "macos")]
        {
            let mut groups = vec![0i32; ngroups as usize];
            let ret = libc::getgrouplist(
                (*pw).pw_name,
                (*pw).pw_gid as i32,
                groups.as_mut_ptr(),
                &mut ngroups,
            );
            if ret < 0 {
                groups.resize(ngroups as usize, 0);
                libc::getgrouplist(
                    (*pw).pw_name,
                    (*pw).pw_gid as i32,
                    groups.as_mut_ptr(),
                    &mut ngroups,
                );
            }
            groups[..ngroups as usize].contains(&(target_gid as i32))
        }

        #[cfg(target_os = "linux")]
        {
            let mut groups = vec![0u32; ngroups as usize];
            let ret = libc::getgrouplist(
                (*pw).pw_name,
                (*pw).pw_gid,
                groups.as_mut_ptr(),
                &mut ngroups,
            );
            if ret < 0 {
                groups.resize(ngroups as usize, 0);
                libc::getgrouplist(
                    (*pw).pw_name,
                    (*pw).pw_gid,
                    groups.as_mut_ptr(),
                    &mut ngroups,
                );
            }
            groups[..ngroups as usize].contains(&target_gid)
        }
    }
}
