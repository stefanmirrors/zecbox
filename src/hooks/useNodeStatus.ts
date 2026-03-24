import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getNodeStatus, parseNodeStatus } from "../lib/tauri";
import type { NodeStatusInfo } from "../lib/types";

const INITIAL_STATUS: NodeStatusInfo = { status: "stopped" };

export function useNodeStatus() {
  const [nodeStatus, setNodeStatus] = useState<NodeStatusInfo>(INITIAL_STATUS);

  useEffect(() => {
    // Fetch initial status
    getNodeStatus()
      .then(setNodeStatus)
      .catch((e) => console.warn("Initial node status fetch failed:", e));

    // Listen for status change events from the Rust backend
    const unlisten = listen<Record<string, unknown>>(
      "node_status_changed",
      (event) => {
        setNodeStatus(parseNodeStatus(event.payload));
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  return nodeStatus;
}
