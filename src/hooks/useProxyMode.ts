import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ProxyStatusInfo } from "../lib/types";
import {
  getProxyStatus,
  enableProxyMode,
  disableProxyMode,
} from "../lib/tauri";

export function useProxyMode() {
  const [status, setStatus] = useState<ProxyStatusInfo>({
    enabled: false,
    status: "disabled",
  });
  const [toggling, setToggling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getProxyStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));

    const unlisten = listen<ProxyStatusInfo>("proxy_status_changed", (event) => {
      setStatus(event.payload);
      if (
        event.payload.status === "active" ||
        event.payload.status === "disabled"
      ) {
        setToggling(false);
      }
      if (event.payload.status === "error" || event.payload.status === "interrupted") {
        setToggling(false);
        setError(event.payload.message ?? "Proxy Mode error");
      }
    });

    const unlistenInterrupt = listen<string>("proxy_interrupted", (event) => {
      setError(event.payload);
      setToggling(false);
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenInterrupt.then((fn) => fn());
    };
  }, []);

  const toggle = useCallback(async () => {
    setToggling(true);
    setError(null);
    try {
      if (status.enabled) {
        await disableProxyMode();
      } else {
        await enableProxyMode();
      }
    } catch (e) {
      setError(String(e));
      setToggling(false);
      getProxyStatus().then(setStatus).catch(() => {});
    }
  }, [status.enabled]);

  return {
    status,
    toggling,
    error,
    toggle,
    clearError: () => setError(null),
  };
}
