use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use http_body_util::{BodyExt, Full};
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use tokio::signal;

static BLOCK_HEIGHT: AtomicU64 = AtomicU64::new(1_000_000);

async fn handle_request(
    req: Request<Incoming>,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    if req.method() != hyper::Method::POST {
        let resp = Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .body(Full::new(Bytes::from("POST only")))
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
                    "subversion": "/Zebra:1.8.0/",
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
                    "estimatedheight": 2_500_000u64,
                    "upgrades": {},
                    "consensus": { "chaintip": "c2d6d0b4", "nextblock": "c2d6d0b4" }
                },
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
    // Accept --config <path> for compatibility with real zebrad invocation (ignored)
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--config" {
        eprintln!("mock-zebrad: ignoring config file {}", args[2]);
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 8232));
    let listener = TcpListener::bind(addr).await.expect("failed to bind to port 8232");
    eprintln!("mock-zebrad: listening on {}", addr);

    // Increment block height in the background to simulate syncing
    tokio::spawn(async {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            BLOCK_HEIGHT.fetch_add(1, Ordering::Relaxed);
        }
    });

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
