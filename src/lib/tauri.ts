import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, NodeStatusInfo, StorageInfo, Volume } from "./types";

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

export function parseNodeStatus(raw: Record<string, unknown>): NodeStatusInfo {
  const status = (raw.status as string) as NodeStatusInfo["status"];
  return {
    status,
    blockHeight: raw.blockHeight as number | undefined,
    peerCount: raw.peerCount as number | undefined,
    message: raw.message as string | undefined,
  };
}
