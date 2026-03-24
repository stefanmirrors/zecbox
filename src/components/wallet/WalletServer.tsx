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
    <div className="max-w-2xl space-y-6">
      {/* Main toggle card */}
      <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div
              className={`w-10 h-10 rounded-lg flex items-center justify-center ${
                status.enabled
                  ? "bg-emerald-500/10 text-emerald-400"
                  : "bg-zec-border text-zec-muted"
              }`}
              aria-hidden="true"
            >
              <svg
                width="22"
                height="22"
                viewBox="0 0 18 18"
                fill="none"
                stroke="currentColor"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <rect x="1.5" y="4" width="15" height="11" rx="2" />
                <path d="M1.5 8h15" />
                <circle cx="13" cy="11.5" r="1" />
              </svg>
            </div>
            <div>
              <h2 className="text-lg font-semibold text-zec-text">
                Wallet Server
              </h2>
              <p className="text-sm text-zec-muted">
                Serve light wallets via gRPC (Zaino)
              </p>
            </div>
          </div>
          <button
            onClick={toggle}
            role="switch"
            aria-label="Toggle Wallet Server"
            aria-checked={status.enabled}
            disabled={toggling || !nodeRunning || status.status === "starting"}
            className={`relative w-12 h-7 rounded-full transition-colors ${
              toggling || status.status === "starting"
                ? "bg-zec-border cursor-wait"
                : !nodeRunning
                  ? "bg-zec-border cursor-not-allowed opacity-50"
                  : status.enabled
                    ? "bg-emerald-500"
                    : "bg-zec-border hover:bg-zec-muted/30"
            }`}
          >
            <span
              className={`absolute top-1 left-1 w-5 h-5 rounded-full bg-white transition-transform duration-200 ${
                status.enabled ? "translate-x-5" : ""
              }`}
            />
          </button>
        </div>

        {/* Status indicator */}
        <div className="flex items-center gap-2 text-sm">
          <span
            className={`w-2 h-2 rounded-full ${
              status.status === "running"
                ? "bg-emerald-400"
                : status.status === "starting"
                  ? "bg-yellow-400 animate-pulse"
                  : status.status === "error"
                    ? "bg-red-400"
                    : "bg-zec-muted/40"
            }`}
          />
          <span className="text-zec-muted">
            {status.status === "running" && "Running"}
            {status.status === "starting" && "Starting..."}
            {status.status === "stopped" && "Stopped"}
            {status.status === "stopping" && "Stopping..."}
            {status.status === "error" && "Error"}
          </span>
        </div>
      </div>

      {/* Endpoint display when running */}
      {status.status === "running" && status.endpoint && (
        <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
          <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider mb-4">
            gRPC Endpoint
          </h3>
          <div className="flex items-center gap-3 mb-4">
            <code className="flex-1 bg-zec-dark px-4 py-2.5 rounded-lg text-zec-text font-mono text-sm">
              {status.endpoint}
            </code>
            <button
              onClick={copyEndpoint}
              className="px-3 py-2.5 bg-zec-dark hover:bg-zec-border rounded-lg text-sm text-zec-muted hover:text-zec-text transition-colors"
            >
              {copied ? "Copied" : "Copy"}
            </button>
          </div>
          {qrDataUrl && (
            <div className="flex justify-center">
              <div className="bg-zec-dark rounded-lg p-4">
                <img
                  src={qrDataUrl}
                  alt="gRPC endpoint QR code"
                  className="w-48 h-48"
                />
              </div>
            </div>
          )}
        </div>
      )}

      {/* Error message */}
      {error && (
        <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-4 flex items-start justify-between">
          <div>
            <p className="text-sm font-medium text-red-400">
              Wallet Server Error
            </p>
            <p className="text-sm text-red-400/80 mt-1">{error}</p>
          </div>
          <button
            onClick={clearError}
            className="text-red-400/60 hover:text-red-400 text-sm ml-4"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* Info cards */}
      <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4">
        <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
          How it works
        </h3>
        <div className="space-y-3 text-sm text-zec-muted">
          <InfoRow
            title="Light wallet support"
            description="Zaino indexes blockchain data and serves it to light wallets via gRPC on port 9067."
          />
          <InfoRow
            title="Local network"
            description="Wallets on the same WiFi network can connect using the endpoint address. Remote access requires port forwarding."
          />
          <InfoRow
            title="Node dependency"
            description="The wallet server requires the node to be running. If the node stops, the wallet server will also stop."
          />
        </div>
      </div>

      {/* Node not running message */}
      {!nodeRunning && status.status === "stopped" && (
        <div className="bg-zec-surface/50 border border-zec-border rounded-lg p-4">
          <p className="text-sm text-zec-muted">
            The node must be running to enable the wallet server. Start the node
            from the Dashboard, then return here to enable Zaino.
          </p>
        </div>
      )}
    </div>
  );
}

function InfoRow({
  title,
  description,
}: {
  title: string;
  description: string;
}) {
  return (
    <div>
      <p className="text-zec-text font-medium">{title}</p>
      <p className="text-zec-muted mt-0.5">{description}</p>
    </div>
  );
}
