import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, BinaryUpdateInfo, NodeStatusInfo, ShieldStatusInfo, StorageInfo, UpdateStatusInfo, VersionInfo, Volume, WalletStatusInfo } from "./types";

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

export async function completeOnboarding(path: string): Promise<void> {
  return invoke("complete_onboarding", { path });
}

export async function getLogs(): Promise<string[]> {
  return invoke<string[]>("get_logs");
}

export async function getShieldStatus(): Promise<ShieldStatusInfo> {
  return invoke<ShieldStatusInfo>("get_shield_status");
}

export async function enableShieldMode(): Promise<void> {
  return invoke("enable_shield_mode");
}

export async function disableShieldMode(): Promise<void> {
  return invoke("disable_shield_mode");
}

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

export async function getAutoStartEnabled(): Promise<boolean> {
  return invoke<boolean>("get_auto_start_enabled");
}

export async function setAutoStart(enabled: boolean): Promise<void> {
  return invoke("set_auto_start", { enabled });
}

export async function rebuildDatabase(): Promise<void> {
  return invoke("rebuild_database");
}

export function parseNodeStatus(raw: Record<string, unknown>): NodeStatusInfo {
  const status = (raw.status as string) as NodeStatusInfo["status"];
  return {
    status,
    blockHeight: raw.blockHeight as number | undefined,
    peerCount: raw.peerCount as number | undefined,
    message: raw.message as string | undefined,
  };
}
