import { invoke } from "@tauri-apps/api/core";

export async function getNodeStatus(): Promise<string> {
  return invoke("get_node_status");
}

export async function startNode(): Promise<void> {
  return invoke("start_node");
}

export async function stopNode(): Promise<void> {
  return invoke("stop_node");
}
