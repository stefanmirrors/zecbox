import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import type { ShieldStatusInfo } from "../../lib/types";
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

const shieldFacts = [
  {
    title: "Hiding your IP address",
    description: "No one on the Zcash network will ever see your real IP. Not peers, not observers, not your ISP.",
  },
  {
    title: "Building encrypted circuits",
    description: "Your traffic passes through multiple encrypted relays. Each relay only knows the previous and next hop — never the full path.",
  },
  {
    title: "Creating your .onion address",
    description: "Your node gets a unique hidden service address. Other Zcash nodes can connect to you without knowing where you are.",
  },
  {
    title: "Protecting your identity",
    description: "Running a Zcash node reveals nothing about your transactions. Shield Mode ensures it also reveals nothing about your location.",
  },
  {
    title: "Enforcing firewall rules",
    description: "System-level firewall rules guarantee all traffic goes through Tor. Even if something goes wrong, your IP can never leak.",
  },
  {
    title: "Kill switch enabled",
    description: "If Tor drops or firewall rules are removed, your node stops immediately. We never silently fall back to an unprotected connection. Your privacy is protected even during failures.",
  },
];

interface Props {
  selectedPath: string;
  shieldMode: boolean;
  onComplete: () => void;
}

export function Ready({ selectedPath, shieldMode, onComplete }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [activeFact, setActiveFact] = useState(0);

  // Listen for shield bootstrap progress during onboarding
  useEffect(() => {
    if (!loading || !shieldMode) return;

    const unlisten = listen<ShieldStatusInfo>("shield_status_changed", (event) => {
      if (event.payload.status === "active") {
        // Tor connected — the command will complete shortly
      }
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [loading, shieldMode]);

  // Cycle through facts while waiting
  useEffect(() => {
    if (!loading || !shieldMode) return;
    const interval = setInterval(() => {
      setActiveFact((prev) => (prev + 1) % shieldFacts.length);
    }, 8000);
    return () => clearInterval(interval);
  }, [loading, shieldMode]);

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    setActiveFact(0);
    try {
      await completeOnboarding(selectedPath, shieldMode);
      onComplete();
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start node. Please try again.");
      setLoading(false);
    }
  };

  const summary = modeSummaries[shieldMode ? "shield" : "standard"];

  // Show shield startup screen
  if (loading && shieldMode) {
    const fact = shieldFacts[activeFact];
    return (
      <div className="flex min-h-[90vh] items-center justify-center px-6">
        <div className="max-w-sm w-full text-center space-y-10">
          <div className="space-y-4">
            <div className="flex justify-center">
              <div className="w-12 h-12 border-2 border-zec-yellow/20 border-t-zec-yellow rounded-full animate-spin" />
            </div>
            <h2 className="text-xl font-bold text-zec-text">Securing Your Node</h2>
          </div>

          <div className="border border-zec-border/50 rounded-xl p-6 space-y-3 min-h-[120px] flex flex-col justify-center transition-all">
            <p className="text-sm font-medium text-zec-yellow">{fact.title}</p>
            <p className="text-xs text-zec-muted leading-relaxed">{fact.description}</p>
          </div>

          {/* Dot indicator for facts */}
          <div className="flex justify-center gap-1.5">
            {shieldFacts.map((_, i) => (
              <div
                key={i}
                className={`h-1 rounded-full transition-all duration-500 ${
                  i === activeFact ? "w-4 bg-zec-yellow" : "w-1 bg-zec-border"
                }`}
              />
            ))}
          </div>

          <p className="text-[11px] text-zec-muted/30">
            This usually takes 30-60 seconds
          </p>

          {error && (
            <p className="text-sm text-red-400/80">{error}</p>
          )}
        </div>
      </div>
    );
  }

  // Standard loading
  if (loading) {
    return (
      <div className="flex min-h-[90vh] items-center justify-center px-6">
        <div className="max-w-sm w-full text-center space-y-6">
          <div className="flex justify-center">
            <div className="w-10 h-10 border-2 border-zec-yellow/20 border-t-zec-yellow rounded-full animate-spin" />
          </div>
          <h2 className="text-xl font-bold text-zec-text">Starting Node</h2>
          <p className="text-sm text-zec-muted">Preparing your node...</p>
          {error && <p className="text-sm text-red-400/80">{error}</p>}
        </div>
      </div>
    );
  }

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
          className="w-full py-3.5 rounded-xl font-semibold bg-zec-yellow text-zec-dark hover:brightness-110 transition-all"
        >
          {shieldMode ? "Start Shielded Node" : "Start Node"}
        </button>
      </div>
    </div>
  );
}
