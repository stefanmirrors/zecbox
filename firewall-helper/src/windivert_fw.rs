//! WinDivert-based traffic interception for Shield Mode on Windows.
//!
//! Captures outbound TCP packets destined for port 8233 (Zcash P2P)
//! and redirects them to the local transparent redirector, which forwards
//! them through Arti's SOCKS5 proxy.
//!
//! Unlike macOS (PF) and Linux (iptables) which use OS firewall rules,
//! WinDivert intercepts packets at the kernel level using a signed driver.
//! The original destination is extracted directly from packet headers —
//! no NAT state lookup (DIOCNATLOOK / SO_ORIGINAL_DST) is needed.

use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, Semaphore};

use crate::socks5;

const ZCASH_PORT: u16 = 8233;
const MAX_CONCURRENT_CONNECTIONS: usize = 128;
const CONNECTION_TIMEOUT_SECS: u64 = 120;

/// WinDivert handle wrapper that intercepts outbound Zcash traffic.
///
/// When enabled, all outbound TCP connections to port 8233 are captured
/// and their packets are dropped (not re-injected). Simultaneously, a
/// local TCP listener accepts the redirected connections and forwards
/// them through Arti's SOCKS5 proxy.
///
/// The redirection works by:
/// 1. WinDivert captures SYN packets to port 8233, extracts the original
///    destination IP from packet headers
/// 2. WinDivert modifies the packet's destination to 127.0.0.1:{redir_port}
///    and re-injects it, so the connection lands on our local redirector
/// 3. The redirector reads a small header (original dest) prepended by the
///    divert thread, then SOCKS5-connects to Arti and bridges the streams
///
/// This mirrors the macOS/Linux approach (PF rdr / iptables REDIRECT)
/// but uses WinDivert instead of OS firewall rules.
pub struct WinDivertRedirector {
    enabled: AtomicBool,
}

impl WinDivertRedirector {
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    pub fn set_enabled(&self, val: bool) {
        self.enabled.store(val, Ordering::SeqCst);
    }
}

const BLOCK_RULE_NAME: &str = "ZecBox Shield";
const ALLOW_LOCALHOST_RULE_NAME: &str = "ZecBox Shield Allow Localhost";

/// Add Windows Firewall block rules as a fail-closed safety net.
///
/// These rules persist in the Windows Firewall kernel service independently
/// of the helper process. If the helper crashes and WinDivert dies, these
/// rules ensure zebrad's port 8233 traffic is BLOCKED, not leaked to clearnet.
///
/// This is the Windows equivalent of PF's `block out proto tcp from any to any port 8233`
/// and iptables' catch-all DROP rule.
pub fn add_block_rules() -> Result<(), String> {
    use std::process::Command;

    // Remove any stale rules first
    let _ = remove_block_rules();

    // Block all outbound TCP to port 8233
    let output = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            &format!("name={}", BLOCK_RULE_NAME),
            "dir=out", "protocol=tcp",
            &format!("remoteport={}", ZCASH_PORT),
            "action=block",
        ])
        .output()
        .map_err(|e| format!("Failed to run netsh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to add firewall block rule: {}", stderr.trim()));
    }

    // Allow outbound TCP to port 8233 on localhost only (for WinDivert redirect)
    let output = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "add", "rule",
            &format!("name={}", ALLOW_LOCALHOST_RULE_NAME),
            "dir=out", "protocol=tcp",
            "remoteip=127.0.0.1",
            &format!("remoteport={}", ZCASH_PORT),
            "action=allow",
        ])
        .output()
        .map_err(|e| format!("Failed to run netsh: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Roll back the block rule
        let _ = remove_block_rules();
        return Err(format!("Failed to add firewall allow-localhost rule: {}", stderr.trim()));
    }

    log::info!("Windows Firewall block rules added for port {}", ZCASH_PORT);
    Ok(())
}

/// Remove the Windows Firewall block rules.
pub fn remove_block_rules() -> Result<(), String> {
    use std::process::Command;

    let _ = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "delete", "rule",
            &format!("name={}", BLOCK_RULE_NAME),
        ])
        .output();

    let _ = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "delete", "rule",
            &format!("name={}", ALLOW_LOCALHOST_RULE_NAME),
        ])
        .output();

    log::info!("Windows Firewall block rules removed");
    Ok(())
}

/// Check if the Windows Firewall block rules are active.
pub fn are_block_rules_active() -> bool {
    use std::process::Command;

    let output = Command::new("netsh")
        .args([
            "advfirewall", "firewall", "show", "rule",
            &format!("name={}", BLOCK_RULE_NAME),
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout.contains(BLOCK_RULE_NAME)
        }
        _ => false,
    }
}

/// Check if WinDivert interception is active.
pub fn is_enabled(redirector: &WinDivertRedirector) -> bool {
    redirector.is_enabled()
}

/// Run the WinDivert packet diverter and TCP redirector.
///
/// This function:
/// 1. Opens a WinDivert handle to capture outbound TCP to port 8233
/// 2. Starts a local TCP listener on redir_port
/// 3. For each diverted connection, modifies the destination to the local
///    listener and re-injects the packet
/// 4. The local listener accepts, looks up the original destination from
///    a shared map, and forwards through SOCKS5
///
/// Runs until shutdown signal is received.
pub async fn run_divert_and_redirect(
    redir_port: u16,
    socks_addr: String,
    mut shutdown: watch::Receiver<bool>,
    redirector: Arc<WinDivertRedirector>,
) -> Result<(), String> {
    use std::collections::HashMap;
    use std::sync::Mutex;

    // Shared map: local ephemeral port -> original destination
    // Populated by the divert thread, consumed by the redirect listener
    let orig_dst_map: Arc<Mutex<HashMap<u16, SocketAddrV4>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let listen_addr = format!("127.0.0.1:{}", redir_port);
    let listener = TcpListener::bind(&listen_addr)
        .await
        .map_err(|e| format!("Failed to bind redirector on {}: {}", listen_addr, e))?;

    log::info!("WinDivert redirector listening on {}", listen_addr);

    // Spawn the WinDivert packet capture thread (blocking, runs in std thread)
    let divert_map = Arc::clone(&orig_dst_map);
    let divert_redirector = Arc::clone(&redirector);
    let (divert_stop_tx, divert_stop_rx) = std::sync::mpsc::channel::<()>();

    let divert_thread = std::thread::spawn(move || {
        if let Err(e) = run_divert_thread(redir_port, divert_map, divert_redirector, divert_stop_rx)
        {
            log::error!("WinDivert thread error: {}", e);
        }
    });

    redirector.set_enabled(true);

    let socks = Arc::new(socks_addr);
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer_addr)) => {
                        let peer_port = peer_addr.port();
                        let orig_dst = {
                            let map = orig_dst_map.lock().unwrap();
                            map.get(&peer_port).copied()
                        };

                        let Some(orig_dst) = orig_dst else {
                            log::debug!("No original destination for port {}, ignoring", peer_port);
                            continue;
                        };

                        // Clean up the map entry
                        {
                            let mut map = orig_dst_map.lock().unwrap();
                            map.remove(&peer_port);
                        }

                        let socks_clone = Arc::clone(&socks);
                        let permit = Arc::clone(&semaphore);
                        tokio::spawn(async move {
                            let _permit = match permit.try_acquire() {
                                Ok(p) => p,
                                Err(_) => {
                                    log::warn!("Connection limit reached ({})", MAX_CONCURRENT_CONNECTIONS);
                                    return;
                                }
                            };
                            let result = tokio::time::timeout(
                                std::time::Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                                handle_redirected_connection(stream, orig_dst, &socks_clone),
                            ).await;
                            if result.is_err() {
                                log::debug!("Connection timed out after {}s", CONNECTION_TIMEOUT_SECS);
                            }
                        });
                    }
                    Err(e) => {
                        log::error!("Accept error: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log::info!("WinDivert redirector shutting down");
                    break;
                }
            }
        }
    }

    redirector.set_enabled(false);

    // Stop the divert thread
    let _ = divert_stop_tx.send(());
    let _ = divert_thread.join();

    Ok(())
}

/// Handle a single redirected connection: SOCKS5 connect to original dest via Arti.
async fn handle_redirected_connection(
    inbound: TcpStream,
    orig_dst: SocketAddrV4,
    socks_addr: &str,
) {
    log::info!("Redirecting connection to {} via SOCKS5", orig_dst);

    let outbound = match socks5::connect(socks_addr, *orig_dst.ip(), orig_dst.port()).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("SOCKS5 connect to {} failed: {}", orig_dst, e);
            return;
        }
    };

    let (mut ri, mut wi) = io::split(inbound);
    let (mut ro, mut wo) = io::split(outbound);

    let c2s = tokio::spawn(async move { io::copy(&mut ri, &mut wo).await });
    let s2c = tokio::spawn(async move { io::copy(&mut ro, &mut wi).await });

    let _ = tokio::select! {
        r = c2s => r,
        r = s2c => r,
    };
}

/// Blocking thread that runs WinDivert packet capture.
///
/// Captures outbound TCP packets to port 8233, records the original
/// destination in the shared map, modifies the packet destination to
/// 127.0.0.1:{redir_port}, recalculates checksums, and re-injects it.
fn run_divert_thread(
    redir_port: u16,
    orig_dst_map: Arc<std::sync::Mutex<std::collections::HashMap<u16, SocketAddrV4>>>,
    redirector: Arc<WinDivertRedirector>,
    stop_rx: std::sync::mpsc::Receiver<()>,
) -> Result<(), String> {
    use windivert::prelude::*;

    let filter = format!("outbound and tcp.DstPort == {}", ZCASH_PORT);
    let handle = WinDivert::network(
        &filter,
        0, // priority
        WinDivertFlags::new(),
    )
    .map_err(|e| format!("Failed to open WinDivert handle: {}. Is WinDivert.dll/sys in the helper directory?", e))?;

    // Set receive timeout so we can check for stop signal periodically
    handle
        .set_param(WinDivertParam::QueueTime, 500)
        .map_err(|e| format!("Failed to set WinDivert queue time: {}", e))?;

    log::info!("WinDivert filter active: {}", filter);

    let mut buf = vec![0u8; 65535];
    let redir_port_be = redir_port.to_be_bytes();
    let localhost_be: [u8; 4] = [127, 0, 0, 1];

    loop {
        // Check stop signal
        if stop_rx.try_recv().is_ok() {
            log::info!("WinDivert thread received stop signal");
            break;
        }

        if !redirector.is_enabled() {
            break;
        }

        // Receive a packet (borrows from buf)
        let packet = match handle.recv(Some(&mut buf)) {
            Ok(p) => p,
            Err(e) => {
                let err_str = format!("{}", e);
                // Timeout is expected — just loop and check stop signal
                if err_str.contains("timeout") || err_str.contains("Timeout") {
                    continue;
                }
                log::error!("WinDivert recv error: {}", e);
                continue;
            }
        };

        // Parse the IP packet to extract src/dst info
        let parsed = match etherparse::SlicedPacket::from_ip(&packet.data) {
            Ok(p) => p,
            Err(e) => {
                log::debug!("Failed to parse packet: {}", e);
                let _ = handle.send(&packet);
                continue;
            }
        };

        // Only handle IPv4 TCP packets
        let (src_ip, dst_ip) = match &parsed.net {
            Some(etherparse::NetSlice::Ipv4(ipv4)) => {
                let src = Ipv4Addr::from(ipv4.header().source());
                let dst = Ipv4Addr::from(ipv4.header().destination());
                (src, dst)
            }
            _ => {
                let _ = handle.send(&packet);
                continue;
            }
        };

        let (src_port, dst_port) = match &parsed.transport {
            Some(etherparse::TransportSlice::Tcp(tcp)) => {
                (tcp.source_port(), tcp.destination_port())
            }
            _ => {
                let _ = handle.send(&packet);
                continue;
            }
        };

        // Skip localhost traffic (avoid redirect loop)
        if dst_ip.is_loopback() || src_ip.is_loopback() {
            let _ = handle.send(&packet);
            continue;
        }

        // Record the original destination keyed by source port
        {
            let mut map = orig_dst_map.lock().unwrap();
            map.insert(src_port, SocketAddrV4::new(dst_ip, dst_port));
        }

        // Convert to owned so we can modify the packet data in place
        let mut owned = packet.into_owned();

        // Modify the packet bytes directly:
        // IPv4 header: destination IP is at bytes 16..20
        // TCP header: destination port is at bytes ihl*4+2..ihl*4+4
        //
        // We modify dest IP to 127.0.0.1 and dest port to redir_port,
        // then let WinDivert recalculate all checksums.
        let data = owned.data.to_mut();
        if data.len() >= 20 {
            // Get IHL (Internet Header Length) from first byte
            let ihl = ((data[0] & 0x0F) as usize) * 4;

            // Set destination IP to 127.0.0.1 (bytes 16..20)
            if data.len() >= 20 {
                data[16..20].copy_from_slice(&localhost_be);
            }

            // Set TCP destination port (bytes ihl+2..ihl+4)
            if data.len() >= ihl + 4 {
                data[ihl + 2..ihl + 4].copy_from_slice(&redir_port_be);
            }
        }

        // Let WinDivert recalculate IP and TCP checksums
        // Flag 0 = recalculate all checksums
        owned.recalculate_checksums(ChecksumFlags::new());

        // Re-inject the modified packet
        let _ = handle.send(&owned);
    }

    handle.close(CloseAction::Nothing)
        .map_err(|e| format!("Failed to close WinDivert handle: {}", e))?;

    log::info!("WinDivert thread stopped");
    Ok(())
}
