# ZecBox (Under Development)

**One click. Full node.**

ZecBox turns your computer into a Zcash full node. Download, install, click start. No terminal. No Docker. No configuration files.

---

## What It Does

- Runs a [Zebra](https://github.com/ZcashFoundation/zebra) full node (zebrad) behind a clean dashboard
- **Shield Mode** — route all node traffic through Tor with one toggle. If Tor drops, the node stops. It will never silently fall back to clearnet.
- **Wallet Server** — serve your own light wallet backend (Zaino) so ZODL, Ywallet, or any compatible wallet can sync from _your_ node instead of someone else's
- Monitors storage, network peers, sync progress, and node health in real time
- Survives reboots, sleep, crashes, and power loss without losing chain data
- Lives in your system tray — close the window, the node keeps running

## What It Doesn't Do

- It doesn't hold your keys. ZecBox is not a wallet.
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
2. Open the `.dmg`, drag ZecBox to Applications
3. Launch ZecBox
4. Pick your storage location (or accept the default)
5. Click **Start**

Your node is running. That's it.

---

## Verify Your Download

Every release includes a `SHA256SUMS` file. To verify:

```bash
shasum -a 256 -c SHA256SUMS
```
