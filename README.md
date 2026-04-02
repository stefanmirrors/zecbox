# zecbox

Zcash full node in one click. Any device. No terminal. No config files. Fully open source.

**Website:** [zecbox.io](https://zecbox.io)

## What is zecbox?

zecbox is a desktop application that lets you run a full Zcash node without touching a command line. It bundles everything you need into a single installer: [zebrad](https://github.com/ZcashFoundation/zebra) (the Zcash node), [Zaino](https://github.com/zingolabs/zaino) (a light wallet server), and [Arti](https://gitlab.torproject.org/tpo/core/arti) (a Tor client for private networking). All configuration is generated and managed by the app. You pick a storage location, click start, and the node takes care of itself.

zecbox is built with [Tauri](https://tauri.app/) (Rust backend + native webview) and React. It runs on macOS, Linux, and Windows.

## Features

- **One-click setup:** Guided onboarding walks you through storage selection and privacy preferences. Three decisions, then you're syncing.
- **Real-time dashboard:** Live sync progress, peer count, block height, storage usage, and uptime stats. Updated every two seconds.
- **Shield Mode:** Route all node traffic through Tor with a single toggle. Includes a kill switch: if the Tor connection drops, the node stops immediately rather than falling back to clearnet. Your IP is never exposed.
- **Wallet Server:** Enable Zaino to serve light wallets over gRPC (port 9067). Shows the connection endpoint and a QR code for easy pairing.
- **System tray:** Closing the window keeps the node running in the background. The tray icon shows current status. "Quit zecbox" stops everything.
- **Auto-updates:** The app checks for updates to itself and to the bundled binaries (zebrad, Zaino, Arti). Binary updates are verified with SHA256 before swapping.
- **External drive support:** Store the blockchain on an external drive. If the drive disconnects, the app pauses cleanly and tells you to reconnect.
- **Sleep/wake recovery:** On macOS, the app detects sleep/wake events and reconnects automatically. On other platforms, health checks handle it.
- **Launch at login:** Optional setting to start zecbox when you log in.

## Downloads

Download the latest release from [zecbox.io](https://zecbox.io/#downloads) or from [GitHub Releases](https://github.com/stefanmirrors/zecbox/releases/latest).

| Platform    | Architecture          | Format                  |
| ----------- | --------------------- | ----------------------- |
| macOS 12+   | Apple Silicon (M1-M4) | `.dmg`                  |
| macOS 12+   | Intel                 | `.dmg`                  |
| Linux       | x86_64                | `.deb`, `.AppImage`     |
| Linux       | ARM (aarch64)         | `.deb`, `.AppImage`     |
| Windows 10+ | x86_64                | `.exe` (NSIS installer) |

You will need roughly **300 GB** of free disk space for the full Zcash blockchain.

## Installation

### macOS

1. Open the downloaded `.dmg` and drag zecbox into your Applications folder.
2. Open Terminal and run:
   ```
   xattr -cr /Applications/zecbox.app
   ```
   This is needed because the app is not yet code-signed. It clears the quarantine flag so macOS will let you open it.
3. Open zecbox from Applications.

### Linux (.deb)

```
sudo dpkg -i ~/Downloads/zecbox*.deb
```

Then open zecbox from your application menu.

### Linux (AppImage)

```
chmod +x ~/Downloads/zecbox*.AppImage
~/Downloads/zecbox*.AppImage
```

### Windows

1. Run the downloaded `.exe` installer.
2. If Windows SmartScreen appears, click **More info**, then **Run anyway**.
3. Open zecbox from the Start menu.

## How It Works

zecbox manages three processes behind the scenes:

- **zebrad:**The Zcash node. Syncs the blockchain, validates transactions, and connects to the peer-to-peer network. Listens for JSON-RPC on `127.0.0.1:8232` and P2P connections on port `8233`.
- **Zaino:**A light wallet indexing server. Reads from zebrad and serves light wallet clients over gRPC on port `9067`. Only runs when you enable the Wallet Server toggle.
- **Arti:**A Tor client. Provides a SOCKS5 proxy for routing zebrad traffic through the Tor network. Only runs when Shield Mode is enabled.

All three are bundled as sidecar binaries inside the app. The Rust backend spawns and monitors them, restarts them on crash (with exponential backoff), and writes their config files on the fly. The React frontend communicates with the backend over Tauri's IPC, and the backend pushes status updates to the frontend via events.

### Data directory

By default, zecbox stores everything under:

- **macOS:** `~/Library/Application Support/com.zecbox.app/`
- **Linux:** `~/.local/share/com.zecbox.app/`
- **Windows:** `%APPDATA%/com.zecbox.app/`

Inside that directory:

```
com.zecbox.app/
  zebra/          # Blockchain data (RocksDB, ~300 GB)
  zaino/          # Wallet index data
  config/         # zebrad.toml, zaino.toml, zecbox.json (all generated)
  logs/           # Rotated logs, 7-day retention
```

You can also choose an external drive during onboarding. The app stores a pointer to the chosen location and checks that the drive is mounted on every launch.

## Building from Source

### Prerequisites

**All platforms:**

- [Rust](https://rustup.rs/) 1.70 or later
- [Node.js](https://nodejs.org/) 20 or later

**macOS:**

- Xcode Command Line Tools (`xcode-select --install`)

**Linux (Debian/Ubuntu):**

```
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

**Windows:**

- Visual Studio 2022 with C++ build tools (Community edition works fine)

### Development

```bash
git clone https://github.com/stefanmirrors/zecbox.git
cd zecbox

# Install frontend dependencies
npm install

# Build mock sidecar binaries (these simulate zebrad/Zaino/Arti for development)
cargo build -p mock-zebrad --release
cargo build -p mock-arti --release
cargo build -p mock-zaino --release
cargo build -p firewall-helper --release

# Copy them where Tauri expects them (replace the target triple with yours)
# macOS Apple Silicon: aarch64-apple-darwin
# macOS Intel: x86_64-apple-darwin
# Linux x86_64: x86_64-unknown-linux-gnu
# Linux ARM: aarch64-unknown-linux-gnu
# Windows: x86_64-pc-windows-msvc
TARGET=aarch64-apple-darwin
mkdir -p src-tauri/binaries
cp target/release/mock-zebrad src-tauri/binaries/zebrad-$TARGET
cp target/release/mock-arti src-tauri/binaries/arti-$TARGET
cp target/release/mock-zaino src-tauri/binaries/zaino-$TARGET
cp target/release/zecbox-firewall-helper src-tauri/binaries/zecbox-firewall-helper-$TARGET

# Start the dev server (Vite hot-reload + Rust recompilation on changes)
npx tauri dev
```

The mock binaries respond to health checks and simulate sync progress so you can develop the UI without downloading 300 GB of blockchain data.

### Production builds

Use the build scripts in `scripts/`:

```bash
# macOS
./scripts/build-macos.sh

# Linux
TARGET_TRIPLE=x86_64-unknown-linux-gnu ./scripts/build-linux.sh
```

To build with real upstream binaries instead of mocks, set `REAL_BINARIES=1`:

```bash
REAL_BINARIES=1 ./scripts/build-macos.sh
```

The scripts handle fetching binaries, code signing (if credentials are provided via environment variables), and calling `npx tauri build`. Output goes to `target/release/bundle/`.

For details on code signing and notarization, see the `scripts/` directory and the CI workflows under `.github/workflows/`.

## Project Structure

```
zecbox/
  src/                  # React frontend (components, hooks, styles)
  src-tauri/            # Tauri Rust backend (commands, process management, config)
    src/
      commands/         # IPC command handlers (node, storage, shield, wallet, etc.)
      process/          # Sidecar process spawning and monitoring
      config/           # Config file generation (zebrad.toml, zaino.toml)
      tor/              # Arti lifecycle, firewall rules, DNS-over-Tor
      health/           # Health check polling
      storage/          # Disk space monitoring
      updates/          # Binary update logic
      power/            # Sleep/wake handling (IOKit on macOS)
    binaries/           # Sidecar executables (placed here by build scripts)
  site/                 # Project website (Astro, deployed to zecbox.io)
  scripts/              # Build and release scripts
  mock-zebrad/          # Mock Zcash node for development
  mock-arti/            # Mock Tor client for development
  mock-zaino/           # Mock wallet server for development
  firewall-helper/      # Privileged helper for Shield Mode firewall rules
  tools/
    manifest-signer/    # Signing tool for binary update manifests
```

## Contributing

Contributions are welcome. If you want to report a bug or suggest a feature, please [open an issue](https://github.com/stefanmirrors/zecbox/issues).

For code contributions:

1. Fork the repository and create a branch for your work.
2. Follow the "Building from Source" instructions above to get a dev environment running.
3. Make your changes and test them with `npx tauri dev`.
4. Open a pull request with a clear description of what you changed and why.

The project is organized as a Cargo workspace. The main app lives in `src-tauri/`, and the mock binaries, firewall helper, and manifest signer are separate workspace members that you can build independently.

## License

MIT. See [LICENSE](LICENSE) for the full text.
