import { useState } from "react";
import { completeOnboarding } from "../../lib/tauri";

interface ModeSummary {
  title: string;
  description: string;
  details: string[];
}

const modeSummaries: Record<string, ModeSummary> = {
  standard: {
    title: "Standard Node",
    description: "Your node connects directly to the Zcash network with no privacy features enabled.",
    details: [
      "Fastest sync speed — direct connection to peers",
      "Your home IP address is visible to other Zcash nodes",
      "No extra infrastructure or cost required",
    ],
  },
  shield: {
    title: "Shielded Node",
    description: "All traffic routed through Tor. Your node accepts incoming connections via a .onion hidden service — full network participation with complete IP privacy.",
    details: [
      "Your IP is hidden from all peers and your ISP",
      "Accept incoming connections via .onion address",
      "Full network participation while staying private",
      "No VPS or extra cost — uses the Tor network directly",
      "Initial sync will be slower due to Tor overhead",
    ],
  },
};

interface Props {
  selectedPath: string;
  shieldMode: boolean;
  onComplete: () => void;
}

export function Ready({ selectedPath, shieldMode, onComplete }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    try {
      await completeOnboarding(selectedPath, shieldMode);
      onComplete();
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start node. Please try again.");
      setLoading(false);
    }
  };

  const summary = modeSummaries[shieldMode ? "shield" : "standard"];

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
            ? shieldMode ? "Starting shielded node..." : "Starting..."
            : shieldMode ? "Start Shielded Node" : "Start Node"}
        </button>
      </div>
    </div>
  );
}
