import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { WalletStatusInfo } from "../lib/types";
import {
  getWalletStatus,
  enableWalletServer,
  disableWalletServer,
} from "../lib/tauri";

export function useWalletServer() {
  const [status, setStatus] = useState<WalletStatusInfo>({
    enabled: false,
    status: "stopped",
  });
  const [toggling, setToggling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getWalletStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));

    const unlisten = listen<WalletStatusInfo>("wallet_status_changed", (event) => {
      setStatus(event.payload);
      if (
        event.payload.status === "running" ||
        event.payload.status === "stopped"
      ) {
        setToggling(false);
      }
      if (event.payload.status === "error") {
        setToggling(false);
        setError(event.payload.message ?? "Wallet server error");
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const toggle = useCallback(async () => {
    setToggling(true);
    setError(null);
    try {
      if (status.enabled) {
        await disableWalletServer();
      } else {
        await enableWalletServer();
      }
    } catch (e) {
      setError(String(e));
      setToggling(false);
      getWalletStatus().then(setStatus).catch(() => {});
    }
  }, [status.enabled]);

  return { status, toggling, error, toggle, clearError: () => setError(null) };
}
