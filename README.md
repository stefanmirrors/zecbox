# zecbox (Under Development)

**One click. Full node.**

zecbox turns your computer into a Zcash full node. Download, install, click start. No terminal. No Docker. No configuration files.

---

## What It Does

- Runs a [Zebra](https://github.com/ZcashFoundation/zebra) full node (zebrad) behind a clean dashboard
- **Shield Mode** — route all node traffic through Tor with one toggle. If Tor drops, the node stops. It will never silently fall back to clearnet.
- **Wallet Server** — serve your own light wallet backend (Zaino) so ZODL, Ywallet, or any compatible wallet can sync from _your_ node instead of someone else's
- Monitors storage, network peers, sync progress, and node health in real time
- Survives reboots, sleep, crashes, and power loss without losing chain data
- Lives in your system tray — close the window, the node keeps running

## What It Doesn't Do

- It doesn't hold your keys. zecbox is not a wallet.
- It doesn't mine.
- It doesn't require you to know what a terminal is.

---

## Requirements

- **Storage:** ~300 GB free (the Zcash blockchain). External drives supported.
- **OS:** macOS (v1). Windows, Linux, and Android are planned.
- **RAM:** 4 GB minimum. 8 GB recommended.
- **Internet:** Required for syncing. Initial sync takes 1–3 days depending on connection speed.

---

## Install

1. Download the `.dmg` from [Releases](https://github.com/stefanmirrors/zecbox/releases)
2. Open the `.dmg`, drag zecbox to Applications
3. Launch zecbox
4. Pick your storage location (or accept the default)
5. Click **Start**

Your node is running. That's it.

### First Launch (Unsigned Builds)

Until official signed releases are available, macOS Gatekeeper will block the app. To open it:

1. Try opening zecbox normally (it will be blocked)
2. Open **System Settings > Privacy & Security**
3. Scroll down -- you'll see "zecbox was blocked"
4. Click **Open Anyway**

This is only needed once. Signed releases will not require this step.

---

## Verify Your Download

Every release includes a `SHA256SUMS` file. To verify:

```bash
shasum -a 256 -c SHA256SUMS
```

---

## Architecture

zecbox bundles three binaries as sidecars, managed as child processes:

- **zebrad** -- Zcash full node ([Zebra](https://github.com/ZcashFoundation/zebra)). Syncs the blockchain, serves JSON-RPC on localhost.
- **Zaino** -- Light wallet gRPC server. Reads from zebrad, serves compact blocks to wallets on port 9067.
- **Arti** -- Tor SOCKS5 proxy (Shield Mode). Routes zebrad traffic through the Tor network.

No Docker. No external dependencies. Everything runs as native processes managed by the Rust backend.

---

## Build from Source

### Requirements

- macOS 12+ (Monterey or later)
- Rust toolchain ([rustup.rs](https://rustup.rs))
- Node.js 18+
- Xcode Command Line Tools (`xcode-select --install`)

### Steps

```bash
git clone https://github.com/stefanmirrors/zecbox.git
cd zecbox
npm install
./scripts/build-macos.sh
```

The DMG will be at `src-tauri/target/release/bundle/dmg/zecbox_<version>_aarch64.dmg`.

The build script automatically compiles mock sidecar binaries for development. For production builds with real zebrad/Zaino/Arti binaries, place them in `src-tauri/binaries/` before building.

---

## License

MIT
