//! Minimal SOCKS5 client — connect command only.
//! Used by the transparent redirector to forward connections through Arti.

use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Connect to a destination through a SOCKS5 proxy.
/// Returns the connected stream ready for data transfer.
pub async fn connect(
    proxy_addr: &str,
    dest_ip: Ipv4Addr,
    dest_port: u16,
) -> Result<TcpStream, String> {
    let mut stream = TcpStream::connect(proxy_addr)
        .await
        .map_err(|e| format!("Failed to connect to SOCKS5 proxy at {}: {}", proxy_addr, e))?;

    // SOCKS5 greeting: version 5, 1 auth method (no auth)
    stream
        .write_all(&[0x05, 0x01, 0x00])
        .await
        .map_err(|e| format!("SOCKS5 greeting write failed: {}", e))?;

    // Read server's chosen auth method
    let mut auth_resp = [0u8; 2];
    stream
        .read_exact(&mut auth_resp)
        .await
        .map_err(|e| format!("SOCKS5 auth response read failed: {}", e))?;

    if auth_resp[0] != 0x05 || auth_resp[1] != 0x00 {
        return Err(format!(
            "SOCKS5 auth failed: version={}, method={}",
            auth_resp[0], auth_resp[1]
        ));
    }

    // SOCKS5 connect request: version 5, cmd connect (1), reserved, addr type IPv4 (1)
    let dest_addr = SocketAddrV4::new(dest_ip, dest_port);
    let mut request = Vec::with_capacity(10);
    request.push(0x05); // version
    request.push(0x01); // connect
    request.push(0x00); // reserved
    request.push(0x01); // IPv4
    request.extend_from_slice(&dest_addr.ip().octets());
    request.push((dest_port >> 8) as u8);
    request.push((dest_port & 0xFF) as u8);

    stream
        .write_all(&request)
        .await
        .map_err(|e| format!("SOCKS5 connect request write failed: {}", e))?;

    // Read connect response (minimum 10 bytes for IPv4)
    let mut resp = [0u8; 10];
    stream
        .read_exact(&mut resp)
        .await
        .map_err(|e| format!("SOCKS5 connect response read failed: {}", e))?;

    if resp[0] != 0x05 {
        return Err(format!("SOCKS5 invalid version in response: {}", resp[0]));
    }

    if resp[1] != 0x00 {
        let reason = match resp[1] {
            0x01 => "general SOCKS server failure",
            0x02 => "connection not allowed by ruleset",
            0x03 => "network unreachable",
            0x04 => "host unreachable",
            0x05 => "connection refused",
            0x06 => "TTL expired",
            0x07 => "command not supported",
            0x08 => "address type not supported",
            _ => "unknown error",
        };
        return Err(format!(
            "SOCKS5 connect to {}:{} failed: {} (code {})",
            dest_ip, dest_port, reason, resp[1]
        ));
    }

    Ok(stream)
}
