import { useEffect, useState } from "react";
import { useWalletServer } from "../../hooks/useWalletServer";
import { useNodeStatus } from "../../hooks/useNodeStatus";
import { getWalletQr } from "../../lib/tauri";

export default function WalletServer() {
  const { status, toggling, error, toggle, clearError } = useWalletServer();
  const nodeStatus = useNodeStatus();
  const nodeRunning = nodeStatus.status === "running";
  const [qrDataUrl, setQrDataUrl] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (status.status === "running") {
      getWalletQr().then(setQrDataUrl).catch(() => setQrDataUrl(null));
    } else {
      setQrDataUrl(null);
    }
  }, [status.status]);

  const copyEndpoint = async () => {
    if (status.endpoint) {
      await navigator.clipboard.writeText(status.endpoint);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="space-y-6">
      {/* Main toggle */}
      <div className="border border-zec-border rounded-xl p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-base font-semibold text-zec-text">Wallet Server</h2>
            <p className="text-xs text-zec-muted mt-0.5">Serve light wallets via gRPC</p>
          </div>
          <button
            onClick={toggle}
            role="switch"
            aria-label="Toggle Wallet Server"
            aria-checked={status.enabled}
            disabled={toggling || !nodeRunning || status.status === "starting"}
            className={`relative w-10 h-5.5 rounded-full transition-colors shrink-0 ${
              toggling || status.status === "starting"
                ? "bg-zec-border/50 cursor-wait"
                : !nodeRunning
                  ? "bg-zec-border/30 cursor-not-allowed"
                  : status.enabled
                    ? "bg-emerald-400"
                    : "bg-zec-border hover:bg-zec-muted/20"
            }`}
          >
            <span
              className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200 ${
                status.enabled ? "translate-x-5" : ""
              }`}
            />
          </button>
        </div>

        <div className="flex items-center gap-2">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              status.status === "running" ? "bg-emerald-400"
              : status.status === "starting" ? "bg-zec-yellow animate-pulse"
              : status.status === "error" ? "bg-red-400"
              : "bg-zec-muted/30"
            }`}
          />
          <span className="text-xs text-zec-muted">
            {status.status === "running" && "Running on port 9067"}
            {status.status === "starting" && "Starting..."}
            {status.status === "stopped" && "Stopped"}
            {status.status === "stopping" && "Stopping..."}
            {status.status === "error" && "Error"}
          </span>
        </div>
      </div>

      {/* Endpoint */}
      {status.status === "running" && status.endpoint && (
        <div className="border border-zec-border rounded-xl p-5 space-y-4">
          <h3 className="text-xs font-medium text-zec-muted">gRPC Endpoint</h3>
          <div className="flex items-center gap-2">
            <code className="flex-1 px-3 py-2 rounded-lg border border-zec-border text-sm font-mono text-zec-text truncate">
              {status.endpoint}
            </code>
            <button
              onClick={copyEndpoint}
              className="px-3 py-2 rounded-lg border border-zec-border text-xs text-zec-muted hover:text-zec-text transition-colors shrink-0"
            >
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
          {qrDataUrl && (
            <div className="flex justify-center pt-2">
              <img src={qrDataUrl} alt="Endpoint QR code" className="w-40 h-40 rounded-lg" />
            </div>
          )}
        </div>
      )}

      {/* Error */}
      {error && (
        <div className="border border-red-400/20 rounded-xl p-4 flex items-start justify-between">
          <p className="text-sm text-red-400/80">{error}</p>
          <button onClick={clearError} className="text-xs text-zec-muted hover:text-zec-text ml-4">
            Dismiss
          </button>
        </div>
      )}

      {/* How it works */}
      <div className="space-y-4">
        <h3 className="text-xs font-medium text-zec-muted">How it works</h3>
        <div className="space-y-3">
          <Info title="Light wallet support" text="Zaino indexes blockchain data and serves it to light wallets via gRPC on port 9067." />
          <Info title="Local network" text="Wallets on the same network can connect using the endpoint. Remote access requires port forwarding." />
          <Info title="Node dependency" text="The wallet server requires the node to be running. If the node stops, the wallet server will also stop." />
        </div>
      </div>

      {!nodeRunning && status.status === "stopped" && (
        <p className="text-xs text-zec-muted/60">
          Start the node from the Dashboard to enable the wallet server.
        </p>
      )}
    </div>
  );
}

function Info({ title, text }: { title: string; text: string }) {
  return (
    <div>
      <p className="text-sm text-zec-text">{title}</p>
      <p className="text-xs text-zec-muted mt-0.5">{text}</p>
    </div>
  );
}
