interface Props {
  stealth: {
    status: { status: string; bootstrapProgress?: number; enabled: boolean };
    error: string | null;
    clearError: () => void;
    helperInstalled: boolean | null;
    installing: boolean;
    installHelper: () => void;
  };
}

export default function StealthStatus({ stealth }: Props) {
  const { status, error, clearError, helperInstalled, installing, installHelper } = stealth;

  return (
    <div className="border border-zec-border rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-medium text-zec-text">Stealth Mode (Tor)</span>
        <div className="flex items-center gap-2">
          <span
            className={`w-1.5 h-1.5 rounded-full ${
              status.status === "active" ? "bg-emerald-400"
              : status.status === "bootstrapping" ? "bg-zec-yellow animate-pulse"
              : status.status === "error" || status.status === "interrupted" ? "bg-red-400"
              : "bg-zec-muted/30"
            }`}
          />
          <span className="text-xs text-zec-muted">
            {status.status === "active" && "Connected via Tor"}
            {status.status === "bootstrapping" && `Connecting... ${status.bootstrapProgress ?? 0}%`}
            {status.status === "disabled" && "Disabled"}
            {status.status === "error" && "Error"}
            {status.status === "interrupted" && "Interrupted"}
          </span>
        </div>
      </div>

      {status.status === "bootstrapping" && (
        <div className="h-1 bg-zec-border rounded-full overflow-hidden">
          <div
            className="h-full bg-zec-yellow rounded-full transition-all duration-500"
            style={{ width: `${status.bootstrapProgress ?? 0}%` }}
          />
        </div>
      )}

      {helperInstalled === false && (
        <div className="border-t border-zec-border/50 pt-3">
          <p className="text-xs text-zec-muted mb-2">System helper required for Tor routing.</p>
          <button
            onClick={installHelper}
            disabled={installing}
            className="px-3 py-1.5 bg-zec-yellow text-zec-dark rounded-lg text-xs font-medium hover:brightness-110 disabled:opacity-50 transition-all"
          >
            {installing ? "Installing..." : "Install Helper"}
          </button>
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
