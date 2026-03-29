import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { PrivacyMode } from "../lib/types";
import { getPrivacyMode, setPrivacyMode } from "../lib/tauri";

export function usePrivacyMode() {
  const [mode, setMode] = useState<PrivacyMode>("standard");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getPrivacyMode()
      .then((m) => {
        setMode(m);
        setLoading(false);
      })
      .catch((e) => {
        setError(String(e));
        setLoading(false);
      });

    const unlisten = listen<PrivacyMode>("privacy_mode_changed", (event) => {
      setMode(event.payload);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const changeMode = useCallback(async (newMode: PrivacyMode) => {
    setError(null);
    try {
      await setPrivacyMode(newMode);
      setMode(newMode);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  return {
    mode,
    loading,
    error,
    changeMode,
    clearError: () => setError(null),
  };
}
