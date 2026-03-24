use std::net::SocketAddr;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::signal;

/// Mock Arti SOCKS5 proxy for development.
/// Simulates bootstrap progress output and accepts SOCKS5 connections
/// (responds with connection-refused to all proxy requests).

#[tokio::main]
async fn main() {
    // Accept --socks-port <port> for configurability
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args
        .windows(2)
        .find(|w| w[0] == "--socks-port")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(9150);

    // Simulate bootstrap progress
    for pct in (0..=100).step_by(10) {
        eprintln!("BOOTSTRAP PROGRESS={}", pct);
        if pct < 100 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    eprintln!("mock-arti: Tor bootstrap complete");

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to port {}: {}", port, e));
    eprintln!("mock-arti: SOCKS5 listening on {}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (mut stream, _) = match result {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                // Handle minimal SOCKS5 handshake then close
                tokio::spawn(async move {
                    let mut buf = [0u8; 256];
                    if let Ok(n) = stream.read(&mut buf).await {
                        if n >= 2 && buf[0] == 0x05 {
                            // SOCKS5 greeting: respond with no-auth
                            let _ = stream.write_all(&[0x05, 0x00]).await;
                            // Read connect request and respond with general failure
                            if let Ok(n2) = stream.read(&mut buf).await {
                                if n2 >= 4 && buf[0] == 0x05 && buf[1] == 0x01 {
                                    // Reply: general failure (0x01)
                                    let _ = stream.write_all(&[
                                        0x05, 0x05, 0x00, 0x01,
                                        0x00, 0x00, 0x00, 0x00,
                                        0x00, 0x00,
                                    ]).await;
                                }
                            }
                        }
                    }
                });
            }
            _ = signal::ctrl_c() => {
                eprintln!("mock-arti: shutting down");
                break;
            }
        }
    }
}
