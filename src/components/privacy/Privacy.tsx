import { useState } from "react";
import type { PrivacyMode } from "../../lib/types";
import { usePrivacyMode } from "../../hooks/usePrivacyMode";
import { useStealthMode } from "../../hooks/useStealthMode";
import { useProxyMode } from "../../hooks/useProxyMode";
import StealthStatus from "./StealthStatus";
import ProxyStatus from "./ProxyStatus";
import ProxySetup from "./ProxySetup";

const modes: { id: PrivacyMode; label: string; tag?: string; description: string; benefits: string; tradeoffs: string }[] = [
  {
    id: "standard",
    label: "Standard",
    description: "Your node connects directly to the Zcash network.",
    benefits: "Fastest sync, simplest setup, no extra cost.",
    tradeoffs: "Other Zcash nodes can see your home IP address.",
  },
  {
    id: "stealth",
    label: "Stealth Mode",
    tag: "Tor",
    description: "All your node's traffic is routed through the Tor network. You're invisible.",
    benefits: "Your IP is completely hidden from all Zcash peers.",
    tradeoffs: "Cannot accept incoming connections — your node doesn't help other nodes find peers. Slower sync due to Tor overhead.",
  },
  {
    id: "proxy",
    label: "Proxy Mode",
    tag: "VPS",
    description: "A lightweight relay on your VPS accepts connections on your node's behalf.",
    benefits: "Full network participation — serve other nodes, help decentralize the network, and accept incoming connections, all while keeping your home IP private.",
    tradeoffs: "Requires a VPS running Docker ($3-5/month). Outbound connections still use your home IP.",
  },
  {
    id: "shield",
    label: "Shield Mode",
    tag: "Stealth + Proxy",
    description: "Maximum protection. Outbound through Tor, inbound through your VPS relay.",
    benefits: "Complete IP privacy AND full network participation. Your home IP is never visible to any peer in either direction.",
    tradeoffs: "Requires a VPS ($3-5/month). Slower sync from Tor overhead. Most complex setup.",
  },
];

export default function Privacy() {
  const { mode, changeMode } = usePrivacyMode();
  const stealth = useStealthMode();
  const proxy = useProxyMode();
  const [selected, setSelected] = useState<PrivacyMode | null>(null);
  const [showProxySetup, setShowProxySetup] = useState(false);

  const currentMode = mode;
  const pendingSelection = selected ?? currentMode;

  const needsProxySetup = (m: PrivacyMode) => m === "proxy" || m === "shield";
  const proxyConfigured = proxy.status.status !== "disabled" || proxy.status.vpsIp;

  const handleApply = async () => {
    if (!selected || selected === currentMode) return;

    if (needsProxySetup(selected) && !proxyConfigured) {
      setShowProxySetup(true);
      return;
    }

    await changeMode(selected);
    setSelected(null);
  };

  const handleProxySetupComplete = () => {
    setShowProxySetup(false);
    if (selected) {
      changeMode(selected);
      setSelected(null);
    }
  };

  if (showProxySetup) {
    return (
      <div className="space-y-6">
        <button
          onClick={() => setShowProxySetup(false)}
          className="text-xs text-zec-muted hover:text-zec-text transition-colors"
        >
          ← Back to privacy modes
        </button>
        <ProxySetup onComplete={handleProxySetupComplete} />
      </div>
    );
  }

  return (
    <div className="space-y-6">
      {/* Mode selection */}
      <div className="space-y-2">
        {modes.map((m) => {
          const isSelected = pendingSelection === m.id;
          const isCurrent = currentMode === m.id;
          const isDisabled = (m.id === "stealth" || m.id === "shield") && stealth.platformSupported === false;

          return (
            <button
              key={m.id}
              onClick={() => !isDisabled && setSelected(m.id)}
              disabled={isDisabled}
              className={`w-full text-left p-4 rounded-xl border transition-all ${
                isSelected
                  ? "border-zec-yellow/60 bg-zec-yellow/5"
                  : "border-zec-border hover:border-zec-border hover:bg-zec-surface-hover"
              } ${isDisabled ? "opacity-40 cursor-not-allowed" : ""}`}
            >
              <div className="flex items-center gap-2">
                <span className={`font-medium text-sm ${isSelected ? "text-zec-text" : "text-zec-text"}`}>
                  {m.label}
                </span>
                {m.tag && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-yellow/10 text-zec-yellow">
                    {m.tag}
                  </span>
                )}
                {isCurrent && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-emerald-400/10 text-emerald-400">
                    Active
                  </span>
                )}
                {isDisabled && (
                  <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-border/50 text-zec-muted">
                    macOS/Linux
                  </span>
                )}
              </div>
              <p className="text-xs text-zec-muted mt-1">{m.description}</p>
              <div className="mt-3 space-y-1.5 border-t border-zec-border/50 pt-3">
                <p className="text-xs text-emerald-400/80">
                  <span className="font-medium">Benefits:</span> {m.benefits}
                </p>
                <p className="text-xs text-zec-muted">
                  <span className="font-medium">Tradeoffs:</span> {m.tradeoffs}
                </p>
              </div>
            </button>
          );
        })}
      </div>

      {/* Apply button */}
      {selected && selected !== currentMode && (
        <button
          onClick={handleApply}
          className="w-full py-3 rounded-xl font-semibold bg-zec-yellow text-zec-dark hover:brightness-110 transition-all"
        >
          {needsProxySetup(selected) && !proxyConfigured ? "Set Up Proxy" : "Apply"}
        </button>
      )}

      {/* Current status */}
      <div className="space-y-4">
        {(currentMode === "stealth" || currentMode === "shield") && (
          <StealthStatus stealth={stealth} />
        )}
        {(currentMode === "proxy" || currentMode === "shield") && (
          <ProxyStatus proxy={proxy} />
        )}
        {currentMode === "standard" && (
          <div className="border border-zec-border rounded-xl p-4">
            <div className="flex items-center gap-2">
              <span className="w-1.5 h-1.5 rounded-full bg-zec-muted/30" />
              <span className="text-xs text-zec-muted">
                Standard mode — your node is connected directly to the Zcash network.
              </span>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}
