//! Resolve Zcash DNS seeders through Tor using DNS-over-TCP via SOCKS5.
//! Prevents DNS leaks in Shield Mode by routing all DNS queries through Arti.

use std::net::Ipv4Addr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

const ARTI_SOCKS: &str = "127.0.0.1:9150";
const QUERY_TIMEOUT: Duration = Duration::from_secs(15);

/// Zcash mainnet DNS seeders — same ones used in clearnet zebrad.toml.
const MAINNET_SEEDERS: &[&str] = &[
    "dnsseed.z.cash",
    "dnsseed.str4d.xyz",
    "mainnet.seeder.zfnd.org",
    "mainnet.is.yolo.money",
];

/// Resolve all Zcash DNS seeders through Tor, returning IP:port strings.
/// Queries each seeder's DNS server directly through Arti SOCKS5.
/// Returns Err if zero IPs are resolved (Shield Mode cannot proceed).
pub async fn resolve_seeders_via_tor() -> Result<Vec<String>, String> {
    let mut all_peers = Vec::new();

    // Resolve all seeders in parallel
    let mut join_set = tokio::task::JoinSet::new();
    for seeder in MAINNET_SEEDERS {
        let seeder = seeder.to_string();
        join_set.spawn(async move {
            let result = resolve_single_seeder(&seeder).await;
            (seeder, result)
        });
    }

    while let Some(result) = join_set.join_next().await {
        match result {
            Ok((seeder, Ok(ips))) => {
                log::info!("Resolved {} IPs from {} through Tor", ips.len(), seeder);
                for ip in ips {
                    let peer = format!("{}:8233", ip);
                    if !all_peers.contains(&peer) {
                        all_peers.push(peer);
                    }
                }
            }
            Ok((seeder, Err(e))) => {
                log::warn!("Failed to resolve {} through Tor: {}", seeder, e);
            }
            Err(e) => {
                log::warn!("Seeder resolution task panicked: {}", e);
            }
        }
    }

    if all_peers.is_empty() {
        return Err(
            "Could not resolve any Zcash peers through Tor. Check your network connection.".into(),
        );
    }

    log::info!(
        "Resolved {} unique Zcash peers through Tor",
        all_peers.len()
    );
    Ok(all_peers)
}

/// Resolve a single DNS seeder through Tor.
/// Connects to the seeder's DNS port (53) via SOCKS5 domain connect,
/// sends a DNS A query, and parses the response for IPv4 addresses.
async fn resolve_single_seeder(hostname: &str) -> Result<Vec<Ipv4Addr>, String> {
    tokio::time::timeout(QUERY_TIMEOUT, resolve_single_seeder_inner(hostname))
        .await
        .map_err(|_| format!("Timeout resolving {} through Tor", hostname))?
}

async fn resolve_single_seeder_inner(hostname: &str) -> Result<Vec<Ipv4Addr>, String> {
    // Connect to Arti SOCKS5 proxy
    let mut stream = TcpStream::connect(ARTI_SOCKS)
        .await
        .map_err(|e| format!("Cannot connect to Arti SOCKS5: {}", e))?;

    // SOCKS5 handshake: version 5, 1 method, no auth
    stream
        .write_all(&[0x05, 0x01, 0x00])
        .await
        .map_err(|e| format!("SOCKS5 greeting failed: {}", e))?;

    let mut auth_resp = [0u8; 2];
    stream
        .read_exact(&mut auth_resp)
        .await
        .map_err(|e| format!("SOCKS5 auth read failed: {}", e))?;

    if auth_resp[0] != 0x05 || auth_resp[1] != 0x00 {
        return Err(format!(
            "SOCKS5 auth rejected: ver={} method={}",
            auth_resp[0], auth_resp[1]
        ));
    }

    // SOCKS5 CONNECT with domain address type (0x03) to hostname:53
    // Arti resolves the hostname internally through Tor
    let hostname_bytes = hostname.as_bytes();
    let mut request = Vec::with_capacity(7 + hostname_bytes.len());
    request.push(0x05); // version
    request.push(0x01); // CONNECT
    request.push(0x00); // reserved
    request.push(0x03); // address type: domain name
    request.push(hostname_bytes.len() as u8);
    request.extend_from_slice(hostname_bytes);
    request.push(0x00); // port high byte (53)
    request.push(0x35); // port low byte (53)

    stream
        .write_all(&request)
        .await
        .map_err(|e| format!("SOCKS5 connect request failed: {}", e))?;

    // Read SOCKS5 response — variable length depending on address type
    let mut resp_header = [0u8; 4];
    stream
        .read_exact(&mut resp_header)
        .await
        .map_err(|e| format!("SOCKS5 response read failed: {}", e))?;

    if resp_header[1] != 0x00 {
        let reason = match resp_header[1] {
            0x01 => "general failure",
            0x02 => "not allowed",
            0x03 => "network unreachable",
            0x04 => "host unreachable",
            0x05 => "connection refused",
            0x06 => "TTL expired",
            0x07 => "command not supported",
            0x08 => "address type not supported",
            _ => "unknown",
        };
        return Err(format!(
            "SOCKS5 connect to {}:53 failed: {} (code {})",
            hostname, reason, resp_header[1]
        ));
    }

    // Skip the rest of the SOCKS5 response (BND.ADDR + BND.PORT)
    match resp_header[3] {
        0x01 => {
            // IPv4: 4 bytes addr + 2 bytes port
            let mut skip = [0u8; 6];
            stream.read_exact(&mut skip).await.ok();
        }
        0x03 => {
            // Domain: 1 byte len + domain + 2 bytes port
            let mut len = [0u8; 1];
            stream.read_exact(&mut len).await.ok();
            let mut skip = vec![0u8; len[0] as usize + 2];
            stream.read_exact(&mut skip).await.ok();
        }
        0x04 => {
            // IPv6: 16 bytes addr + 2 bytes port
            let mut skip = [0u8; 18];
            stream.read_exact(&mut skip).await.ok();
        }
        _ => {}
    }

    // Now we have a TCP connection to the seeder's DNS port through Tor.
    // Send a DNS-over-TCP A query for the seeder hostname.
    let query = build_dns_a_query(hostname);

    // DNS-over-TCP: 2-byte length prefix + DNS message
    let query_len = query.len() as u16;
    stream
        .write_all(&query_len.to_be_bytes())
        .await
        .map_err(|e| format!("DNS query length write failed: {}", e))?;
    stream
        .write_all(&query)
        .await
        .map_err(|e| format!("DNS query write failed: {}", e))?;

    // Read DNS-over-TCP response
    let mut resp_len_buf = [0u8; 2];
    stream
        .read_exact(&mut resp_len_buf)
        .await
        .map_err(|e| format!("DNS response length read failed: {}", e))?;
    let resp_len = u16::from_be_bytes(resp_len_buf) as usize;

    if resp_len > 4096 {
        return Err(format!("DNS response too large: {} bytes", resp_len));
    }

    let mut response = vec![0u8; resp_len];
    stream
        .read_exact(&mut response)
        .await
        .map_err(|e| format!("DNS response read failed: {}", e))?;

    // Parse A records from the DNS response
    parse_dns_a_records(&response)
}

/// Build a minimal DNS A query for the given hostname.
fn build_dns_a_query(hostname: &str) -> Vec<u8> {
    let mut query = Vec::with_capacity(64);

    // Header (12 bytes)
    let id: u16 = rand::random();
    query.extend_from_slice(&id.to_be_bytes()); // ID
    query.extend_from_slice(&[0x01, 0x00]); // Flags: standard query, recursion desired
    query.extend_from_slice(&[0x00, 0x01]); // QDCOUNT: 1 question
    query.extend_from_slice(&[0x00, 0x00]); // ANCOUNT: 0
    query.extend_from_slice(&[0x00, 0x00]); // NSCOUNT: 0
    query.extend_from_slice(&[0x00, 0x00]); // ARCOUNT: 0

    // Question section: hostname as length-prefixed labels
    for label in hostname.split('.') {
        query.push(label.len() as u8);
        query.extend_from_slice(label.as_bytes());
    }
    query.push(0x00); // root label terminator

    query.extend_from_slice(&[0x00, 0x01]); // QTYPE: A (1)
    query.extend_from_slice(&[0x00, 0x01]); // QCLASS: IN (1)

    query
}

/// Parse A records (IPv4 addresses) from a DNS response message.
fn parse_dns_a_records(response: &[u8]) -> Result<Vec<Ipv4Addr>, String> {
    if response.len() < 12 {
        return Err("DNS response too short".into());
    }

    // Check response code (RCODE) in flags
    let rcode = response[3] & 0x0F;
    if rcode != 0 {
        return Err(format!("DNS query failed with RCODE {}", rcode));
    }

    let qdcount = u16::from_be_bytes([response[4], response[5]]) as usize;
    let ancount = u16::from_be_bytes([response[6], response[7]]) as usize;

    if ancount == 0 {
        return Err("DNS response contains no answers".into());
    }

    // Skip header (12 bytes) and question section
    let mut pos = 12;
    for _ in 0..qdcount {
        pos = skip_dns_name(response, pos)?;
        pos += 4; // QTYPE (2) + QCLASS (2)
    }

    // Parse answer section
    let mut ips = Vec::new();
    for _ in 0..ancount {
        if pos >= response.len() {
            break;
        }

        // Skip name (may be compressed)
        pos = skip_dns_name(response, pos)?;

        if pos + 10 > response.len() {
            break;
        }

        let rtype = u16::from_be_bytes([response[pos], response[pos + 1]]);
        let rdlength = u16::from_be_bytes([response[pos + 8], response[pos + 9]]) as usize;
        pos += 10; // TYPE(2) + CLASS(2) + TTL(4) + RDLENGTH(2)

        if rtype == 1 && rdlength == 4 && pos + 4 <= response.len() {
            // A record: 4 bytes IPv4
            let ip = Ipv4Addr::new(response[pos], response[pos + 1], response[pos + 2], response[pos + 3]);
            ips.push(ip);
        }

        pos += rdlength;
    }

    if ips.is_empty() {
        return Err("DNS response contains no A records".into());
    }

    Ok(ips)
}

/// Skip a DNS name in a response (handles label compression).
fn skip_dns_name(data: &[u8], mut pos: usize) -> Result<usize, String> {
    if pos >= data.len() {
        return Err("DNS name: position out of bounds".into());
    }

    loop {
        if pos >= data.len() {
            return Err("DNS name: unexpected end of data".into());
        }

        let len = data[pos];

        if len == 0 {
            // End of name
            return Ok(pos + 1);
        }

        if len & 0xC0 == 0xC0 {
            // Compressed pointer: 2 bytes total, name ends here
            return Ok(pos + 2);
        }

        // Regular label: skip length byte + label bytes
        pos += 1 + len as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_dns_query() {
        let query = build_dns_a_query("dnsseed.z.cash");
        // Header: 12 bytes
        assert_eq!(query[2], 0x01); // flags: recursion desired
        assert_eq!(query[5], 0x01); // QDCOUNT: 1
        // Question should end with QTYPE=A, QCLASS=IN
        let len = query.len();
        assert_eq!(query[len - 4..], [0x00, 0x01, 0x00, 0x01]);
    }

    #[test]
    fn test_parse_a_records() {
        // Minimal DNS response with 1 A record for "example.com" → 1.2.3.4
        let mut resp = Vec::new();
        // Header
        resp.extend_from_slice(&[0x00, 0x01]); // ID
        resp.extend_from_slice(&[0x81, 0x80]); // Flags: response, recursion
        resp.extend_from_slice(&[0x00, 0x01]); // QDCOUNT: 1
        resp.extend_from_slice(&[0x00, 0x01]); // ANCOUNT: 1
        resp.extend_from_slice(&[0x00, 0x00]); // NSCOUNT
        resp.extend_from_slice(&[0x00, 0x00]); // ARCOUNT
        // Question: example.com A IN
        resp.push(7); resp.extend_from_slice(b"example");
        resp.push(3); resp.extend_from_slice(b"com");
        resp.push(0);
        resp.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]); // QTYPE, QCLASS
        // Answer: compressed name pointer, A record
        resp.extend_from_slice(&[0xC0, 0x0C]); // name pointer to offset 12
        resp.extend_from_slice(&[0x00, 0x01]); // TYPE: A
        resp.extend_from_slice(&[0x00, 0x01]); // CLASS: IN
        resp.extend_from_slice(&[0x00, 0x00, 0x01, 0x00]); // TTL: 256
        resp.extend_from_slice(&[0x00, 0x04]); // RDLENGTH: 4
        resp.extend_from_slice(&[1, 2, 3, 4]); // RDATA: 1.2.3.4

        let ips = parse_dns_a_records(&resp).unwrap();
        assert_eq!(ips, vec![Ipv4Addr::new(1, 2, 3, 4)]);
    }
}
