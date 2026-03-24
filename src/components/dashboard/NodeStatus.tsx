import { useNodeStatus } from "../../hooks/useNodeStatus";
import { startNode, stopNode } from "../../lib/tauri";

export function NodeStatus() {
  const nodeStatus = useNodeStatus();
  const isRunning = nodeStatus.status === "running";
  const isBusy =
    nodeStatus.status === "starting" || nodeStatus.status === "stopping";

  const handleToggle = async () => {
    try {
      if (isRunning) {
        await stopNode();
      } else {
        await startNode();
      }
    } catch (e) {
      console.error("Node toggle failed:", e);
    }
  };

  const statusColor = {
    stopped: "bg-zec-muted",
    starting: "bg-yellow-500 animate-pulse",
    running: "bg-green-500",
    stopping: "bg-yellow-500 animate-pulse",
    error: "bg-red-500",
  }[nodeStatus.status];

  return (
    <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
      <div className="flex items-center justify-between">
        <div className="space-y-3">
          <div className="flex items-center gap-3">
            <span className={`inline-block w-3 h-3 rounded-full ${statusColor}`} />
            <span className="text-lg font-semibold text-zec-text capitalize">
              {nodeStatus.status === "error" ? "Error" : nodeStatus.status}
            </span>
          </div>

          {isRunning && (
            <div className="flex items-center gap-6">
              <div>
                <p className="text-xs text-zec-muted uppercase tracking-wider">
                  Block Height
                </p>
                <p className="text-2xl font-bold text-zec-text tabular-nums">
                  {nodeStatus.blockHeight?.toLocaleString() ?? "..."}
                </p>
              </div>
              <div>
                <p className="text-xs text-zec-muted uppercase tracking-wider">
                  Peers
                </p>
                <p className="text-2xl font-bold text-zec-text tabular-nums">
                  {nodeStatus.peerCount ?? 0}
                </p>
              </div>
            </div>
          )}

          {nodeStatus.status === "error" && nodeStatus.message && (
            <p className="text-sm text-red-400">{nodeStatus.message}</p>
          )}
        </div>

        <button
          onClick={handleToggle}
          disabled={isBusy}
          className={`px-6 py-2.5 rounded-lg font-medium transition-colors ${
            isBusy
              ? "bg-zec-border text-zec-muted cursor-not-allowed"
              : isRunning
                ? "bg-red-600 hover:bg-red-700 text-white"
                : "bg-zec-yellow hover:brightness-110 text-zec-dark"
          }`}
        >
          {isBusy
            ? nodeStatus.status === "starting"
              ? "Starting..."
              : "Stopping..."
            : isRunning
              ? "Stop Node"
              : "Start Node"}
        </button>
      </div>
    </div>
  );
}
