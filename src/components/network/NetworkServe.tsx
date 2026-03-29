import { useState, useMemo } from "react";
import { useNetworkServe } from "../../hooks/useNetworkServe";
import { useNodeStatus } from "../../hooks/useNodeStatus";
import { useStealthMode } from "../../hooks/useStealthMode";

function maskIp(ip: string): string {
  const parts = ip.split(".");
  if (parts.length === 4) {
    return `${parts[0]}.${parts[1]}.***.***`;
  }
  return ip.replace(/:[\da-f]+$/i, ":****");
}

export default function NetworkServe() {
  const { status, toggling, error, toggle, recheck, clearError } = useNetworkServe();
  const nodeStatus = useNodeStatus();
  const { status: stealthStatus } = useStealthMode();
  const nodeRunning = nodeStatus.status === "running";
  const stealthActive = stealthStatus.status === "active" || stealthStatus.status === "bootstrapping";
  const [rechecking, setRechecking] = useState(false);
  const [showIp, setShowIp] = useState(false);
  const displayIp = useMemo(() => {
    if (!status.publicIp) return null;
    return showIp ? status.publicIp : maskIp(status.publicIp);
  }, [status.publicIp, showIp]);

  const handleRecheck = async () => {
    setRechecking(true);
    await recheck();
    setRechecking(false);
  };

  const isDisabled = toggling || !nodeRunning || stealthActive || status.status === "enabling";

  return (
    <div className="space-y-6">
      {/* Main toggle */}
      <div className="border border-zec-border rounded-xl p-6 space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h2 className="text-base font-semibold text-zec-text">Serve the Network</h2>
            <p className="text-xs text-zec-muted mt-0.5">
              Accept inbound connections to strengthen Zcash
            </p>
          </div>
          <button
            onClick={toggle}
            role="switch"
            aria-label="Toggle network serving"
            aria-checked={status.enabled}
            disabled={isDisabled}
            className={`relative w-10 h-5.5 rounded-full transition-colors shrink-0 ${
              toggling || status.status === "enabling"
                ? "bg-zec-border/50 cursor-wait"
                : !nodeRunning || stealthActive
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
              : status.status === "enabling" ? "bg-zec-yellow animate-pulse"
              : status.status === "error" ? "bg-red-400"
              : "bg-zec-muted/30"
            }`}
          />
          <span className="text-xs text-zec-muted">
            {status.status === "active" && "Serving"}
            {status.status === "enabling" && "Setting up..."}
            {status.status === "disabled" && "Off"}
            {status.status === "error" && "Error"}
          </span>
        </div>
      </div>

      {/* Stealth Mode warning */}
      {stealthActive && (
        <div className="border border-zec-yellow/20 rounded-xl p-4">
          <p className="text-sm text-zec-yellow/80">
            Stealth Mode is active. Accepting inbound connections is not possible while routing through Tor.
            Disable Stealth Mode first to serve the network.
          </p>
        </div>
      )}

      {/* Status panel — when active */}
      {status.status === "active" && (
        <div className="border border-zec-border rounded-xl p-5 space-y-4">
          <h3 className="text-xs font-medium text-zec-muted">Status</h3>
          <div className="space-y-3">
            {/* Reachability */}
            <Row
              label="Reachable"
              value={
                status.reachable === true
                  ? "Yes"
                  : status.reachable === false
                    ? "No"
                    : "Checking..."
              }
              dot={
                status.reachable === true
                  ? "bg-emerald-400"
                  : status.reachable === false
                    ? "bg-zec-yellow"
                    : "bg-zec-muted/30 animate-pulse"
              }
            />
            {/* UPnP */}
            <Row
              label="Port forwarding"
              value={status.upnpActive ? "Active (UPnP)" : "Manual required"}
              dot={status.upnpActive ? "bg-emerald-400" : "bg-zec-yellow"}
            />
            {/* Public IP */}
            {displayIp && (
              <div className="flex items-center justify-between">
                <span className="text-sm text-zec-muted">Public IP</span>
                <div className="flex items-center gap-2">
                  <span className="text-sm text-zec-text tabular-nums">{displayIp}</span>
                  <button
                    onClick={() => setShowIp(!showIp)}
                    className="text-[10px] text-zec-muted hover:text-zec-text"
                  >
                    {showIp ? "Hide" : "Show"}
                  </button>
                </div>
              </div>
            )}
            {/* Peers */}
            {status.inboundPeers != null && status.outboundPeers != null ? (
              <>
                <Row
                  label="Inbound peers"
                  value={String(status.inboundPeers)}
                />
                <Row
                  label="Outbound peers"
                  value={String(status.outboundPeers)}
                />
              </>
            ) : (
              <Row
                label="Peers"
                value={String((status.inboundPeers ?? 0) + (status.outboundPeers ?? 0))}
              />
            )}
          </div>
        </div>
      )}

      {/* Manual port forwarding instructions */}
      {status.status === "active" && !status.upnpActive && (
        <div className="border border-zec-border rounded-xl p-5 space-y-3">
          <h3 className="text-xs font-medium text-zec-yellow">Manual port forwarding needed</h3>
          <p className="text-xs text-zec-muted">
            Automatic port forwarding (UPnP) is not available on your router. To make your node reachable:
          </p>
          <ol className="text-xs text-zec-muted list-decimal list-inside space-y-1">
            <li>Open your router's admin page</li>
            <li>
              Forward <span className="text-zec-text font-medium">TCP port 8233</span> to{" "}
              <span className="text-zec-text font-medium">{status.localIp ?? "your computer's IP"}</span>
            </li>
            <li>Save and click Recheck below</li>
          </ol>
          <button
            onClick={handleRecheck}
            disabled={rechecking}
            className="mt-2 px-4 py-1.5 rounded-lg border border-zec-border text-xs text-zec-muted hover:text-zec-text transition-colors disabled:cursor-wait"
          >
            {rechecking ? "Checking..." : "Recheck"}
          </button>
        </div>
      )}

      {/* CGNAT warning */}
      {status.status === "active" && status.cgnatDetected && (
        <div className="border border-red-400/20 rounded-xl p-4">
          <p className="text-sm text-red-400/80">
            Your ISP uses shared networking (CGNAT). Inbound connections are not possible on this network.
            This is a limitation of your internet provider, not your router. Contact your ISP to request a dedicated public IP if available.
          </p>
        </div>
      )}

      {/* Reachability warning */}
      {status.status === "active" && status.reachable === false && !status.cgnatDetected && status.upnpActive && (
        <div className="border border-zec-yellow/20 rounded-xl p-4 space-y-2">
          <p className="text-sm text-zec-yellow/80">
            Port 8233 is not reachable from outside. Your router may be blocking the connection despite UPnP.
          </p>
          <button
            onClick={handleRecheck}
            disabled={rechecking}
            className="px-4 py-1.5 rounded-lg border border-zec-border text-xs text-zec-muted hover:text-zec-text transition-colors disabled:cursor-wait"
          >
            {rechecking ? "Checking..." : "Recheck"}
          </button>
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
          <Info
            title="Strengthen the network"
            text="Accepting inbound connections lets other nodes connect to you, helping the Zcash network stay decentralized and resilient."
          />
          <Info
            title="Port forwarding"
            text="Your router needs to allow incoming connections on port 8233. ZecBox tries to set this up automatically via UPnP. If that fails, you can configure it manually."
          />
          <Info
            title="Stealth Mode conflict"
            text="This feature cannot be used with Stealth Mode. Tor routes traffic through onion relays which prevent direct inbound connections."
          />
        </div>
      </div>

      {!nodeRunning && !stealthActive && status.status === "disabled" && (
        <p className="text-xs text-zec-muted/60">
          Start the node from the Dashboard to enable network serving.
        </p>
      )}
    </div>
  );
}

function Row({ label, value, dot }: { label: string; value: string; dot?: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-zec-muted">{label}</span>
      <div className="flex items-center gap-2">
        {dot && <span className={`w-1.5 h-1.5 rounded-full ${dot}`} />}
        <span className="text-sm text-zec-text tabular-nums">{value}</span>
      </div>
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
