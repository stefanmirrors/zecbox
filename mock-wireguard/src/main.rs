//! Mock boringtun (WireGuard userspace) binary for development.
//! Simulates a WireGuard tunnel by staying alive and printing status.

use std::time::Duration;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    eprintln!("mock-wireguard: starting with args: {:?}", &args[1..]);

    if args.iter().any(|a| a == "--config") {
        let config_idx = args.iter().position(|a| a == "--config").unwrap();
        if let Some(path) = args.get(config_idx + 1) {
            eprintln!("mock-wireguard: reading config from {}", path);
        }
    }

    eprintln!("mock-wireguard: creating utun interface");
    std::thread::sleep(Duration::from_secs(1));
    eprintln!("mock-wireguard: interface up");
    eprintln!("mock-wireguard: tunnel established");

    // Keep running until killed
    loop {
        std::thread::sleep(Duration::from_secs(10));
        eprintln!("mock-wireguard: keepalive sent");
    }
}
