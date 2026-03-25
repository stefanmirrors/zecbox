import { useStorage } from "../../hooks/useStorage";
import { formatBytes } from "../../lib/format";
import { InfoTip } from "../shared/InfoTip";

export function StoragePanel() {
  const { storageInfo, loading, driveConnected } = useStorage();

  if (loading) {
    return (
      <div className="border border-zec-border rounded-xl p-5">
        <h3 className="text-xs font-medium text-zec-muted">Storage</h3>
        <p className="text-sm text-zec-muted/60 mt-4">Loading...</p>
      </div>
    );
  }

  if (!driveConnected) {
    return (
      <div className="border border-red-400/30 rounded-xl p-5 space-y-2">
        <h3 className="text-xs font-medium text-red-400/80">Drive Disconnected</h3>
        <p className="text-sm text-zec-muted">
          Reconnect your external drive to continue.
        </p>
      </div>
    );
  }

  if (!storageInfo) {
    return (
      <div className="border border-zec-border rounded-xl p-5">
        <h3 className="text-xs font-medium text-zec-muted">Storage</h3>
        <p className="text-sm text-zec-muted/60 mt-4">No storage info.</p>
      </div>
    );
  }

  const usedBytes = storageInfo.totalBytes - storageInfo.availableBytes;
  const usedPercent = Math.round((usedBytes / storageInfo.totalBytes) * 100);

  const barColor = {
    none: "bg-zec-muted/40",
    warning: "bg-yellow-400",
    critical: "bg-red-400",
    paused: "bg-red-400 animate-pulse",
  }[storageInfo.warningLevel];

  return (
    <div className="border border-zec-border rounded-xl p-5 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium text-zec-muted flex items-center gap-1.5">
          Storage <InfoTip text="Your node stores a full copy of the Zcash blockchain (~300 GB). This lets you verify every transaction independently without trusting anyone." />
        </h3>
        {storageInfo.isExternal && (
          <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-border/50 text-zec-muted">
            External
          </span>
        )}
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between text-xs text-zec-muted">
          <span>{formatBytes(storageInfo.availableBytes)} free</span>
          <span>{formatBytes(storageInfo.totalBytes)}</span>
        </div>
        <div className="h-1 rounded-full bg-zec-border overflow-hidden">
          <div
            className={`h-full rounded-full transition-all ${barColor}`}
            style={{ width: `${usedPercent}%` }}
          />
        </div>
      </div>

      {storageInfo.warningLevel === "warning" && (
        <p className="text-[11px] text-yellow-400/80">Low disk space</p>
      )}
      {storageInfo.warningLevel === "critical" && (
        <p className="text-[11px] text-red-400/80">Critically low — node may pause</p>
      )}
      {storageInfo.warningLevel === "paused" && (
        <p className="text-[11px] text-red-400/80 font-medium">Node paused — free up space</p>
      )}
    </div>
  );
}
