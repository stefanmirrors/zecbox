import { useShieldMode } from "../../hooks/useShieldMode";
import { useNodeStatus } from "../../hooks/useNodeStatus";

export default function ShieldMode() {
  const {
    status,
    toggling,
    error,
    toggle,
    clearError,
    helperInstalled,
    installing,
    installHelper,
  } = useShieldMode();
  const nodeStatus = useNodeStatus();
  const nodeRunning = nodeStatus.status === "running";

  return (
    <div className="max-w-2xl space-y-6">
      {/* Helper installation prompt */}
      {helperInstalled === false && (
        <div className="bg-zec-yellow/10 border border-zec-yellow/30 rounded-lg p-5">
          <div className="flex items-start gap-3">
            <div className="w-8 h-8 rounded-lg bg-zec-yellow/20 flex items-center justify-center flex-shrink-0 mt-0.5">
              <svg
                width="18"
                height="18"
                viewBox="0 0 18 18"
                fill="none"
                stroke="currentColor"
                className="text-zec-yellow"
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
              >
                <path d="M9 1.5L2 4.5v4.5c0 4.1 3 7.3 7 8.5 4-1.2 7-4.4 7-8.5V4.5z" />
              </svg>
            </div>
            <div className="flex-1">
              <h3 className="text-sm font-semibold text-zec-text mb-1">
                System Helper Required
              </h3>
              <p className="text-sm text-zec-muted mb-3">
                Shield Mode uses macOS firewall rules to enforce Tor routing for
                all node traffic. A one-time system helper installation is
                needed. You will be prompted for your admin password.
              </p>
              <button
                onClick={installHelper}
                disabled={installing}
                className="px-4 py-2 bg-zec-yellow text-zec-bg rounded-lg text-sm font-medium hover:bg-zec-yellow/90 disabled:opacity-50 disabled:cursor-wait transition-colors"
              >
                {installing ? "Installing..." : "Install System Helper"}
              </button>
            </div>
          </div>
        </div>
      )}

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
                <path d="M9 1.5L2 4.5v4.5c0 4.1 3 7.3 7 8.5 4-1.2 7-4.4 7-8.5V4.5z" />
              </svg>
            </div>
            <div>
              <h2 className="text-lg font-semibold text-zec-text">
                Shield Mode
              </h2>
              <p className="text-sm text-zec-muted">
                Route node traffic through the Tor network
              </p>
            </div>
          </div>
          <button
            onClick={toggle}
            role="switch"
            aria-label="Toggle Shield Mode"
            aria-checked={status.enabled}
            disabled={
              toggling ||
              status.status === "bootstrapping" ||
              helperInstalled === false
            }
            className={`relative w-12 h-7 rounded-full transition-colors ${
              toggling || status.status === "bootstrapping"
                ? "bg-zec-border cursor-wait"
                : helperInstalled === false
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
              status.status === "active"
                ? "bg-emerald-400"
                : status.status === "bootstrapping"
                  ? "bg-yellow-400 animate-pulse"
                  : status.status === "error" || status.status === "interrupted"
                    ? "bg-red-400"
                    : "bg-zec-muted/40"
            }`}
          />
          <span className="text-zec-muted">
            {status.status === "active" && "Connected via Tor — traffic enforced by firewall"}
            {status.status === "bootstrapping" &&
              `Connecting to Tor network... ${status.bootstrapProgress ?? 0}%`}
            {status.status === "disabled" && "Disabled"}
            {status.status === "error" && "Error"}
            {status.status === "interrupted" && "Interrupted"}
          </span>
        </div>

        {/* Bootstrap progress bar */}
        {status.status === "bootstrapping" && (
          <div className="mt-3 h-1.5 bg-zec-border rounded-full overflow-hidden">
            <div
              className="h-full bg-zec-yellow rounded-full transition-all duration-500"
              style={{ width: `${status.bootstrapProgress ?? 0}%` }}
            />
          </div>
        )}
      </div>

      {/* Error message */}
      {error && (
        <div className="bg-red-500/10 border border-red-500/30 rounded-lg p-4 flex items-start justify-between">
          <div>
            <p className="text-sm font-medium text-red-400">
              Shield Mode Error
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
            title="Network privacy"
            description="Your ISP cannot see you are running a Zcash node. Peers cannot see your real IP address."
          />
          <InfoRow
            title="Firewall enforcement"
            description="macOS firewall rules redirect all node P2P traffic through Tor. No traffic can bypass Tor while Shield Mode is active."
          />
          <InfoRow
            title="Kill switch"
            description="If the Tor connection or firewall rules drop, the node is immediately stopped to prevent clearnet exposure."
          />
          <InfoRow
            title="Performance"
            description="Tor adds latency. Initial sync will be significantly slower. Best used after initial sync is complete."
          />
        </div>
      </div>

      {/* Node status context */}
      {!nodeRunning && status.status === "disabled" && (
        <div className="bg-zec-surface/50 border border-zec-border rounded-lg p-4">
          <p className="text-sm text-zec-muted">
            The node is not running. Shield Mode can be enabled before or after
            starting the node. If the node is running when Shield Mode is
            toggled, it will be automatically restarted.
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
