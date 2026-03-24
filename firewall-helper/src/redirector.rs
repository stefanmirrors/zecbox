//! Transparent SOCKS5 redirector.
//! Accepts PF-redirected connections, looks up the original destination
//! via DIOCNATLOOK on /dev/pf, and forwards through Arti SOCKS5.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::os::fd::RawFd;
use std::sync::Arc;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;

use crate::socks5;

// --- macOS PF ioctl definitions ---
// Based on XNU open-source pfvar.h

/// pf_addr: union of IPv4/IPv6 address, stored as 4 x u32 (16 bytes)
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct PfAddr {
    addr32: [u32; 4],
}

impl PfAddr {
    fn from_ipv4(ip: Ipv4Addr) -> Self {
        let octets = ip.octets();
        let v = u32::from_be_bytes(octets);
        let mut addr = PfAddr::default();
        addr.addr32[0] = v;
        addr
    }

    fn to_ipv4(&self) -> Ipv4Addr {
        let bytes = self.addr32[0].to_be_bytes();
        Ipv4Addr::new(bytes[0], bytes[1], bytes[2], bytes[3])
    }
}

/// pf_state_xport: port union (4 bytes)
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct PfStateXport {
    port: u16,
    _pad: u16,
}

impl PfStateXport {
    fn from_port(port: u16) -> Self {
        PfStateXport {
            port: port.to_be(),
            _pad: 0,
        }
    }

    fn to_port(&self) -> u16 {
        u16::from_be(self.port)
    }
}

/// pfioc_natlook: structure for DIOCNATLOOK ioctl
#[repr(C)]
#[derive(Copy, Clone, Default)]
struct PfiocNatlook {
    saddr: PfAddr,        // source address
    daddr: PfAddr,        // destination address (our proxy addr after rdr)
    rsaddr: PfAddr,       // real source (result)
    rdaddr: PfAddr,       // real destination (result — the original dst before rdr)
    sxport: PfStateXport, // source port
    dxport: PfStateXport, // destination port (our proxy port after rdr)
    rsxport: PfStateXport, // real source port (result)
    rdxport: PfStateXport, // real destination port (result)
    af: u8,               // AF_INET = 2
    proto: u8,            // IPPROTO_TCP = 6
    direction: u8,        // PF_IN = 0
    _pad: [u8; 1],
}

const AF_INET: u8 = 2;
const IPPROTO_TCP: u8 = 6;
const PF_IN: u8 = 0;

// DIOCNATLOOK = _IOWR('D', 23, struct pfioc_natlook)
// On macOS: IOC_INOUT(0xC0000000) | (sizeof << 16) | ('D' << 8) | 23
// sizeof(PfiocNatlook) = 4*16 + 4*4 + 4 = 84 bytes
// ioctl = 0xC0000000 | (84 << 16) | (0x44 << 8) | 0x17 = 0xC0544417
const DIOCNATLOOK: libc::c_ulong = 0xC054_4417;

/// Look up the original destination of a PF-redirected connection.
fn natlook(pf_fd: RawFd, peer: SocketAddrV4, local: SocketAddrV4) -> Result<SocketAddrV4, String> {
    let mut nl = PfiocNatlook {
        saddr: PfAddr::from_ipv4(*peer.ip()),
        daddr: PfAddr::from_ipv4(*local.ip()),
        sxport: PfStateXport::from_port(peer.port()),
        dxport: PfStateXport::from_port(local.port()),
        af: AF_INET,
        proto: IPPROTO_TCP,
        direction: PF_IN,
        ..Default::default()
    };

    let ret = unsafe { libc::ioctl(pf_fd, DIOCNATLOOK, &mut nl as *mut PfiocNatlook) };
    if ret < 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("DIOCNATLOOK failed: {}", err));
    }

    let orig_ip = nl.rdaddr.to_ipv4();
    let orig_port = nl.rdxport.to_port();

    Ok(SocketAddrV4::new(orig_ip, orig_port))
}

/// Handle a single redirected connection.
async fn handle_connection(
    inbound: TcpStream,
    pf_fd: RawFd,
    socks_addr: String,
) {
    let peer = match inbound.peer_addr() {
        Ok(SocketAddr::V4(a)) => a,
        Ok(other) => {
            log::debug!("Ignoring non-IPv4 connection from {:?}", other);
            return;
        }
        Err(e) => {
            log::error!("Failed to get peer addr: {}", e);
            return;
        }
    };

    let local = match inbound.local_addr() {
        Ok(SocketAddr::V4(a)) => a,
        Ok(_) | Err(_) => {
            log::error!("Failed to get local addr");
            return;
        }
    };

    // Look up original destination via PF NAT table
    let orig_dst = match natlook(pf_fd, peer, local) {
        Ok(dst) => dst,
        Err(e) => {
            log::error!("NAT lookup failed for {}→{}: {}", peer, local, e);
            return;
        }
    };

    log::info!("Redirecting {} → {} (original dst: {})", peer, local, orig_dst);

    // Connect to original destination through SOCKS5 proxy
    let outbound = match socks5::connect(&socks_addr, *orig_dst.ip(), orig_dst.port()).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("SOCKS5 connect to {} failed: {}", orig_dst, e);
            return;
        }
    };

    // Bidirectional proxy
    let (mut ri, mut wi) = io::split(inbound);
    let (mut ro, mut wo) = io::split(outbound);

    let c2s = tokio::spawn(async move { io::copy(&mut ri, &mut wo).await });
    let s2c = tokio::spawn(async move { io::copy(&mut ro, &mut wi).await });

    let _ = tokio::select! {
        r = c2s => r,
        r = s2c => r,
    };
}

/// Run the transparent redirector.
/// Listens on `listen_addr`, looks up original destinations via /dev/pf,
/// and forwards through SOCKS5 at `socks_addr`.
pub async fn run(
    listen_addr: &str,
    socks_addr: String,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    // Open /dev/pf for DIOCNATLOOK queries
    let pf_fd = unsafe {
        let path = std::ffi::CString::new("/dev/pf").unwrap();
        libc::open(path.as_ptr(), libc::O_RDONLY)
    };
    if pf_fd < 0 {
        let err = std::io::Error::last_os_error();
        return Err(format!("Failed to open /dev/pf: {} (are we running as root?)", err));
    }

    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|e| format!("Failed to bind {}: {}", listen_addr, e))?;

    log::info!("Transparent redirector listening on {}", listen_addr);

    let socks = Arc::new(socks_addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let socks_clone = Arc::clone(&socks);
                        tokio::spawn(async move {
                            handle_connection(stream, pf_fd, (*socks_clone).clone()).await;
                        });
                    }
                    Err(e) => {
                        log::error!("Accept error: {}", e);
                    }
                }
            }
            _ = shutdown.changed() => {
                if *shutdown.borrow() {
                    log::info!("Redirector shutting down");
                    break;
                }
            }
        }
    }

    unsafe { libc::close(pf_fd) };
    Ok(())
}
