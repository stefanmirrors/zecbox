import { invoke } from "@tauri-apps/api/core";
import type {
  AppConfig, BinaryUpdateInfo, NetworkServeStatusInfo, NodeStats,
  NodeStatusInfo, PrivacyMode, ProxySetupConfig, ProxyStatusInfo,
  StealthStatusInfo, StorageInfo, UpdateStatusInfo, VersionInfo,
  Volume, VpsProvider, WalletStatusInfo,
} from "./types";

export async function getNodeStatus(): Promise<NodeStatusInfo> {
  const raw = await invoke<Record<string, unknown>>("get_node_status");
  return parseNodeStatus(raw);
}

export async function startNode(): Promise<void> {
  return invoke("start_node");
}

export async function stopNode(): Promise<void> {
  return invoke("stop_node");
}

export async function getVolumes(): Promise<Volume[]> {
  return invoke<Volume[]>("get_volumes");
}

export async function getStorageInfo(): Promise<StorageInfo> {
  return invoke<StorageInfo>("get_storage_info");
}

export async function setDataDir(path: string): Promise<void> {
  return invoke("set_data_dir", { path });
}

export async function getAppConfig(): Promise<AppConfig> {
  return invoke<AppConfig>("get_app_config");
}

export async function completeOnboarding(path: string, privacyMode: PrivacyMode): Promise<void> {
  return invoke("complete_onboarding", { path, privacyMode });
}

export async function resetOnboarding(): Promise<void> {
  return invoke("reset_onboarding");
}

export async function getLogs(): Promise<string[]> {
  return invoke<string[]>("get_logs");
}

// --- Stealth Mode (Tor) ---

export async function getStealthStatus(): Promise<StealthStatusInfo> {
  return invoke<StealthStatusInfo>("get_stealth_status");
}

export async function enableStealthMode(): Promise<void> {
  return invoke("enable_stealth_mode");
}

export async function disableStealthMode(): Promise<void> {
  return invoke("disable_stealth_mode");
}

export async function isFirewallHelperInstalled(): Promise<boolean> {
  return invoke<boolean>("is_firewall_helper_installed");
}

export async function installFirewallHelper(): Promise<void> {
  return invoke("install_firewall_helper");
}

export async function isStealthSupported(): Promise<boolean> {
  return invoke<boolean>("is_stealth_supported");
}

// --- Proxy Mode (VPS relay) ---

export async function getProxyStatus(): Promise<ProxyStatusInfo> {
  return invoke<ProxyStatusInfo>("get_proxy_status");
}

export async function startProxySetup(vpsIp: string, vpsWgPort?: number): Promise<void> {
  return invoke("start_proxy_setup", { vpsIp, vpsWgPort });
}

export async function getProxySetupConfig(): Promise<ProxySetupConfig> {
  return invoke<ProxySetupConfig>("get_proxy_setup_config");
}

export async function enableProxyMode(): Promise<void> {
  return invoke("enable_proxy_mode");
}

export async function disableProxyMode(): Promise<void> {
  return invoke("disable_proxy_mode");
}

export async function verifyProxyConnection(): Promise<boolean> {
  return invoke<boolean>("verify_proxy_connection");
}

export async function resetProxyConfig(): Promise<void> {
  return invoke("reset_proxy_config");
}

export async function getVpsProviders(tier: string): Promise<VpsProvider[]> {
  return invoke<VpsProvider[]>("get_vps_providers", { tier });
}

// --- Privacy Mode ---

export async function getPrivacyMode(): Promise<PrivacyMode> {
  return invoke<PrivacyMode>("get_privacy_mode");
}

export async function setPrivacyMode(mode: PrivacyMode): Promise<void> {
  return invoke("set_privacy_mode", { mode });
}

// --- Wallet Server ---

export async function getWalletStatus(): Promise<WalletStatusInfo> {
  return invoke<WalletStatusInfo>("get_wallet_status");
}

export async function enableWalletServer(): Promise<void> {
  return invoke("enable_wallet_server");
}

export async function disableWalletServer(): Promise<void> {
  return invoke("disable_wallet_server");
}

export async function getWalletQr(): Promise<string> {
  return invoke<string>("get_wallet_qr");
}

// --- Updates ---

export async function getVersions(): Promise<VersionInfo> {
  return invoke<VersionInfo>("get_versions");
}

export async function getUpdateStatus(): Promise<UpdateStatusInfo> {
  return invoke<UpdateStatusInfo>("get_update_status");
}

export async function checkForUpdates(): Promise<BinaryUpdateInfo[]> {
  return invoke<BinaryUpdateInfo[]>("check_for_updates");
}

export async function applyUpdate(name: string): Promise<void> {
  return invoke("apply_update", { name });
}

export async function applyAllUpdates(): Promise<void> {
  return invoke("apply_all_updates");
}

export async function dismissUpdates(): Promise<void> {
  return invoke("dismiss_updates");
}

export async function checkAppUpdate(): Promise<boolean> {
  return invoke<boolean>("check_app_update");
}

// --- Settings ---

export async function getAutoStartEnabled(): Promise<boolean> {
  return invoke<boolean>("get_auto_start_enabled");
}

export async function setAutoStart(enabled: boolean): Promise<void> {
  return invoke("set_auto_start", { enabled });
}

export async function rebuildDatabase(): Promise<void> {
  return invoke("rebuild_database");
}

// --- Network Serve ---

export async function getNetworkServeStatus(): Promise<NetworkServeStatusInfo> {
  return invoke<NetworkServeStatusInfo>("get_network_serve_status");
}

export async function enableNetworkServe(): Promise<void> {
  return invoke("enable_network_serve");
}

export async function disableNetworkServe(): Promise<void> {
  return invoke("disable_network_serve");
}

export async function recheckReachability(): Promise<void> {
  return invoke("recheck_reachability");
}

// --- Node Stats ---

export async function getNodeStats(): Promise<NodeStats> {
  return invoke<NodeStats>("get_node_stats");
}

export function parseNodeStatus(raw: Record<string, unknown>): NodeStatusInfo {
  const status = (raw.status as string) as NodeStatusInfo["status"];
  return {
    status,
    blockHeight: raw.blockHeight as number | undefined,
    peerCount: raw.peerCount as number | undefined,
    estimatedHeight: raw.estimatedHeight as number | undefined,
    bestBlockHash: raw.bestBlockHash as string | undefined,
    syncPercentage: raw.syncPercentage as number | undefined,
    chain: raw.chain as string | undefined,
    message: raw.message as string | undefined,
    progress: raw.progress as number | undefined,
  };
}
