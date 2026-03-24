import { useStorage } from "../../hooks/useStorage";
import { formatBytes } from "../../lib/format";

export function StoragePanel() {
  const { storageInfo, loading, driveConnected } = useStorage();

  if (loading) {
    return (
      <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
        <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
          Storage
        </h3>
        <p className="text-sm text-zec-muted mt-4">Loading...</p>
      </div>
    );
  }

  if (!driveConnected) {
    return (
      <div className="bg-zec-surface border border-red-500/50 rounded-lg p-6 space-y-3">
        <h3 className="text-sm font-medium text-red-400 uppercase tracking-wider">
          Storage - Drive Disconnected
        </h3>
        <p className="text-sm text-zec-muted">
          Reconnect your external drive to continue.
        </p>
      </div>
    );
  }

  if (!storageInfo) {
    return (
      <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
        <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
          Storage
        </h3>
        <p className="text-sm text-zec-muted mt-4">No storage info available.</p>
      </div>
    );
  }

  const usedBytes = storageInfo.totalBytes - storageInfo.availableBytes;
  const usedPercent = Math.round((usedBytes / storageInfo.totalBytes) * 100);

  const warningColors = {
    none: "bg-zec-muted",
    warning: "bg-yellow-500",
    critical: "bg-red-500",
    paused: "bg-red-500 animate-pulse",
  };

  const barColor = warningColors[storageInfo.warningLevel];

  return (
    <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4">
      <div className="flex items-center justify-between">
        <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
          Storage
        </h3>
        {storageInfo.isExternal && (
          <span className="text-xs px-2 py-0.5 rounded bg-zec-border text-zec-muted">
            External
          </span>
        )}
      </div>

      <div className="space-y-2">
        <div className="flex items-center justify-between text-sm">
          <span className="text-zec-muted">
            {formatBytes(storageInfo.availableBytes)} free
          </span>
          <span className="text-zec-muted">
            {formatBytes(storageInfo.totalBytes)} total
          </span>
        </div>
        <div className="h-2 rounded-full bg-zec-border overflow-hidden">
          <div
            className={`h-full rounded-full transition-all ${barColor}`}
            style={{ width: `${usedPercent}%` }}
          />
        </div>
      </div>

      {storageInfo.warningLevel === "warning" && (
        <p className="text-xs text-yellow-400">
          Low disk space. Consider freeing up storage.
        </p>
      )}
      {storageInfo.warningLevel === "critical" && (
        <p className="text-xs text-red-400">
          Critically low disk space. Node may be paused soon.
        </p>
      )}
      {storageInfo.warningLevel === "paused" && (
        <p className="text-xs text-red-400 font-medium">
          Node paused due to insufficient disk space.
        </p>
      )}

      <div className="flex items-center justify-between">
        <span className="text-sm text-zec-muted">Data Directory</span>
        <span
          className="text-sm text-zec-text font-mono truncate max-w-48"
          title={storageInfo.dataDir}
        >
          {storageInfo.dataDir}
        </span>
      </div>
    </div>
  );
}
