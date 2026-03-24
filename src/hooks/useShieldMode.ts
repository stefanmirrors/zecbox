import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ShieldStatusInfo } from "../lib/types";
import {
  getShieldStatus,
  enableShieldMode,
  disableShieldMode,
} from "../lib/tauri";

export function useShieldMode() {
  const [status, setStatus] = useState<ShieldStatusInfo>({
    enabled: false,
    status: "disabled",
  });
  const [toggling, setToggling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getShieldStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));

    const unlisten = listen<ShieldStatusInfo>("shield_status_changed", (event) => {
      setStatus(event.payload);
      if (
        event.payload.status === "active" ||
        event.payload.status === "disabled"
      ) {
        setToggling(false);
      }
      if (event.payload.status === "error" || event.payload.status === "interrupted") {
        setToggling(false);
        setError(event.payload.message ?? "Shield Mode error");
      }
    });

    const unlistenInterrupt = listen<string>("shield_interrupted", (event) => {
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
        await disableShieldMode();
      } else {
        await enableShieldMode();
      }
    } catch (e) {
      setError(String(e));
      setToggling(false);
      // Refresh status after error
      getShieldStatus().then(setStatus).catch(() => {});
    }
  }, [status.enabled]);

  return { status, toggling, error, toggle, clearError: () => setError(null) };
}
