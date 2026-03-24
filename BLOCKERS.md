# BLOCKERS

## Phase 5: Shield Mode — zebrad SOCKS Proxy Support

**Status:** Resolved via PF firewall enforcement (March 2026)

### Finding

zebrad (Zebra node) does not support SOCKS proxy configuration in zebrad.toml. The `[network]` section only has: `listen_addr`, `external_addr`, `network`, `initial_mainnet_peers`, `initial_testnet_peers`, `cache_dir`, `peerset_initial_target_size`, `crawl_new_peer_interval`, `max_connections_per_ip`. No proxy or Tor-related fields exist.

The legacy zcashd node supported `-proxy=ip:port` and `-onion=ip:port` flags for SOCKS5 routing, but zebrad's networking layer (`zebra-network` crate) uses raw `tokio::net::TcpStream::connect()` with no proxy interception.

### Solution: PF Firewall + Transparent SOCKS5 Redirector

Shield Mode now enforces Tor routing at the kernel level using macOS PF (packet filter) firewall rules:

1. **Privileged helper daemon** (`zecbox-firewall-helper`): Runs as root via LaunchDaemon, installed at first Shield Mode activation with a one-time admin password prompt
2. **PF anchor rules** (`com.zecbox.shield`): Redirect all outbound Zcash P2P traffic (port 8233) to a local transparent proxy
3. **Transparent SOCKS5 redirector**: Accepts PF-redirected connections, uses DIOCNATLOOK to recover the original destination, and forwards through Arti's SOCKS5 proxy (127.0.0.1:9150)
4. **Kill switch enhanced**: Monitors both Arti process health AND PF firewall status. If either fails, zebrad is immediately stopped

This approach works regardless of zebrad's lack of native proxy support — the kernel enforces the routing before zebrad's networking layer sees it.

### Previous (Removed) Approach

The original Phase 5 implementation set `ALL_PROXY`, `SOCKS5_PROXY`, and `socks_proxy` environment variables when spawning zebrad. These were removed because zebrad's P2P networking ignores standard proxy environment variables.

### Action Items

- [x] PF firewall enforcement implemented
- [x] Transparent SOCKS5 redirector built
- [x] Privileged LaunchDaemon helper with one-time admin install
- [x] Kill switch enhanced to monitor firewall status
- [ ] Monitor Zebra upstream for native SOCKS/Tor support in the Z3 stack (may simplify future implementation)
