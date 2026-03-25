import { useStorage } from "../../hooks/useStorage";
import { formatBytes } from "../../lib/format";
import { InfoTip } from "../shared/InfoTip";

export function StorageBar() {
  const { storageInfo, loading, driveConnected } = useStorage();

  if (loading || !storageInfo) return null;

  if (!driveConnected) {
    return (
      <div className="flex items-center gap-2 px-4 py-2.5 rounded-lg border border-red-400/30 text-xs text-red-400/80">
        <span className="w-1.5 h-1.5 rounded-full bg-red-400 animate-pulse" />
        Drive disconnected — reconnect to continue
      </div>
    );
  }

  const usedBytes = storageInfo.totalBytes - storageInfo.availableBytes;
  const usedPercent = Math.round((usedBytes / storageInfo.totalBytes) * 100);

  const barColor = {
    none: "bg-zec-muted/30",
    warning: "bg-yellow-400",
    critical: "bg-red-400",
    paused: "bg-red-400 animate-pulse",
  }[storageInfo.warningLevel];

  const warningText = {
    none: null,
    warning: "Low disk space",
    critical: "Critically low",
    paused: "Node paused — free up space",
  }[storageInfo.warningLevel];

  return (
    <div className="border border-zec-border rounded-xl px-5 py-3 space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-zec-muted flex items-center gap-1.5">
          Storage
          <InfoTip text="Your node stores a full copy of the Zcash blockchain (~300 GB). This lets you verify every transaction independently." />
          {storageInfo.isExternal && (
            <span className="text-[9px] px-1.5 py-0.5 rounded-full bg-zec-border/50 text-zec-muted/60 ml-1">
              External
            </span>
          )}
        </span>
        <span className="text-xs text-zec-muted tabular-nums">
          {formatBytes(storageInfo.availableBytes)} free of {formatBytes(storageInfo.totalBytes)}
        </span>
      </div>
      <div className="h-1 rounded-full bg-zec-border overflow-hidden">
        <div
          className={`h-full rounded-full transition-all ${barColor}`}
          style={{ width: `${usedPercent}%` }}
        />
      </div>
      {warningText && (
        <p className={`text-[11px] ${storageInfo.warningLevel === "warning" ? "text-yellow-400/80" : "text-red-400/80"}`}>
          {warningText}
        </p>
      )}
    </div>
  );
}
