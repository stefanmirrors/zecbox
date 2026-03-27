//! Transparent SOCKS5 redirector.
//! Accepts firewall-redirected connections, looks up the original destination,
//! and forwards through Arti SOCKS5.
//!
//! macOS: uses DIOCNATLOOK on /dev/pf
//! Linux: uses SO_ORIGINAL_DST getsockopt

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{watch, Semaphore};

use crate::socks5;

const MAX_CONCURRENT_CONNECTIONS: usize = 128;
const CONNECTION_TIMEOUT_SECS: u64 = 120;

// ========================= macOS: DIOCNATLOOK on /dev/pf =========================

#[cfg(target_os = "macos")]
mod macos_natlook {
    use super::*;
    use std::os::fd::RawFd;

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

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    struct PfStateXport {
        port: u16,
        _pad: u16,
    }

    impl PfStateXport {
        fn from_port(port: u16) -> Self {
            PfStateXport { port: port.to_be(), _pad: 0 }
        }

        fn to_port(&self) -> u16 {
            u16::from_be(self.port)
        }
    }

    #[repr(C)]
    #[derive(Copy, Clone, Default)]
    struct PfiocNatlook {
        saddr: PfAddr,
        daddr: PfAddr,
        rsaddr: PfAddr,
        rdaddr: PfAddr,
        sxport: PfStateXport,
        dxport: PfStateXport,
        rsxport: PfStateXport,
        rdxport: PfStateXport,
        af: u8,
        proto: u8,
        direction: u8,
        _pad: [u8; 1],
    }

    const AF_INET: u8 = 2;
    const IPPROTO_TCP: u8 = 6;
    const PF_IN: u8 = 0;
    const DIOCNATLOOK: libc::c_ulong = 0xC054_4417;

    pub fn natlook(pf_fd: RawFd, peer: SocketAddrV4, local: SocketAddrV4) -> Result<SocketAddrV4, String> {
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

        Ok(SocketAddrV4::new(nl.rdaddr.to_ipv4(), nl.rdxport.to_port()))
    }

    pub fn open_pf() -> Result<RawFd, String> {
        let fd = unsafe {
            let path = std::ffi::CString::new("/dev/pf").unwrap();
            libc::open(path.as_ptr(), libc::O_RDONLY)
        };
        if fd < 0 {
            let err = std::io::Error::last_os_error();
            Err(format!("Failed to open /dev/pf: {} (are we running as root?)", err))
        } else {
            Ok(fd)
        }
    }

    pub fn close_pf(fd: RawFd) {
        unsafe { libc::close(fd) };
    }
}

// ========================= Linux: SO_ORIGINAL_DST =========================

#[cfg(target_os = "linux")]
mod linux_natlook {
    use super::*;
    use std::os::fd::AsRawFd;

    // SO_ORIGINAL_DST = 80 (from linux/netfilter_ipv4.h)
    const SO_ORIGINAL_DST: libc::c_int = 80;
    const SOL_IP: libc::c_int = 0;

    pub fn get_original_dst(stream: &TcpStream) -> Result<SocketAddrV4, String> {
        let fd = stream.as_raw_fd();
        let mut addr: libc::sockaddr_in = unsafe { std::mem::zeroed() };
        let mut len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;

        let ret = unsafe {
            libc::getsockopt(
                fd,
                SOL_IP,
                SO_ORIGINAL_DST,
                &mut addr as *mut libc::sockaddr_in as *mut libc::c_void,
                &mut len,
            )
        };
        if ret != 0 {
            let err = std::io::Error::last_os_error();
            return Err(format!("getsockopt SO_ORIGINAL_DST failed: {}", err));
        }

        let ip = Ipv4Addr::from(u32::from_be(addr.sin_addr.s_addr));
        let port = u16::from_be(addr.sin_port);
        Ok(SocketAddrV4::new(ip, port))
    }
}

/// Handle a single redirected connection.
async fn handle_connection(
    inbound: TcpStream,
    #[cfg(target_os = "macos")] pf_fd: std::os::fd::RawFd,
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

    #[cfg(target_os = "macos")]
    let orig_dst = {
        let local = match inbound.local_addr() {
            Ok(SocketAddr::V4(a)) => a,
            Ok(_) | Err(_) => {
                log::error!("Failed to get local addr");
                return;
            }
        };
        match macos_natlook::natlook(pf_fd, peer, local) {
            Ok(dst) => dst,
            Err(e) => {
                log::error!("NAT lookup failed for {}→{}: {}", peer, local, e);
                return;
            }
        }
    };

    #[cfg(target_os = "linux")]
    let orig_dst = match linux_natlook::get_original_dst(&inbound) {
        Ok(dst) => dst,
        Err(e) => {
            log::error!("SO_ORIGINAL_DST lookup failed for {}: {}", peer, e);
            return;
        }
    };

    log::info!("Redirecting {} (original dst: {})", peer, orig_dst);

    let outbound = match socks5::connect(&socks_addr, *orig_dst.ip(), orig_dst.port()).await {
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

/// Run the transparent redirector.
pub async fn run(
    listen_addr: &str,
    socks_addr: String,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let pf_fd = macos_natlook::open_pf()?;

    let listener = TcpListener::bind(listen_addr)
        .await
        .map_err(|e| format!("Failed to bind {}: {}", listen_addr, e))?;

    log::info!("Transparent redirector listening on {}", listen_addr);

    let socks = Arc::new(socks_addr);
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let socks_clone = Arc::clone(&socks);
                        let permit = Arc::clone(&semaphore);
                        #[cfg(target_os = "macos")]
                        let pf_fd_copy = pf_fd;
                        tokio::spawn(async move {
                            let _permit = match permit.try_acquire() {
                                Ok(p) => p,
                                Err(_) => {
                                    log::warn!("Connection limit reached ({}), dropping connection", MAX_CONCURRENT_CONNECTIONS);
                                    return;
                                }
                            };
                            let result = tokio::time::timeout(
                                Duration::from_secs(CONNECTION_TIMEOUT_SECS),
                                handle_connection(
                                    stream,
                                    #[cfg(target_os = "macos")]
                                    pf_fd_copy,
                                    (*socks_clone).clone(),
                                ),
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
                    log::info!("Redirector shutting down");
                    break;
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    macos_natlook::close_pf(pf_fd);
    Ok(())
}
