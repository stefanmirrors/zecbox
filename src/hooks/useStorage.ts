import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getVolumes, getStorageInfo } from "../lib/tauri";
import type { Volume, StorageInfo } from "../lib/types";

export function useStorage() {
  const [volumes, setVolumes] = useState<Volume[]>([]);
  const [storageInfo, setStorageInfo] = useState<StorageInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [driveConnected, setDriveConnected] = useState(true);

  useEffect(() => {
    getVolumes()
      .then(setVolumes)
      .catch((e) => console.warn("Failed to load volumes:", e));

    getStorageInfo()
      .then((info) => {
        setStorageInfo(info);
        setLoading(false);
      })
      .catch((e) => {
        console.warn("Failed to load storage info:", e);
        setLoading(false);
      });

    const unlistenInfo = listen<StorageInfo>("storage_info_updated", (e) =>
      setStorageInfo(e.payload)
    );
    const unlistenDisconnect = listen("storage_drive_disconnected", () =>
      setDriveConnected(false)
    );
    const unlistenReconnect = listen("storage_drive_reconnected", () =>
      setDriveConnected(true)
    );

    return () => {
      unlistenInfo.then((fn) => fn());
      unlistenDisconnect.then((fn) => fn());
      unlistenReconnect.then((fn) => fn());
    };
  }, []);

  return { volumes, storageInfo, loading, driveConnected };
}
