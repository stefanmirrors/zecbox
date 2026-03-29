import { useState } from "react";
import type { PrivacyMode } from "../../lib/types";
import { completeOnboarding } from "../../lib/tauri";

interface ModeSummary {
  title: string;
  description: string;
  details: string[];
}

const modeSummaries: Record<PrivacyMode, ModeSummary> = {
  standard: {
    title: "Standard Node",
    description: "Your node connects directly to the Zcash network with no privacy features enabled.",
    details: [
      "Fastest sync speed — direct connection to peers",
      "Your home IP address is visible to other Zcash nodes",
      "No extra infrastructure or cost required",
    ],
  },
  stealth: {
    title: "Stealth Node",
    description: "All your node's traffic will be routed through the Tor network. Your IP address is hidden from every peer.",
    details: [
      "Outbound traffic routed through Tor — your IP is invisible",
      "Your ISP cannot see you are running a Zcash node",
      "Cannot accept incoming connections from other nodes",
      "Initial sync will be slower due to Tor overhead",
    ],
  },
  proxy: {
    title: "Proxy Node",
    description: "A lightweight relay on your VPS will accept connections on behalf of your node. The network sees your VPS IP, never your home IP.",
    details: [
      "Full network participation — accept incoming peer connections",
      "Other nodes see your VPS IP, not your home IP",
      "You help decentralize the Zcash network while staying private",
      "Outbound connections still use your home IP",
      "Proxy setup continues after onboarding — you'll need your VPS IP",
    ],
  },
  shield: {
    title: "Shielded Node",
    description: "Maximum protection. Outbound traffic goes through Tor, inbound connections arrive through your VPS relay. Your home IP is never visible to any peer in either direction.",
    details: [
      "Complete IP privacy — hidden in both directions",
      "Full network participation through your VPS relay",
      "Outbound through Tor, inbound through WireGuard tunnel",
      "Initial sync will be slower due to Tor overhead",
      "Proxy setup continues after onboarding — you'll need your VPS IP",
    ],
  },
};

interface Props {
  selectedPath: string;
  privacyMode: PrivacyMode;
  onComplete: () => void;
}

export function Ready({ selectedPath, privacyMode, onComplete }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    try {
      await completeOnboarding(selectedPath, privacyMode);
      onComplete();
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start node. Please try again.");
      setLoading(false);
    }
  };

  const summary = modeSummaries[privacyMode];
  const isPrivate = privacyMode !== "standard";

  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="max-w-sm w-full text-center space-y-8">
        <div className="space-y-2">
          <h2 className="text-2xl font-bold text-zec-text">Ready</h2>
          <p className="text-sm text-zec-muted">
            Review your setup before starting.
          </p>
        </div>

        {/* Node summary card */}
        <div className="border border-zec-border rounded-xl p-5 text-left space-y-4">
          <div>
            <h3 className="text-base font-semibold text-zec-text">{summary.title}</h3>
            <p className="text-xs text-zec-muted mt-1.5 leading-relaxed">{summary.description}</p>
          </div>

          <ul className="space-y-2">
            {summary.details.map((detail, i) => (
              <li key={i} className="flex items-start gap-2 text-xs text-zec-muted">
                <span className="text-zec-yellow mt-0.5 shrink-0">-</span>
                <span>{detail}</span>
              </li>
            ))}
          </ul>

          <div className="border-t border-zec-border/50 pt-3">
            <span className="text-[10px] text-zec-muted/50 uppercase tracking-wider">Storage</span>
            <p className="text-xs text-zec-text font-mono mt-1 break-all">{selectedPath}</p>
          </div>
        </div>

        {error && (
          <p className="text-sm text-red-400/80">{error}</p>
        )}

        <button
          onClick={handleStart}
          disabled={loading}
          className={`w-full py-3.5 rounded-xl font-semibold transition-all ${
            loading
              ? "bg-zec-border/50 text-zec-muted cursor-not-allowed"
              : "bg-zec-yellow text-zec-dark hover:brightness-110"
          }`}
        >
          {loading
            ? isPrivate ? "Starting private node..." : "Starting..."
            : isPrivate ? "Start Private Node" : "Start Node"}
        </button>
      </div>
    </div>
  );
}
