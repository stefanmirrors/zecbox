import { useNodeStatus } from "../../hooks/useNodeStatus";
import { startNode, stopNode } from "../../lib/tauri";

export function Dashboard() {
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

  return (
    <div className="flex min-h-screen items-center justify-center bg-zec-dark">
      <div className="text-center space-y-6">
        <h1 className="text-5xl font-bold text-zec-yellow tracking-tight">
          ZecBox
        </h1>
        <p className="text-zec-muted text-lg">
          Your Zcash full node, simplified.
        </p>

        <div className="space-y-4">
          <div className="flex items-center justify-center gap-3">
            <span
              className={`inline-block w-3 h-3 rounded-full ${
                isRunning
                  ? "bg-green-500"
                  : nodeStatus.status === "error"
                    ? "bg-red-500"
                    : nodeStatus.status === "starting"
                      ? "bg-yellow-500 animate-pulse"
                      : "bg-zec-muted"
              }`}
            />
            <span className="text-zec-text capitalize">
              {nodeStatus.status}
            </span>
          </div>

          {isRunning && (
            <div className="text-sm text-zec-muted space-y-1">
              <p>Block Height: {nodeStatus.blockHeight?.toLocaleString()}</p>
              <p>Peers: {nodeStatus.peerCount}</p>
            </div>
          )}

          {nodeStatus.status === "error" && nodeStatus.message && (
            <p className="text-sm text-red-400">{nodeStatus.message}</p>
          )}

          <button
            onClick={handleToggle}
            disabled={isBusy}
            className={`px-6 py-2 rounded-lg font-medium transition-colors ${
              isBusy
                ? "bg-zec-border text-zec-muted cursor-not-allowed"
                : isRunning
                  ? "bg-red-600 hover:bg-red-700 text-white"
                  : "bg-zec-yellow hover:bg-yellow-500 text-zec-dark"
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
    </div>
  );
}
