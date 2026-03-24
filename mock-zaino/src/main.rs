use std::net::SocketAddr;

use tokio::net::TcpListener;
use tokio::signal;

/// Mock Zaino gRPC server for development.
/// Listens on a configurable port and accepts TCP connections.
/// Prints "ZAINO READY port=XXXX" to stderr when listening.

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args
        .windows(2)
        .find(|w| w[0] == "--grpc-port")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(9067);

    let addr = SocketAddr::from(([0, 0, 0, 1], port));
    let listener = TcpListener::bind(addr)
        .await
        .unwrap_or_else(|e| panic!("failed to bind to port {}: {}", port, e));
    eprintln!("ZAINO READY port={}", port);

    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, peer)) => {
                        eprintln!("mock-zaino: connection from {}", peer);
                        // Accept and immediately drop (mock behavior)
                        drop(stream);
                    }
                    Err(e) => {
                        eprintln!("mock-zaino: accept error: {}", e);
                    }
                }
            }
            _ = signal::ctrl_c() => {
                eprintln!("mock-zaino: shutting down");
                break;
            }
        }
    }
}
