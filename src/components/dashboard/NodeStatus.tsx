import { useState } from "react";
import { useNodeStatus } from "../../hooks/useNodeStatus";
import { startNode, stopNode } from "../../lib/tauri";
import { InfoTip } from "../shared/InfoTip";

export function NodeStatus() {
  const ns = useNodeStatus();
  const isRunning = ns.status === "running";
  const isSyncing = isRunning && ns.syncPercentage != null && ns.syncPercentage < 99.9;
  const isSynced = isRunning && ns.syncPercentage != null && ns.syncPercentage >= 99.9;
  const isBusy = ns.status === "starting" || ns.status === "stopping";
  const [toggling, setToggling] = useState(false);
  const [toggleError, setToggleError] = useState<string | null>(null);

  const handleToggle = async () => {
    setToggleError(null);
    setToggling(true);
    try {
      if (isRunning) {
        await stopNode();
      } else {
        await startNode();
      }
    } catch (e) {
      setToggleError(
        typeof e === "string" ? e : "Failed to toggle node. Please try again."
      );
    } finally {
      setToggling(false);
    }
  };

  const statusDot = {
    stopped: "bg-zec-muted/50",
    starting: "bg-zec-yellow animate-pulse",
    running: "bg-emerald-400",
    stopping: "bg-zec-yellow animate-pulse",
    error: "bg-red-400",
  }[ns.status];

  return (
    <div className="border border-zec-border rounded-xl p-6 space-y-5">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2.5">
          <span className={`w-2 h-2 rounded-full ${statusDot}`} />
          <span className="text-sm font-medium text-zec-text capitalize">
            {ns.status === "error" ? "Error" : ns.status}
          </span>
        </div>
        <button
          onClick={handleToggle}
          disabled={isBusy || toggling}
          className={`px-4 py-1.5 rounded-lg text-sm font-medium transition-colors ${
            isBusy || toggling
              ? "bg-zec-border/50 text-zec-muted cursor-not-allowed"
              : isRunning
                ? "border border-zec-border text-zec-muted hover:text-zec-text hover:border-red-400/50"
                : "bg-zec-yellow text-zec-dark hover:brightness-110"
          }`}
        >
          {isBusy || toggling
            ? ns.status === "starting" || (!isRunning && toggling)
              ? "Starting..."
              : "Stopping..."
            : isRunning
              ? "Stop"
              : "Start Node"}
        </button>
      </div>

      {/* Sync progress */}
      {isSyncing && (
        <div className="space-y-3">
          <div className="flex items-baseline justify-between">
            <span className="text-3xl font-bold text-zec-text tabular-nums">
              {ns.syncPercentage?.toFixed(1)}%
            </span>
            <span className="text-xs text-zec-muted">
              {ns.blockHeight?.toLocaleString()} / {ns.estimatedHeight?.toLocaleString()}
            </span>
          </div>
          <div className="h-1.5 rounded-full bg-zec-border overflow-hidden">
            <div
              className="h-full rounded-full bg-zec-yellow transition-all duration-500"
              style={{ width: `${ns.syncPercentage ?? 0}%` }}
            />
          </div>
          <p className="text-xs text-zec-muted">
            Downloading and verifying every Zcash transaction since 2016
          </p>
        </div>
      )}

      {/* Synced state */}
      {isSynced && (
        <div className="space-y-4">
          <div className="flex items-center gap-8">
            <div>
              <p className="text-xs text-zec-muted mb-1 flex items-center gap-1.5">
                Block Height <InfoTip text="The number of blocks your node has verified. Each block contains Zcash transactions. A new block is added roughly every 75 seconds." />
              </p>
              <p className="text-2xl font-bold text-zec-text tabular-nums">
                {ns.blockHeight?.toLocaleString() ?? "..."}
              </p>
            </div>
            <div>
              <p className="text-xs text-zec-muted mb-1 flex items-center gap-1.5">
                Peers <InfoTip text="Other Zcash nodes your node is connected to. More peers means a healthier connection to the network." />
              </p>
              <p className="text-2xl font-bold text-zec-text tabular-nums">
                {ns.peerCount ?? 0}
              </p>
            </div>
          </div>
          <p className="text-xs text-zec-muted">
            Your node is fully synced — verifying new blocks as they arrive
          </p>
        </div>
      )}

      {/* Running but no sync data yet */}
      {isRunning && ns.syncPercentage == null && (
        <div className="flex items-center gap-8">
          <div>
            <p className="text-xs text-zec-muted mb-1">Block Height</p>
            <p className="text-2xl font-bold text-zec-text tabular-nums">
              {ns.blockHeight?.toLocaleString() ?? "..."}
            </p>
          </div>
          <div>
            <p className="text-xs text-zec-muted mb-1">Peers</p>
            <p className="text-2xl font-bold text-zec-text tabular-nums">
              {ns.peerCount ?? 0}
            </p>
          </div>
        </div>
      )}

      {/* Best block hash */}
      {isRunning && ns.bestBlockHash && (
        <div>
          <p className="text-xs text-zec-muted mb-1 flex items-center gap-1.5">
            Latest Block <InfoTip text="The unique identifier (hash) of the most recent block your node has verified. This changes every time a new block arrives." />
          </p>
          <p className="text-xs font-mono text-zec-muted/60 truncate">
            {ns.bestBlockHash}
          </p>
        </div>
      )}

      {/* Error */}
      {ns.status === "error" && ns.message && (
        <p className="text-sm text-red-400/80">{ns.message}</p>
      )}
      {toggleError && (
        <p className="text-sm text-red-400/80">{toggleError}</p>
      )}
    </div>
  );
}
