import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { NetworkServeStatusInfo } from "../lib/types";
import {
  getNetworkServeStatus,
  enableNetworkServe,
  disableNetworkServe,
  recheckReachability,
} from "../lib/tauri";

export function useNetworkServe() {
  const [status, setStatus] = useState<NetworkServeStatusInfo>({
    enabled: false,
    status: "disabled",
  });
  const [toggling, setToggling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getNetworkServeStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));

    const unlisten = listen<NetworkServeStatusInfo>(
      "network_serve_status_changed",
      (event) => {
        setStatus(event.payload);
        if (
          event.payload.status === "active" ||
          event.payload.status === "disabled"
        ) {
          setToggling(false);
        }
        if (event.payload.status === "error") {
          setToggling(false);
          setError(event.payload.message ?? "Network serving error");
        }
      }
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggle = useCallback(async () => {
    setToggling(true);
    setError(null);
    try {
      if (status.enabled) {
        await disableNetworkServe();
      } else {
        await enableNetworkServe();
      }
    } catch (e) {
      setError(String(e));
      setToggling(false);
      getNetworkServeStatus().then(setStatus).catch(() => {});
    }
  }, [status.enabled]);

  const recheck = useCallback(async () => {
    try {
      await recheckReachability();
    } catch (e) {
      setError(String(e));
    }
  }, []);

  return { status, toggling, error, toggle, recheck, clearError: () => setError(null) };
}
