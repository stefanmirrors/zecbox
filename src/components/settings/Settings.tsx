import { useNodeStatus } from "../../hooks/useNodeStatus";
import { useStorage } from "../../hooks/useStorage";
import { useShieldMode } from "../../hooks/useShieldMode";
import { useUpdates } from "../../hooks/useUpdates";
import { formatBytes } from "../../lib/format";

export function Settings() {
  const nodeStatus = useNodeStatus();
  const { storageInfo } = useStorage();
  const { status: shieldStatus } = useShieldMode();
  const {
    versions,
    updateStatus,
    availableUpdates,
    checking,
    error,
    checkNow,
    applyOne,
    applyAll,
    dismiss,
    clearError,
  } = useUpdates();

  const isUpdating =
    updateStatus.status === "downloading" ||
    updateStatus.status === "installing" ||
    updateStatus.status === "rollingBack";

  return (
    <div className="max-w-2xl space-y-6">
      <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4">
        <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
          About
        </h3>
        <div className="space-y-3">
          <Row label="Version" value={versions?.app ?? "..."} />
          <Row label="Node Status" value={capitalize(nodeStatus.status)} />
          <Row
            label="Shield Mode"
            value={
              shieldStatus.enabled
                ? "Active (Tor)"
                : capitalize(shieldStatus.status)
            }
          />
          {storageInfo && (
            <Row label="Data Directory" value={storageInfo.dataDir} mono />
          )}
        </div>
      </div>

      <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4">
        <div className="flex items-center justify-between">
          <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
            Binary Versions
          </h3>
          <button
            onClick={checkNow}
            disabled={checking || isUpdating}
            className="text-xs px-3 py-1.5 rounded bg-zec-border text-zec-text hover:bg-zec-border/80 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
          >
            {checking ? "Checking..." : "Check for Updates"}
          </button>
        </div>
        <div className="space-y-3">
          <Row label="zebrad" value={versions?.zebrad ?? "..."} mono />
          <Row label="zaino" value={versions?.zaino ?? "..."} mono />
          <Row label="arti" value={versions?.arti ?? "..."} mono />
        </div>
      </div>

      {availableUpdates.length > 0 && (
        <div className="bg-zec-surface border border-amber-500/30 rounded-lg p-6 space-y-4">
          <div className="flex items-center justify-between">
            <h3 className="text-sm font-medium text-amber-400 uppercase tracking-wider">
              Updates Available
            </h3>
            <div className="flex gap-2">
              <button
                onClick={dismiss}
                disabled={isUpdating}
                className="text-xs px-3 py-1.5 rounded bg-zec-border text-zec-muted hover:bg-zec-border/80 disabled:opacity-50 transition-colors"
              >
                Dismiss
              </button>
              {availableUpdates.length > 1 && (
                <button
                  onClick={applyAll}
                  disabled={isUpdating}
                  className="text-xs px-3 py-1.5 rounded bg-amber-500/20 text-amber-400 hover:bg-amber-500/30 disabled:opacity-50 transition-colors"
                >
                  Update All
                </button>
              )}
            </div>
          </div>
          <div className="space-y-3">
            {availableUpdates.map((u) => (
              <div
                key={u.name}
                className="flex items-center justify-between"
              >
                <div>
                  <span className="text-sm text-zec-text">{u.name}</span>
                  <span className="text-xs text-zec-muted ml-2">
                    {u.currentVersion} → {u.newVersion}
                  </span>
                  <span className="text-xs text-zec-muted ml-2">
                    ({formatBytes(u.sizeBytes)})
                  </span>
                </div>
                <button
                  onClick={() => applyOne(u.name)}
                  disabled={isUpdating}
                  className="text-xs px-3 py-1.5 rounded bg-amber-500/20 text-amber-400 hover:bg-amber-500/30 disabled:opacity-50 transition-colors"
                >
                  Update
                </button>
              </div>
            ))}
          </div>
        </div>
      )}

      {isUpdating && (
        <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
          <div className="flex items-center gap-3">
            <div className="w-4 h-4 border-2 border-zec-muted border-t-zec-text rounded-full animate-spin" />
            <div>
              <p className="text-sm text-zec-text">
                {updateStatus.status === "downloading" &&
                  `Downloading ${updateStatus.binary}...${updateStatus.progress !== undefined ? ` ${updateStatus.progress}%` : ""}`}
                {updateStatus.status === "installing" &&
                  `Installing ${updateStatus.binary}...`}
                {updateStatus.status === "rollingBack" &&
                  `Rolling back ${updateStatus.binary}...`}
              </p>
            </div>
          </div>
        </div>
      )}

      {updateStatus.status === "complete" && (
        <div className="bg-zec-surface border border-green-500/30 rounded-lg p-4">
          <p className="text-sm text-green-400">
            Updates applied successfully.
          </p>
        </div>
      )}

      {error && (
        <div className="bg-zec-surface border border-red-500/30 rounded-lg p-4">
          <div className="flex items-center justify-between">
            <p className="text-sm text-red-400">{error}</p>
            <button
              onClick={clearError}
              className="text-xs text-zec-muted hover:text-zec-text"
            >
              Dismiss
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

function Row({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-zec-muted">{label}</span>
      <span
        className={`text-sm text-zec-text ${mono ? "font-mono" : ""} truncate max-w-72`}
        title={value}
      >
        {value}
      </span>
    </div>
  );
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
