import { useState, useMemo } from "react";
import { useNodeStatus } from "../../hooks/useNodeStatus";
import { startNode, stopNode } from "../../lib/tauri";
import { InfoTip } from "../shared/InfoTip";

export function NodeStatus() {
  const ns = useNodeStatus();
  const isRunning = ns.status === "running";
  const isSyncing = isRunning && ns.syncPercentage != null && ns.syncPercentage < 99.9;
  const isSynced = isRunning && ns.syncPercentage != null && ns.syncPercentage >= 99.9;
  const isStarting = ns.status === "starting";
  const isBusy = isStarting || ns.status === "stopping";
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

      {/* Unified progress: startup steps → sync */}
      {(isStarting || isSyncing) && (
        <UnifiedProgress
          isStarting={isStarting}
          isSyncing={isSyncing}
          message={ns.message}
          startupProgress={ns.progress}
          syncPercentage={ns.syncPercentage}
          blockHeight={ns.blockHeight}
          estimatedHeight={ns.estimatedHeight}
        />
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

const STARTUP_STEPS = [
  { key: "prepare", label: "Preparing node" },
  { key: "storage", label: "Setting up storage" },
  { key: "network", label: "Connecting to network" },
  { key: "peers", label: "Finding peers" },
  { key: "verify", label: "Verifying blockchain" },
];

function messageToStep(message?: string): number {
  if (!message) return 0;
  if (message.includes("Preparing")) return 0;
  if (message.includes("storage")) return 1;
  if (message.includes("Connecting")) return 2;
  if (message.includes("Finding") || message.includes("Connected to")) return 3;
  if (message.includes("verify") || message.includes("Verifying") || message.includes("Downloading") || message.includes("Almost")) return 4;
  return 0;
}

function UnifiedProgress({
  isStarting,
  isSyncing,
  message,
  startupProgress,
  syncPercentage,
  blockHeight,
  estimatedHeight,
}: {
  isStarting: boolean;
  isSyncing: boolean;
  message?: string;
  startupProgress?: number;
  syncPercentage?: number;
  blockHeight?: number;
  estimatedHeight?: number;
}) {
  const rawStep = useMemo(() => messageToStep(message), [message]);
  const [highestStep, setHighestStep] = useState(0);
  if (rawStep > highestStep) setHighestStep(rawStep);
  const currentStep = highestStep;

  // Unified 0-100%: startup = 0-5%, sync = 5-100%
  let totalProgress: number;
  let statusText: string;

  if (isStarting) {
    totalProgress = (currentStep / STARTUP_STEPS.length) * 5;
    statusText = message || "Initializing node...";
  } else if (isSyncing && syncPercentage != null) {
    totalProgress = 5 + (syncPercentage * 0.95);
    statusText = `Downloading and verifying every Zcash transaction since 2016`;
  } else {
    totalProgress = 5;
    statusText = "Starting sync...";
  }

  return (
    <div className="space-y-4">
      {/* Main progress */}
      <div className="space-y-2">
        <div className="flex items-baseline justify-between">
          <span className="text-3xl font-bold text-zec-text tabular-nums">
            {totalProgress < 5 ? `${totalProgress.toFixed(0)}%` : `${totalProgress.toFixed(1)}%`}
          </span>
          {isSyncing && blockHeight != null && estimatedHeight != null && (
            <span className="text-xs text-zec-muted tabular-nums">
              {blockHeight.toLocaleString()} / {estimatedHeight.toLocaleString()}
            </span>
          )}
        </div>
        <div className="h-1.5 rounded-full bg-zec-border overflow-hidden">
          <div
            className="h-full rounded-full bg-zec-yellow transition-all duration-700"
            style={{ width: `${Math.max(totalProgress, 0.5)}%` }}
          />
        </div>
        <p className="text-xs text-zec-muted">{statusText}</p>
      </div>

      {/* Step checklist — only during startup */}
      {isStarting && (
        <div className="space-y-1.5">
          {STARTUP_STEPS.map((step, i) => {
            const isDone = i < currentStep;
            const isActive = i === currentStep;
            return (
              <div key={step.key} className="flex items-center gap-2.5">
                {isDone ? (
                  <div className="w-4 h-4 rounded-full bg-emerald-400/20 flex items-center justify-center shrink-0">
                    <svg className="w-2.5 h-2.5 text-emerald-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                    </svg>
                  </div>
                ) : isActive ? (
                  <div className="w-4 h-4 flex items-center justify-center shrink-0">
                    <div className="w-3.5 h-3.5 border-2 border-zec-yellow border-t-transparent rounded-full animate-spin" />
                  </div>
                ) : (
                  <div className="w-4 h-4 rounded-full border border-zec-border shrink-0" />
                )}
                <span className={`text-xs ${isDone ? "text-zec-muted/40" : isActive ? "text-zec-text" : "text-zec-muted/25"}`}>
                  {step.label}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
