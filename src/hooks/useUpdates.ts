import { useCallback, useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { BinaryUpdateInfo, UpdateStatusInfo, VersionInfo } from "../lib/types";
import {
  getVersions,
  getUpdateStatus,
  checkForUpdates,
  applyUpdate,
  applyAllUpdates,
  dismissUpdates,
} from "../lib/tauri";

export function useUpdates() {
  const [versions, setVersions] = useState<VersionInfo | null>(null);
  const [updateStatus, setUpdateStatus] = useState<UpdateStatusInfo>({
    status: "idle",
  });
  const [availableUpdates, setAvailableUpdates] = useState<BinaryUpdateInfo[]>(
    []
  );
  const [checking, setChecking] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    getVersions()
      .then(setVersions)
      .catch((e) => setError(String(e)));

    getUpdateStatus()
      .then(setUpdateStatus)
      .catch(() => {});

    const unlistenStatus = listen<UpdateStatusInfo>(
      "update_status_changed",
      (event) => {
        setUpdateStatus(event.payload);
        if (event.payload.status === "idle" || event.payload.status === "complete") {
          setChecking(false);
          getVersions().then(setVersions).catch(() => {});
        }
        if (event.payload.status === "error") {
          setChecking(false);
          setError(event.payload.message ?? "Update error");
        }
        if (event.payload.status === "updateAvailable") {
          setChecking(false);
        }
      }
    );

    const unlistenAvailable = listen<BinaryUpdateInfo[]>(
      "update_available",
      (event) => {
        setAvailableUpdates(event.payload);
      }
    );

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenAvailable.then((fn) => fn());
    };
  }, []);

  const checkNow = useCallback(async () => {
    setChecking(true);
    setError(null);
    try {
      const updates = await checkForUpdates();
      setAvailableUpdates(updates);
    } catch (e) {
      setError(String(e));
      setChecking(false);
    }
  }, []);

  const applyOne = useCallback(async (name: string) => {
    setError(null);
    try {
      await applyUpdate(name);
      setAvailableUpdates((prev) => prev.filter((u) => u.name !== name));
      getVersions().then(setVersions).catch(() => {});
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const applyAll = useCallback(async () => {
    setError(null);
    try {
      await applyAllUpdates();
      setAvailableUpdates([]);
      getVersions().then(setVersions).catch(() => {});
    } catch (e) {
      setError(String(e));
    }
  }, []);

  const dismiss = useCallback(async () => {
    setError(null);
    try {
      await dismissUpdates();
      setAvailableUpdates([]);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  return {
    versions,
    updateStatus,
    availableUpdates,
    checking,
    error,
    checkNow,
    applyOne,
    applyAll,
    dismiss,
    clearError: () => setError(null),
  };
}
