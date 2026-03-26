use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::time::{sleep, Duration};

static BLOCK_HEIGHT: AtomicU64 = AtomicU64::new(0);
static RPC_READY: AtomicBool = AtomicBool::new(false);

async fn handle_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() != hyper::Method::POST || !RPC_READY.load(Ordering::Relaxed) {
        let resp = Response::builder()
            .status(StatusCode::SERVICE_UNAVAILABLE)
            .body(Full::new(Bytes::from("not ready")))
            .unwrap();
        return Ok(resp);
    }

    let body = req.collect().await?.to_bytes();
    let parsed: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(_) => {
            let resp = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Full::new(Bytes::from("invalid JSON")))
                .unwrap();
            return Ok(resp);
        }
    };

    let method = parsed.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = parsed.get("id").cloned().unwrap_or(serde_json::Value::Null);

    let response_body = match method {
        "getinfo" => {
            let height = BLOCK_HEIGHT.fetch_add(1, Ordering::Relaxed);
            serde_json::json!({
                "result": {
                    "version": 5_070_050,
                    "subversion": "/Zebra:4.2.0/",
                    "blocks": height,
                    "connections": 8,
                    "proxy": ""
                },
                "id": id,
                "error": null
            })
        }
        "getblockchaininfo" => {
            let height = BLOCK_HEIGHT.load(Ordering::Relaxed);
            serde_json::json!({
                "result": {
                    "chain": "main",
                    "blocks": height,
                    "bestblockhash": "0000000000000000000000000000000000000000000000000000000000000000",
                    "estimatedheight": 3_300_000u64,
                    "upgrades": {},
                    "consensus": { "chaintip": "c2d6d0b4", "nextblock": "c2d6d0b4" }
                },
                "id": id,
                "error": null
            })
        }
        "getpeerinfo" => {
            serde_json::json!({
                "result": [
                    { "addr": "203.0.113.5:8233", "inbound": false, "conntime": 1700000000 },
                    { "addr": "198.51.100.10:8233", "inbound": false, "conntime": 1700000100 },
                    { "addr": "192.0.2.42:8233", "inbound": false, "conntime": 1700000200 },
                    { "addr": "10.0.0.5:8233", "inbound": false, "conntime": 1700000300 },
                    { "addr": "172.16.0.8:8233", "inbound": false, "conntime": 1700000400 },
                    { "addr": "198.51.100.77:8233", "inbound": true, "conntime": 1700000500 },
                    { "addr": "203.0.113.99:8233", "inbound": true, "conntime": 1700000600 },
                    { "addr": "192.0.2.101:8233", "inbound": true, "conntime": 1700000700 }
                ],
                "id": id,
                "error": null
            })
        }
        _ => {
            serde_json::json!({
                "result": null,
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            })
        }
    };

    let resp = Response::builder()
        .header("content-type", "application/json")
        .body(Full::new(Bytes::from(response_body.to_string())))
        .unwrap();
    Ok(resp)
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--config" {
        eprintln!("mock-zebrad: ignoring config file {}", args[2]);
    }

    // Simulate real zebrad startup sequence via stderr/stdout
    tokio::spawn(async {
        // Stage 1: Starting
        eprintln!("Thank you for running a mainnet zebrad 4.2.0 node!");
        eprintln!("You're helping to strengthen the network and contributing to a social good :)");
        sleep(Duration::from_secs(3)).await;

        // Stage 2: Database
        eprintln!("INFO zebrad::commands::start: opening database, this may take a few minutes");
        eprintln!("INFO zebra_state: creating new database running_version=27.0.0");
        sleep(Duration::from_secs(4)).await;

        // Stage 3: Network
        eprintln!("INFO zebrad::commands::start: initializing network");
        sleep(Duration::from_secs(3)).await;

        // Stage 4: Peers
        eprintln!("INFO add_initial_peers: zebra_network::peer_set::initialize: connecting to initial peer set ipv4_peer_count=23 ipv6_peer_count=2");
        sleep(Duration::from_secs(4)).await;
        eprintln!("INFO add_initial_peers: zebra_network::peer_set::initialize: finished connecting to initial seed and disk cache peers handshake_success_total=15 active_initial_peer_count=15");
        sleep(Duration::from_secs(2)).await;

        // Stage 5: Verifiers
        eprintln!("INFO zebrad::commands::start: initializing verifiers");
        eprintln!("INFO init: zebra_consensus::router: starting state checkpoint validation");
        sleep(Duration::from_secs(3)).await;

        // Stage 6: RPC ready
        eprintln!("INFO zebra_rpc::server: Opened RPC endpoint at 127.0.0.1:8232");
        RPC_READY.store(true, Ordering::Relaxed);
        sleep(Duration::from_secs(1)).await;

        // Stage 7: Checkpoints (simulate with rising block heights)
        let checkpoints = [1200, 12000, 96000, 384000, 900000, 1600000, 2400000, 3000000, 3200000];
        for &height in &checkpoints {
            BLOCK_HEIGHT.store(height, Ordering::Relaxed);
            eprintln!(
                "INFO sync:checkpoint: zebra_consensus::checkpoint: verified checkpoint range block_count=400 current_range=(Excluded(Height({})), Included(Height({})))",
                height.saturating_sub(400), height
            );
            let pct = height as f64 / 3_300_000.0 * 100.0;
            eprintln!(
                "INFO zebrad::components::sync::progress: estimated progress to chain tip sync_percent={:.3}% current_height=Height({}) remaining_sync_blocks={}",
                pct, height, 3_300_000u64.saturating_sub(height)
            );
            sleep(Duration::from_secs(3)).await;
        }

        // Stage 8: Fully synced
        BLOCK_HEIGHT.store(3_300_000, Ordering::Relaxed);
        eprintln!("INFO zebrad::components::sync::progress: estimated progress to chain tip sync_percent=100.000% current_height=Height(3300000) remaining_sync_blocks=0");

        // Keep incrementing slowly like a synced node
        loop {
            sleep(Duration::from_millis(500)).await;
            BLOCK_HEIGHT.fetch_add(1, Ordering::Relaxed);
        }
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], 8232));
    let listener = TcpListener::bind(addr).await.expect("failed to bind to port 8232");
    eprintln!("mock-zebrad: listening on {}", addr);

    let graceful = Arc::new(tokio::sync::Notify::new());
    let graceful_clone = graceful.clone();

    tokio::spawn(async move {
        signal::ctrl_c().await.ok();
        eprintln!("mock-zebrad: shutting down");
        graceful_clone.notify_waiters();
    });

    loop {
        tokio::select! {
            result = listener.accept() => {
                let (stream, _) = match result {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("mock-zebrad: accept error: {}", e);
                        continue;
                    }
                };
                let io = TokioIo::new(stream);
                tokio::spawn(async move {
                    if let Err(e) = http1::Builder::new()
                        .serve_connection(io, service_fn(handle_request))
                        .await
                    {
                        eprintln!("mock-zebrad: connection error: {}", e);
                    }
                });
            }
            _ = graceful.notified() => {
                eprintln!("mock-zebrad: stopped");
                break;
            }
        }
    }
}
