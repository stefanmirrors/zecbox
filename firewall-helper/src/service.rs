//! Windows Service entry point and named pipe listener for the firewall helper.
//!
//! On Windows, the firewall helper runs as a Windows Service (equivalent to
//! LaunchDaemon on macOS / systemd on Linux). It listens on a named pipe
//! for JSON commands from the ZecBox app.
//!
//! Named pipe: \\.\pipe\com.zecbox.firewall
//! Protocol: same JSON-over-newline as the Unix socket version
//!
//! Security: The named pipe is created with a DACL that restricts access
//! to BUILTIN\Administrators and NT AUTHORITY\SYSTEM only.

use std::sync::Arc;

use tokio::sync::{watch, Mutex};
use tokio::task::JoinHandle;

use crate::windivert_fw::{self, WinDivertRedirector};

use windows_service::{define_windows_service, service_dispatcher};

const PIPE_NAME: &str = r"\\.\pipe\com.zecbox.firewall";
const HELPER_VERSION: &str = "2";
const REDIR_PORT: u16 = 9040;
const SOCKS_ADDR: &str = "127.0.0.1:9150";

/// Service name registered with Windows Service Control Manager.
pub const SERVICE_NAME: &str = "ZecBoxFirewall";

#[derive(serde::Deserialize)]
struct Command {
    cmd: String,
}

#[derive(serde::Serialize)]
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

struct DaemonState {
    enabled: bool,
    shutdown_tx: Option<watch::Sender<bool>>,
    redirector_handle: Option<JoinHandle<()>>,
    redirector: Arc<WinDivertRedirector>,
}

impl DaemonState {
    fn new() -> Self {
        DaemonState {
            enabled: false,
            shutdown_tx: None,
            redirector_handle: None,
            redirector: Arc::new(WinDivertRedirector::new()),
        }
    }
}

async fn handle_enable(state: &mut DaemonState) -> Response {
    if state.enabled {
        return Response::success();
    }

    // SAFETY FIRST: Add Windows Firewall block rules BEFORE starting WinDivert.
    // These rules persist in the kernel even if our process crashes, ensuring
    // zebrad can never reach clearnet on port 8233 without going through Tor.
    // This is the Windows equivalent of PF's "block out proto tcp ... port 8233".
    if let Err(e) = windivert_fw::add_block_rules() {
        return Response::error(format!("Failed to add firewall block rules: {}", e));
    }

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let socks_addr = SOCKS_ADDR.to_string();
    let redirector = Arc::clone(&state.redirector);

    let handle = tokio::spawn(async move {
        if let Err(e) =
            windivert_fw::run_divert_and_redirect(REDIR_PORT, socks_addr, shutdown_rx, redirector)
                .await
        {
            log::error!("WinDivert redirector error: {}", e);
        }
    });

    // Give the redirector a moment to bind
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    state.enabled = true;
    state.shutdown_tx = Some(shutdown_tx);
    state.redirector_handle = Some(handle);

    log::info!("Shield firewall enabled (WinDivert + Windows Firewall block rules)");
    Response::success()
}

async fn handle_disable(state: &mut DaemonState) -> Response {
    if !state.enabled {
        return Response::success();
    }

    // Stop WinDivert first
    state.redirector.set_enabled(false);

    if let Some(tx) = state.shutdown_tx.take() {
        let _ = tx.send(true);
    }
    if let Some(handle) = state.redirector_handle.take() {
        handle.abort();
    }

    // Remove Windows Firewall block rules AFTER stopping WinDivert
    if let Err(e) = windivert_fw::remove_block_rules() {
        log::error!("Failed to remove firewall block rules: {}", e);
    }

    state.enabled = false;
    log::info!("Shield firewall disabled (WinDivert + block rules removed)");
    Response::success()
}

fn handle_status(state: &DaemonState) -> Response {
    let redirector_running = state
        .redirector_handle
        .as_ref()
        .map(|h| !h.is_finished())
        .unwrap_or(false);

    // Report enabled based on both WinDivert AND block rules being active.
    // The block rules are the ground truth — they persist even if WinDivert dies.
    let block_rules_active = windivert_fw::are_block_rules_active();
    Response::status(state.enabled && block_rules_active, redirector_running)
}

/// Create a named pipe with a security descriptor that restricts access
/// to BUILTIN\Administrators and NT AUTHORITY\SYSTEM only.
///
/// This prevents unprivileged local processes from connecting to the pipe
/// and sending commands like "disable" to drop the shield.
///
/// SDDL string: D:(A;;GA;;;BA)(A;;GA;;;SY)
///   D:          = DACL
///   (A;;GA;;;BA) = Allow Generic All to BUILTIN\Administrators
///   (A;;GA;;;SY) = Allow Generic All to NT AUTHORITY\SYSTEM
fn create_secure_pipe() -> Result<tokio::net::windows::named_pipe::NamedPipeServer, std::io::Error> {
    use std::os::windows::io::RawHandle;
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
    use windows_sys::Win32::System::Pipes::{
        CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_READMODE_BYTE, PIPE_WAIT,
        PIPE_UNLIMITED_INSTANCES,
    };

    // SDDL: allow Administrators and SYSTEM only
    let sddl: Vec<u16> = "D:(A;;GRGW;;;BU)(A;;GA;;;BA)(A;;GA;;;SY)\0"
        .encode_utf16()
        .collect();

    let mut sd: *mut std::ffi::c_void = std::ptr::null_mut();
    let ok = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1, // SDDL_REVISION_1
            &mut sd,
            std::ptr::null_mut(),
        )
    };

    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }

    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };

    let pipe_name: Vec<u16> = PIPE_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // FILE_FLAG_OVERLAPPED = 0x40000000 (required for async tokio I/O)
    const FILE_FLAG_OVERLAPPED: u32 = 0x40000000;

    let handle = unsafe {
        CreateNamedPipeW(
            pipe_name.as_ptr(),
            PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            4096,  // out buffer
            4096,  // in buffer
            0,     // default timeout
            &sa as *const SECURITY_ATTRIBUTES,
        )
    };

    // Free the security descriptor
    unsafe { windows_sys::Win32::Foundation::LocalFree(sd) };

    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error());
    }

    // Wrap in tokio's NamedPipeServer (inherent unsafe method, returns Result)
    unsafe {
        tokio::net::windows::named_pipe::NamedPipeServer::from_raw_handle(
            handle as RawHandle,
        )
    }
}

/// Run the named pipe server. This is the main loop for the Windows service.
///
/// Listens for connections on the named pipe, processes JSON commands,
/// and manages the WinDivert redirector lifecycle.
pub async fn run_pipe_server(mut service_stop: watch::Receiver<bool>) {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    let state = Arc::new(Mutex::new(DaemonState::new()));

    log::info!("Named pipe server starting on {} (restricted to Administrators)", PIPE_NAME);

    loop {
        // Create a new pipe instance with restricted security descriptor
        let server = match create_secure_pipe() {
            Ok(s) => s,
            Err(e) => {
                log::error!("Failed to create named pipe {}: {}", PIPE_NAME, e);
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                continue;
            }
        };

        // Wait for a client to connect or service stop signal
        tokio::select! {
            result = server.connect() => {
                if let Err(e) = result {
                    log::error!("Named pipe connect error: {}", e);
                    continue;
                }

                let state = Arc::clone(&state);
                tokio::spawn(async move {
                    let (reader, mut writer) = tokio::io::split(server);
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
            _ = service_stop.changed() => {
                if *service_stop.borrow() {
                    log::info!("Service stop signal received, cleaning up");
                    let mut s = state.lock().await;
                    let _ = handle_disable(&mut s).await;
                    break;
                }
            }
        }
    }

    log::info!("Named pipe server stopped");
}

// Generate the FFI entry point required by windows-service.
// This macro creates an extern "system" fn that the Windows SCM can call.
define_windows_service!(ffi_service_main, run_service);

/// The actual service logic, called by the FFI entry point.
fn run_service(_arguments: Vec<std::ffi::OsString>) {
    use windows_service::service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    };
    use windows_service::service_control_handler::{self, ServiceControlHandlerResult};

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("ZecBox Firewall Helper service starting");

    let (stop_tx, stop_rx) = watch::channel(false);

    // Register the service control handler
    let stop_tx_clone = stop_tx.clone();
    let event_handler = move |control_event| -> ServiceControlHandlerResult {
        match control_event {
            ServiceControl::Stop | ServiceControl::Shutdown => {
                log::info!("Service received stop/shutdown control");
                let _ = stop_tx_clone.send(true);
                ServiceControlHandlerResult::NoError
            }
            ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
            _ => ServiceControlHandlerResult::NotImplemented,
        }
    };

    let status_handle = match service_control_handler::register(SERVICE_NAME, event_handler) {
        Ok(h) => h,
        Err(e) => {
            log::error!("Failed to register service control handler: {}", e);
            return;
        }
    };

    // Report running status
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    });

    // Run the async pipe server
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_pipe_server(stop_rx));

    // Report stopped
    let _ = status_handle.set_service_status(ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    });

    log::info!("ZecBox Firewall Helper service stopped");
}

/// Windows Service entry point.
///
/// Registers with the Windows Service Control Manager via the FFI entry point.
/// If not started by SCM (e.g. running from command line), falls back to
/// console mode for testing.
pub fn service_main() {
    if let Err(e) = service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        let err_str = format!("{}", e);
        if err_str.contains("1063") {
            // ERROR_FAILED_SERVICE_CONTROLLER_CONNECT: not started by SCM
            eprintln!("Not running as a service. Starting in console mode for testing.");
            env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
                .init();
            let rt = tokio::runtime::Runtime::new().unwrap();
            let (stop_tx, stop_rx) = watch::channel(false);

            // Handle Ctrl+C for console mode
            let stop_tx_clone = stop_tx.clone();
            rt.spawn(async move {
                let _ = tokio::signal::ctrl_c().await;
                log::info!("Ctrl+C received, shutting down");
                let _ = stop_tx_clone.send(true);
            });

            rt.block_on(run_pipe_server(stop_rx));
        } else {
            eprintln!("Failed to start service dispatcher: {}", e);
        }
    }
}
