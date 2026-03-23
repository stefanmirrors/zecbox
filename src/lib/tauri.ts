import { invoke } from "@tauri-apps/api/core";
import type { NodeStatusInfo } from "./types";

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

export function parseNodeStatus(raw: Record<string, unknown>): NodeStatusInfo {
  const status = (raw.status as string) as NodeStatusInfo["status"];
  return {
    status,
    blockHeight: raw.blockHeight as number | undefined,
    peerCount: raw.peerCount as number | undefined,
    message: raw.message as string | undefined,
  };
}
