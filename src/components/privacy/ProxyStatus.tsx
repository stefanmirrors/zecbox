interface Props {
  proxy: {
    status: { status: string; vpsIp?: string; enabled: boolean; relayReachable?: boolean };
    error: string | null;
    clearError: () => void;
  };
}

export default function ProxyStatus({ proxy }: Props) {
  const { status, error, clearError } = proxy;

  return (
    <div className="border border-zec-border rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-zec-text">Proxy Mode (VPS)</span>
        <div className="flex items-center gap-2">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              status.status === "active" ? "bg-emerald-400"
              : status.status === "connecting" ? "bg-zec-yellow animate-pulse"
              : status.status === "error" || status.status === "interrupted" ? "bg-red-400"
              : "bg-zec-muted/30"
            }`}
          />
          <span className="text-xs text-zec-muted">
            {status.status === "active" && `Connected via ${status.vpsIp}`}
            {status.status === "connecting" && "Establishing tunnel..."}
            {status.status === "disabled" && "Disabled"}
            {status.status === "setup" && "Setup in progress"}
            {status.status === "error" && "Error"}
            {status.status === "interrupted" && "Interrupted"}
          </span>
        </div>
      </div>

      {status.status === "active" && (
        <div className="flex items-center gap-4 text-xs text-zec-muted">
          <span>VPS: {status.vpsIp}</span>
          {status.relayReachable !== undefined && (
            <span className={status.relayReachable ? "text-emerald-400/80" : "text-red-400/80"}>
              Relay: {status.relayReachable ? "reachable" : "unreachable"}
            </span>
          )}
        </div>
      )}

      {error && (
        <div className="flex items-start justify-between border-t border-zec-border/50 pt-3">
          <p className="text-xs text-red-400/80">{error}</p>
          <button onClick={clearError} className="text-xs text-zec-muted hover:text-zec-text ml-4">
            Dismiss
          </button>
        </div>
      )}
    </div>
  );
}
