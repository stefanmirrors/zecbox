import { useShieldMode } from "../../hooks/useShieldMode";

export default function ShieldMode() {
  const {
    status, toggling, error, toggle, clearError,
    helperInstalled, installing, installHelper,
  } = useShieldMode();

  return (
    <div className="space-y-6">
      {/* Helper installation */}
      {helperInstalled === false && (
        <div className="border border-zec-yellow/20 rounded-xl p-5">
          <h3 className="text-sm font-medium text-zec-text mb-1">
            System Helper Required
          </h3>
          <p className="text-xs text-zec-muted mb-4">
            Shield Mode uses system firewall rules to enforce Tor routing.
            A one-time system helper installation is needed.
          </p>
          <button
            onClick={installHelper}
            disabled={installing}
            className="px-4 py-2 bg-zec-yellow text-zec-dark rounded-lg text-sm font-medium hover:brightness-110 disabled:opacity-50 disabled:cursor-wait transition-all"
          >
            {installing ? "Installing..." : "Install Helper"}
          </button>
        </div>
      )}

      {/* Main toggle */}
      <div className="border border-zec-border rounded-xl p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-base font-semibold text-zec-text">Shield Mode</h2>
            <p className="text-xs text-zec-muted mt-0.5">Route node traffic through Tor</p>
          </div>
          <button
            onClick={toggle}
            role="switch"
            aria-label="Toggle Shield Mode"
            aria-checked={status.enabled}
            disabled={toggling || status.status === "bootstrapping" || helperInstalled === false}
            className={`relative w-10 h-5.5 rounded-full transition-colors shrink-0 ${
              toggling || status.status === "bootstrapping"
                ? "bg-zec-border/50 cursor-wait"
                : helperInstalled === false
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
              status.status === "active" ? "bg-emerald-400"
              : status.status === "bootstrapping" ? "bg-zec-yellow animate-pulse"
              : status.status === "error" || status.status === "interrupted" ? "bg-red-400"
              : "bg-zec-muted/30"
            }`}
          />
          <span className="text-xs text-zec-muted">
            {status.status === "active" && "Connected via Tor"}
            {status.status === "bootstrapping" && `Connecting to Tor... ${status.bootstrapProgress ?? 0}%`}
            {status.status === "disabled" && "Disabled"}
            {status.status === "error" && "Error"}
            {status.status === "interrupted" && "Interrupted"}
          </span>
        </div>

        {status.status === "bootstrapping" && (
          <div className="h-1 bg-zec-border rounded-full overflow-hidden">
            <div
              className="h-full bg-zec-yellow rounded-full transition-all duration-500"
              style={{ width: `${status.bootstrapProgress ?? 0}%` }}
            />
          </div>
        )}
      </div>

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
          <Info title="Network privacy" text="Your ISP cannot see you are running a Zcash node. Peers cannot see your real IP." />
          <Info title="Firewall enforcement" text="System firewall rules redirect all P2P traffic through Tor. No traffic can bypass." />
          <Info title="Kill switch" text="If Tor or firewall rules drop, the node stops immediately to prevent clearnet exposure." />
          <Info title="Performance" text="Tor adds latency. Best used after initial sync is complete." />
        </div>
      </div>

      <p className="text-xs text-zec-muted/60">
        Available on macOS and Linux. Windows support coming soon.
      </p>
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
