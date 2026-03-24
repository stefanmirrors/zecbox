//! ZecBox Firewall Helper Daemon
//!
//! Privileged helper that runs as root via LaunchDaemon.
//! Manages PF firewall rules and a transparent SOCKS5 redirector
//! to force zebrad P2P traffic through Arti/Tor.
//!
//! Communication: Unix socket at /var/run/com.zecbox.firewall.sock
//! Commands (JSON):
//!   {"cmd":"enable"}  — Load PF anchor + start redirector
//!   {"cmd":"disable"} — Flush PF anchor + stop redirector
//!   {"cmd":"status"}  — Return current state

mod pf;
mod redirector;
mod socks5;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;
use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

const SOCKET_PATH: &str = "/var/run/com.zecbox.firewall.sock";
const REDIR_LISTEN: &str = "127.0.0.1:9040";
const REDIR_PORT: u16 = 9040;
const SOCKS_ADDR: &str = "127.0.0.1:9150";

#[derive(Debug, Deserialize)]
struct Command {
    cmd: String,
}

#[derive(Debug, Serialize)]
struct Response {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    redirector_running: Option<bool>,
}

impl Response {
    fn success() -> Self {
        Response {
            ok: true,
            error: None,
            enabled: None,
            redirector_running: None,
        }
    }

    fn error(msg: String) -> Self {
        Response {
            ok: false,
            error: Some(msg),
            enabled: None,
            redirector_running: None,
        }
    }

    fn status(enabled: bool, redirector_running: bool) -> Self {
        Response {
            ok: true,
            error: None,
            enabled: Some(enabled),
            redirector_running: Some(redirector_running),
        }
    }
}

struct DaemonState {
    enabled: bool,
    shutdown_tx: Option<watch::Sender<bool>>,
    redirector_handle: Option<JoinHandle<()>>,
}

impl DaemonState {
    fn new() -> Self {
        DaemonState {
            enabled: false,
            shutdown_tx: None,
            redirector_handle: None,
        }
    }
}

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

    // Load PF rules
    if let Err(e) = pf::enable(REDIR_PORT) {
        // Stop redirector if PF fails
        let _ = shutdown_tx.send(true);
        handle.abort();
        return Response::error(format!("Failed to load PF rules: {}", e));
    }

    state.enabled = true;
    state.shutdown_tx = Some(shutdown_tx);
    state.redirector_handle = Some(handle);

    log::info!("Shield firewall enabled");
    Response::success()
}

async fn handle_disable(state: &mut DaemonState) -> Response {
    if !state.enabled {
        return Response::success();
    }

    // Flush PF rules first
    if let Err(e) = pf::disable() {
        log::error!("Failed to flush PF rules: {}", e);
        // Continue with shutdown even if PF flush fails
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

fn handle_status(state: &DaemonState) -> Response {
    let redirector_running = state
        .redirector_handle
        .as_ref()
        .map(|h| !h.is_finished())
        .unwrap_or(false);

    Response::status(state.enabled, redirector_running)
}

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

    // Restrict socket to root and staff group (standard macOS interactive users)
    if let Err(e) = fs::set_permissions(SOCKET_PATH, fs::Permissions::from_mode(0o660)) {
        log::warn!("Failed to set socket permissions: {}", e);
    }
    // Set socket group to 'staff' so the unprivileged ZecBox app can connect
    let staff_gid = get_staff_gid().unwrap_or(20); // 20 is default staff GID on macOS
    unsafe {
        let c_path = std::ffi::CString::new(SOCKET_PATH).unwrap();
        libc::chown(c_path.as_ptr(), 0, staff_gid);
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
                // Verify peer credentials: only allow root or staff group members
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

/// Resolve the GID of the 'staff' group on macOS.
fn get_staff_gid() -> Option<u32> {
    unsafe {
        let name = std::ffi::CString::new("staff").ok()?;
        let grp = libc::getgrnam(name.as_ptr());
        if grp.is_null() {
            None
        } else {
            Some((*grp).gr_gid)
        }
    }
}

/// Verify the connecting peer is root or a member of the staff group.
fn verify_peer_credentials(stream: &tokio::net::UnixStream) -> Result<(), String> {
    let raw_fd = stream.as_raw_fd();
    let mut uid: libc::uid_t = 0;
    let mut gid: libc::gid_t = 0;

    let ret = unsafe { libc::getpeereid(raw_fd, &mut uid, &mut gid) };
    if ret != 0 {
        return Err(format!(
            "getpeereid failed: {}",
            std::io::Error::last_os_error()
        ));
    }

    // Allow root
    if uid == 0 {
        return Ok(());
    }

    // Allow members of the staff group
    let staff_gid = get_staff_gid().unwrap_or(20);
    if gid == staff_gid {
        return Ok(());
    }

    // Also check supplementary groups — the user's primary GID might not be staff
    // but they may still be a member
    if is_uid_in_group(uid, staff_gid) {
        return Ok(());
    }

    Err(format!(
        "Unauthorized: uid={} gid={} is not root or staff",
        uid, gid
    ))
}

/// Check if a UID belongs to a given group (including supplementary groups).
fn is_uid_in_group(uid: libc::uid_t, target_gid: libc::gid_t) -> bool {
    unsafe {
        let pw = libc::getpwuid(uid);
        if pw.is_null() {
            return false;
        }

        let mut ngroups: libc::c_int = 32;
        let mut groups = vec![0i32; ngroups as usize];
        let ret = libc::getgrouplist(
            (*pw).pw_name,
            (*pw).pw_gid as i32,
            groups.as_mut_ptr(),
            &mut ngroups,
        );

        if ret < 0 {
            // Buffer too small, resize and retry
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
}
