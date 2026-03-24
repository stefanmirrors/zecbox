# BLOCKERS

## Phase 5: Shield Mode — zebrad SOCKS Proxy Support

**Status:** Confirmed unsupported as of March 2026

### Finding

zebrad (Zebra node) does not support SOCKS proxy configuration in zebrad.toml. The `[network]` section only has: `listen_addr`, `external_addr`, `network`, `initial_mainnet_peers`, `initial_testnet_peers`, `cache_dir`, `peerset_initial_target_size`, `crawl_new_peer_interval`, `max_connections_per_ip`. No proxy or Tor-related fields exist.

The legacy zcashd node supported `-proxy=ip:port` and `-onion=ip:port` flags for SOCKS5 routing, but zebrad's networking layer (`zebra-network` crate) uses raw `tokio::net::TcpStream::connect()` with no proxy interception.

The Zcash Foundation has native Tor support on the roadmap as part of the Z3 stack (Zebra + Zaino + Zallet), but it is not yet implemented.

### Current Implementation (Phase 5)

Shield Mode is implemented with the following approach:

1. **Arti sidecar**: Managed as a child process (same pattern as zebrad), exposes SOCKS5 on 127.0.0.1:9150
2. **Config restriction**: When Shield ON, zebrad.toml is regenerated with `listen_addr = "127.0.0.1:8233"` (no external listening) and known `.onion` seed peers
3. **Process wrapping**: zebrad is spawned with `ALL_PROXY` and `SOCKS5_PROXY` environment variables pointing to the Arti SOCKS port. This works for any network calls that respect standard proxy env vars.
4. **Kill switch**: If Arti dies while Shield ON, zebrad is immediately stopped. No clearnet fallback.

### Limitation

zebrad's P2P networking layer does not respect proxy environment variables. The current implementation provides the full Shield Mode UX and infrastructure (Arti lifecycle, kill switch, config restriction, UI toggle), but **actual P2P traffic routing through Tor requires one of**:

1. **Upstream zebrad patch** — Add SOCKS proxy support to `zebra-network` Config (preferred, track upstream progress)
2. **torsocks wrapping** — Use `DYLD_INSERT_LIBRARIES` to intercept socket calls at the OS level (fragile on macOS with SIP)
3. **PF firewall rules** — macOS packet filter to redirect zebrad traffic through Arti transparent proxy (requires root)

### Action Items

- [ ] Monitor Zebra upstream for native SOCKS/Tor support in the Z3 stack
- [ ] Consider contributing a SOCKS proxy PR to ZcashFoundation/zebra
- [ ] Evaluate torsocks wrapping feasibility on macOS with SIP enabled
