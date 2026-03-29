import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { StealthStatusInfo } from "../lib/types";
import {
  getStealthStatus,
  enableStealthMode,
  disableStealthMode,
  isFirewallHelperInstalled,
  installFirewallHelper,
  isStealthSupported,
} from "../lib/tauri";

export function useStealthMode() {
  const [status, setStatus] = useState<StealthStatusInfo>({
    enabled: false,
    status: "disabled",
  });
  const [toggling, setToggling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [helperInstalled, setHelperInstalled] = useState<boolean | null>(null);
  const [installing, setInstalling] = useState(false);
  const [platformSupported, setPlatformSupported] = useState<boolean | null>(null);

  useEffect(() => {
    isStealthSupported()
      .then(setPlatformSupported)
      .catch(() => setPlatformSupported(false));

    getStealthStatus()
      .then(setStatus)
      .catch((e) => setError(String(e)));

    isFirewallHelperInstalled()
      .then(setHelperInstalled)
      .catch(() => setHelperInstalled(false));

    const unlisten = listen<StealthStatusInfo>("stealth_status_changed", (event) => {
      setStatus(event.payload);
      if (
        event.payload.status === "active" ||
        event.payload.status === "disabled"
      ) {
        setToggling(false);
      }
      if (event.payload.status === "error" || event.payload.status === "interrupted") {
        setToggling(false);
        setError(event.payload.message ?? "Stealth Mode error");
      }
    });

    const unlistenInterrupt = listen<string>("stealth_interrupted", (event) => {
      setError(event.payload);
      setToggling(false);
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenInterrupt.then((fn) => fn());
    };
  }, []);

  const installHelper = useCallback(async () => {
    setInstalling(true);
    setError(null);
    try {
      await installFirewallHelper();
      setHelperInstalled(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setInstalling(false);
    }
  }, []);

  const toggle = useCallback(async () => {
    setToggling(true);
    setError(null);
    try {
      if (status.enabled) {
        await disableStealthMode();
      } else {
        await enableStealthMode();
      }
    } catch (e) {
      const errMsg = String(e);
      if (errMsg.includes("Firewall helper not installed")) {
        setHelperInstalled(false);
      }
      setError(errMsg);
      setToggling(false);
      getStealthStatus().then(setStatus).catch(() => {});
    }
  }, [status.enabled]);

  return {
    status,
    toggling,
    error,
    toggle,
    clearError: () => setError(null),
    helperInstalled,
    installing,
    installHelper,
    platformSupported,
  };
}
