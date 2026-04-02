use std::net::SocketAddr;
use std::path::PathBuf;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::signal;

/// Mock Arti Tor proxy for development.
/// Simulates bootstrap progress, SOCKS5 proxy, and hidden service.
/// Accepts real Arti CLI format: `arti proxy -c <config>`
/// Also accepts legacy: `--config <path>` or `--socks-port <port>`

const MOCK_ONION_ADDRESS: &str = "zecboxmock1234567890abcdefghijklmnopqrstuvwxyz23456abcde.onion";

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    let mut socks_port: u16 = 9150;
    let mut config_path: Option<PathBuf> = None;

    // Parse args — support both `proxy -c <path>` and `--config <path>` and `--socks-port <port>`
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "proxy" => {
                // Subcommand — skip it, parse remaining flags
                i += 1;
            }
            "-c" | "--config" => {
                if i + 1 < args.len() {
                    config_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "--socks-port" => {
                if i + 1 < args.len() {
                    socks_port = args[i + 1].parse().unwrap_or(9150);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => { i += 1; }
        }
    }

    // If config provided, parse it for state_dir and socks port
    let mut state_dir: Option<PathBuf> = None;
    if let Some(ref cfg_path) = config_path {
        eprintln!("mock-arti: reading config from {:?}", cfg_path);
        if let Ok(contents) = std::fs::read_to_string(cfg_path) {
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("socks_listen") {
                    if let Some(port_str) = trimmed.split(':').last() {
                        let port_str = port_str.trim().trim_matches('"');
                        if let Ok(p) = port_str.parse::<u16>() {
                            socks_port = p;
                        }
                    }
                }
                if trimmed.starts_with("state_dir") {
                    if let Some(dir) = trimmed.split('=').nth(1) {
                        let dir = dir.trim().trim_matches('"').trim();
                        state_dir = Some(PathBuf::from(dir));
                    }
                }
            }
        }
    }

    // Simulate bootstrap progress
    for pct in (0..=100).step_by(10) {
        eprintln!("BOOTSTRAP PROGRESS={}", pct);
        if pct < 100 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }
    eprintln!("mock-arti: Tor bootstrap complete");

    // Create fake .onion hostname file if state_dir is known
    if let Some(ref dir) = state_dir {
        let hs_dir = dir.join("onion_services").join("zecbox");
        if let Err(e) = std::fs::create_dir_all(&hs_dir) {
            eprintln!("mock-arti: failed to create HS dir: {}", e);
        } else {
            let hostname_path = hs_dir.join("hostname");
            if let Err(e) = std::fs::write(&hostname_path, MOCK_ONION_ADDRESS) {
                eprintln!("mock-arti: failed to write hostname: {}", e);
            } else {
                eprintln!("mock-arti: hidden service ready at {}", MOCK_ONION_ADDRESS);
            }
        }
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], socks_port));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to port {}: {}", socks_port, e));
    eprintln!("mock-arti: SOCKS5 listening on {}", addr);

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (mut stream, _) = match result {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                tokio::spawn(async move {
                    let mut buf = [0u8; 256];
                    if let Ok(n) = stream.read(&mut buf).await {
                        if n >= 2 && buf[0] == 0x05 {
                            let _ = stream.write_all(&[0x05, 0x00]).await;
                            if let Ok(n2) = stream.read(&mut buf).await {
                                if n2 >= 4 && buf[0] == 0x05 && buf[1] == 0x01 {
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
