import { useNodeStatus } from "../../hooks/useNodeStatus";

export function NetworkPanel() {
  const nodeStatus = useNodeStatus();
  const isRunning = nodeStatus.status === "running";

  return (
    <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4">
      <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
        Network
      </h3>

      <div className="space-y-3">
        <div className="flex items-center justify-between">
          <span className="text-sm text-zec-muted">Network</span>
          <span className="text-sm text-zec-text font-medium">Mainnet</span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-zec-muted">Connected Peers</span>
          <span className="text-sm text-zec-text font-medium tabular-nums">
            {isRunning ? (nodeStatus.peerCount ?? 0) : "--"}
          </span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-zec-muted">Status</span>
          <div className="flex items-center gap-2">
            <span
              className={`w-2 h-2 rounded-full ${
                isRunning ? "bg-green-500" : "bg-zec-muted"
              }`}
            />
            <span className="text-sm text-zec-text">
              {isRunning ? "Connected" : "Disconnected"}
            </span>
          </div>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-zec-muted">P2P Port</span>
          <span className="text-sm text-zec-text font-mono">8233</span>
        </div>

        <div className="flex items-center justify-between">
          <span className="text-sm text-zec-muted">RPC Port</span>
          <span className="text-sm text-zec-text font-mono">8232</span>
        </div>
      </div>
    </div>
  );
}
