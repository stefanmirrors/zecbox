import { useState, useMemo, useEffect, useRef } from "react";
import { useNodeStatus } from "../../hooks/useNodeStatus";
import { useShieldMode } from "../../hooks/useShieldMode";
import { startNode, stopNode } from "../../lib/tauri";
import { InfoTip } from "../shared/InfoTip";

export function NodeStatus() {
  const ns = useNodeStatus();
  const isRunning = ns.status === "running";
  const isSyncing = isRunning && ns.syncPercentage != null && ns.syncPercentage < 99.9;
  const isSynced = isRunning && ns.syncPercentage != null && ns.syncPercentage >= 99.9;
  const isStarting = ns.status === "starting";
  const isBusy = isStarting || ns.status === "stopping";
  const shield = useShieldMode();
  const [toggling, setToggling] = useState(false);
  const [toggleError, setToggleError] = useState<string | null>(null);
  const [showCongrats, setShowCongrats] = useState(false);
  const wasSyncing = useRef(false);

  useEffect(() => {
    if (isSyncing || isStarting) wasSyncing.current = true;
    if (isSynced && wasSyncing.current) {
      wasSyncing.current = false;
      const shown = sessionStorage.getItem("zecbox_congrats_shown_session");
      if (!shown) {
        setShowCongrats(true);
        localStorage.setItem("zecbox_congrats_shown", "1");
        sessionStorage.setItem("zecbox_congrats_shown_session", "1");
      }
    }
  }, [isSyncing, isSynced, isStarting]);

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
        <div className="flex items-center gap-4">
          {/* Feature toggles */}
          <MiniToggle
            label="Shield"
            tip="Route all node traffic through Tor and accept connections via .onion hidden service."
            enabled={shield.status.enabled}
            loading={shield.toggling || shield.status.status === "bootstrapping"}
            onToggle={shield.toggle}
          />
          <div className="w-px h-5 bg-zec-border" />
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
                : shield.status.enabled ? "Start Shielded Node" : "Start Node"}
          </button>
        </div>
      </div>

      {/* Unified progress: startup steps → sync */}
      {(isStarting || isSyncing) && (
        <UnifiedProgress
          isStarting={isStarting}
          isSyncing={isSyncing}
          message={ns.message}
          progress={ns.progress}
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

      {showCongrats && <CongratsOverlay onClose={() => setShowCongrats(false)} />}
    </div>
  );
}

function CongratsOverlay({ onClose }: { onClose: () => void }) {
  return (
    <div
      className="fixed inset-0 z-[9999] flex items-center justify-center bg-black/70 backdrop-blur-sm"
      onClick={onClose}
    >
      <div
        className="max-w-md mx-4 bg-zec-dark border border-zec-yellow/30 rounded-2xl p-8 text-center space-y-5 shadow-2xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="text-5xl">🛡️</div>
        <h2 className="text-2xl font-bold text-zec-text">Your node is fully synced</h2>
        <p className="text-sm text-zec-muted leading-relaxed">
          You're now running a full Zcash node. Every transaction on the network
          is being verified by your computer — no trust required. You're strengthening
          the privacy and decentralization of the entire Zcash network.
        </p>
        <p className="text-xs text-zec-muted/60 leading-relaxed">
          Every full node makes the network stronger and more private for everyone.
        </p>
        <button
          onClick={onClose}
          className="px-6 py-2.5 rounded-lg bg-zec-yellow text-zec-dark font-semibold hover:brightness-110 transition-all"
        >
          Let's go
        </button>
      </div>
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
  progress,
  syncPercentage,
  blockHeight,
  estimatedHeight,
}: {
  isStarting: boolean;
  isSyncing: boolean;
  message?: string;
  progress?: number;
  syncPercentage?: number;
  blockHeight?: number;
  estimatedHeight?: number;
}) {
  const rawStep = useMemo(() => messageToStep(message), [message]);
  const [highestStep, setHighestStep] = useState(0);
  if (rawStep > highestStep) setHighestStep(rawStep);
  const currentStep = highestStep;

  // Monotonic guard — progress bar never goes backwards
  const highestProgress = useRef(0);

  // Has verification started? (checkpoint progress available)
  const verifying = isStarting && progress != null && progress > 0;

  // Unified 0-100%: quick startup steps = 0-5%, blockchain work = 5-100%
  let totalProgress: number;
  let statusText: string;

  if (isStarting && !verifying) {
    // Quick startup steps: 0-5%
    totalProgress = (currentStep / STARTUP_STEPS.length) * 5;
    statusText = message || "Initializing node...";
  } else if (verifying && progress != null) {
    // Verification phase (before RPC): 5-100% from checkpoint progress
    totalProgress = 5 + (progress * 0.95);
    statusText = progress >= 99
      ? "Almost ready..."
      : message || "Verifying blockchain history";
  } else if (isSyncing && syncPercentage != null) {
    // Syncing phase (RPC live): 5-100% from RPC sync percentage
    totalProgress = 5 + (syncPercentage * 0.95);
    statusText = "Downloading and verifying every Zcash transaction since 2016";
  } else {
    totalProgress = 5;
    statusText = "Starting sync...";
  }

  // Apply monotonic guard
  if (totalProgress > highestProgress.current) {
    highestProgress.current = totalProgress;
  }
  const displayProgress = highestProgress.current;

  // When verification is active, mark first 4 steps done, keep "Verifying blockchain" as active spinner
  const checklistStep = verifying ? STARTUP_STEPS.length - 1 : currentStep;

  return (
    <div className="space-y-4">
      {/* Main progress */}
      <div className="space-y-2">
        <div className="flex items-baseline justify-between">
          <span className="text-3xl font-bold text-zec-text tabular-nums">
            {displayProgress < 5 ? `${displayProgress.toFixed(0)}%` : `${displayProgress.toFixed(1)}%`}
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
            style={{ width: `${Math.max(displayProgress, 0.5)}%` }}
          />
        </div>
        <p className="text-xs text-zec-muted">{statusText}</p>
      </div>

      {/* Step checklist — during startup */}
      {isStarting && (
        <div className="space-y-1.5">
          {STARTUP_STEPS.map((step, i) => {
            const isDone = i < checklistStep;
            const isActive = i === checklistStep;
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

function MiniToggle({
  label,
  tip,
  enabled,
  disabled,
  loading,
  onToggle,
}: {
  label: string;
  tip: string;
  enabled: boolean;
  disabled?: boolean;
  loading?: boolean;
  onToggle?: () => void;
}) {
  return (
    <div className={`flex items-center gap-2 ${disabled ? "opacity-40" : ""}`}>
      <span className="text-xs text-zec-muted flex items-center gap-1">
        {label}
        <InfoTip text={tip} />
      </span>
      <button
        onClick={onToggle}
        role="switch"
        aria-label={`Toggle ${label}`}
        aria-checked={enabled}
        disabled={disabled || loading}
        className={`relative w-7 h-4 rounded-full transition-colors shrink-0 ${
          disabled || loading
            ? loading ? "bg-zec-yellow/20 cursor-wait" : "bg-zec-border/50 cursor-not-allowed"
            : enabled ? "bg-emerald-400" : "bg-zec-border hover:bg-zec-muted/20"
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-3 h-3 rounded-full bg-white transition-transform duration-200 ${
            enabled ? "translate-x-3" : ""
          }`}
        />
      </button>
    </div>
  );
}

