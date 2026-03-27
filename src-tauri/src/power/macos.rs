//! macOS sleep/wake handling via IOKit power notifications,
//! and launchd agent management for auto-start at login.

use std::ffi::c_void;

use core_foundation::base::TCFType;
use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
use tauri::AppHandle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::wake_handler::{self, PowerEvent};

// --- IOKit FFI bindings ---

type IONotificationPortRef = *mut c_void;
type IOReturn = i32;
type IOObject = u32;

#[allow(non_upper_case_globals)]
const kIOMessageSystemHasPoweredOn: u32 = 0xe000_0300;
#[allow(non_upper_case_globals)]
const kIOMessageSystemWillSleep: u32 = 0xe000_0280;
#[allow(non_upper_case_globals)]
const kIOMessageCanSystemSleep: u32 = 0xe000_0270;

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IORegisterForSystemPower(
        refcon: *mut c_void,
        the_port_ref: *mut IONotificationPortRef,
        callback: extern "C" fn(
            refcon: *mut c_void,
            service: IOObject,
            message_type: u32,
            message_argument: *mut c_void,
        ),
        notifier: *mut IOObject,
    ) -> IOObject;

    fn IODeregisterForSystemPower(notifier: *mut IOObject) -> IOReturn;

    fn IONotificationPortGetRunLoopSource(notify_port: IONotificationPortRef)
        -> core_foundation::runloop::CFRunLoopSourceRef;

    fn IOAllowPowerChange(kernel_port: IOObject, notification_id: isize) -> IOReturn;
}

// Shared sender for the power callback to communicate with the async handler
static mut POWER_SENDER: Option<mpsc::UnboundedSender<PowerEvent>> = None;
static mut ROOT_PORT: IOObject = 0;

extern "C" fn power_callback(
    _refcon: *mut c_void,
    _service: IOObject,
    message_type: u32,
    message_argument: *mut c_void,
) {
    #[allow(non_upper_case_globals)]
    match message_type {
        kIOMessageSystemHasPoweredOn => {
            log::info!("System woke from sleep");
            unsafe {
                if let Some(ref sender) = POWER_SENDER {
                    let _ = sender.send(PowerEvent::Wake);
                }
            }
        }
        kIOMessageSystemWillSleep => {
            log::info!("System going to sleep");
            unsafe {
                IOAllowPowerChange(ROOT_PORT, message_argument as isize);
            }
        }
        kIOMessageCanSystemSleep => {
            unsafe {
                IOAllowPowerChange(ROOT_PORT, message_argument as isize);
            }
        }
        _ => {}
    }
}

/// Spawn the power monitor: a std::thread for the CFRunLoop + a tokio task for wake handling.
pub fn spawn_power_monitor(
    app_handle: AppHandle,
) -> (std::thread::JoinHandle<()>, JoinHandle<()>) {
    let (tx, rx) = mpsc::unbounded_channel();

    unsafe {
        POWER_SENDER = Some(tx);
    }

    let thread_handle = std::thread::Builder::new()
        .name("power-monitor".into())
        .spawn(move || {
            let mut notify_port: IONotificationPortRef = std::ptr::null_mut();
            let mut notifier: IOObject = 0;

            unsafe {
                ROOT_PORT = IORegisterForSystemPower(
                    std::ptr::null_mut(),
                    &mut notify_port,
                    power_callback,
                    &mut notifier,
                );

                if ROOT_PORT == 0 {
                    log::error!("Failed to register for system power notifications");
                    return;
                }

                let source = IONotificationPortGetRunLoopSource(notify_port);
                let run_loop = CFRunLoop::get_current();
                let cf_source = core_foundation::runloop::CFRunLoopSource::wrap_under_get_rule(source);
                run_loop.add_source(&cf_source, kCFRunLoopDefaultMode);

                log::info!("Power monitor registered, running CFRunLoop");
                CFRunLoop::run_current();

                IODeregisterForSystemPower(&mut notifier);
            }
        })
        .expect("failed to spawn power monitor thread");

    let wake_task = tokio::spawn(wake_handler::handle_wake_events(app_handle, rx));

    (thread_handle, wake_task)
}

pub fn stop_power_monitor() {
    unsafe {
        POWER_SENDER = None;
    }
}

// --- launchd agent management ---

const LAUNCH_AGENT_LABEL: &str = "com.zecbox.app";

fn launch_agent_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join("Library/LaunchAgents")
        .join(format!("{}.plist", LAUNCH_AGENT_LABEL))
}

pub fn install_launch_agent() -> Result<(), String> {
    let app_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get app path: {}", e))?;

    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <false/>
</dict>
</plist>"#,
        LAUNCH_AGENT_LABEL,
        app_path.display()
    );

    let path = launch_agent_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create LaunchAgents dir: {}", e))?;
    }
    std::fs::write(&path, plist)
        .map_err(|e| format!("Failed to write launch agent plist: {}", e))?;

    log::info!("Installed launch agent at {:?}", path);
    Ok(())
}

pub fn remove_launch_agent() -> Result<(), String> {
    let path = launch_agent_path();
    if path.exists() {
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to remove launch agent: {}", e))?;
        log::info!("Removed launch agent at {:?}", path);
    }
    Ok(())
}

pub fn is_launch_agent_installed() -> bool {
    launch_agent_path().exists()
}
