import { useEffect, useState } from "react";
import type { PrivacyMode } from "../../lib/types";
import {
  isFirewallHelperInstalled,
  installFirewallHelper,
  isStealthSupported,
} from "../../lib/tauri";

interface Props {
  onSelect: (privacyMode: PrivacyMode) => void;
}

const choices: { id: PrivacyMode; label: string; tag?: string; oneLiner: string; benefits: string[]; tradeoffs: string[]; needsStealth?: boolean; needsProxy?: boolean }[] = [
  {
    id: "standard",
    label: "Standard",
    oneLiner: "Direct connection, no privacy features",
    benefits: ["Fastest sync speed", "Simplest setup, no extra cost"],
    tradeoffs: ["Your home IP is visible to other Zcash nodes"],
  },
  {
    id: "stealth",
    label: "Stealth",
    tag: "Tor",
    oneLiner: "All traffic routed through Tor",
    benefits: ["Your IP is hidden from all peers and your ISP"],
    tradeoffs: ["Cannot accept incoming connections", "Slower sync"],
    needsStealth: true,
  },
  {
    id: "proxy",
    label: "Proxy",
    tag: "VPS",
    oneLiner: "VPS relay hides your IP from the network",
    benefits: ["Full network participation while staying private", "Help decentralize Zcash"],
    tradeoffs: ["Requires a VPS ($3-5/mo)", "Outbound still uses home IP"],
    needsProxy: true,
  },
  {
    id: "shield",
    label: "Shield",
    tag: "Stealth + Proxy",
    oneLiner: "Maximum privacy in both directions",
    benefits: ["Complete IP privacy — hidden inbound and outbound", "Full network participation"],
    tradeoffs: ["Requires a VPS ($3-5/mo)", "Slower sync from Tor"],
    needsStealth: true,
    needsProxy: true,
  },
];

export function ShieldSelect({ onSelect }: Props) {
  const [selected, setSelected] = useState<PrivacyMode>("standard");
  const [helperInstalled, setHelperInstalled] = useState<boolean | null>(null);
  const [stealthSupported, setStealthSupported] = useState<boolean | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    isFirewallHelperInstalled()
      .then(setHelperInstalled)
      .catch(() => setHelperInstalled(false));
    isStealthSupported()
      .then(setStealthSupported)
      .catch(() => setStealthSupported(false));
  }, []);

  const handleInstall = async () => {
    setInstalling(true);
    setError(null);
    try {
      await installFirewallHelper();
      setHelperInstalled(true);
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to install helper.");
    } finally {
      setInstalling(false);
    }
  };

  const selectedChoice = choices.find((c) => c.id === selected)!;
  const needsHelper = selectedChoice.needsStealth && helperInstalled === false;
  const stealthDisabled = stealthSupported === false;

  const canContinue =
    selected === "standard" ||
    selected === "proxy" ||
    (selectedChoice.needsStealth && !stealthDisabled && helperInstalled === true);

  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="w-full max-w-sm space-y-6">
        <div className="text-center space-y-2">
          <h2 className="text-2xl font-bold text-zec-text">Privacy</h2>
          <p className="text-sm text-zec-muted">
            Select a mode to see what it does.
          </p>
        </div>

        {/* Compact mode cards */}
        <div className="space-y-1.5">
          {choices.map((choice) => {
            const isSelected = selected === choice.id;
            const isDisabled = choice.needsStealth && stealthDisabled;
            return (
              <button
                key={choice.id}
                onClick={() => !isDisabled && setSelected(choice.id)}
                disabled={isDisabled}
                className={`w-full text-left px-4 py-3 rounded-xl border transition-all ${
                  isSelected
                    ? "border-zec-yellow/60 bg-zec-yellow/5"
                    : "border-zec-border hover:bg-zec-surface-hover"
                } ${isDisabled ? "opacity-30 cursor-not-allowed" : "cursor-pointer"}`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm text-zec-text">{choice.label}</span>
                    {choice.tag && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-yellow/10 text-zec-yellow">
                        {choice.tag}
                      </span>
                    )}
                    {isDisabled && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-border/50 text-zec-muted">
                        macOS/Linux
                      </span>
                    )}
                  </div>
                  <div className={`w-3.5 h-3.5 rounded-full border-2 transition-colors shrink-0 ${
                    isSelected ? "border-zec-yellow bg-zec-yellow" : "border-zec-border"
                  }`}>
                    {isSelected && (
                      <div className="w-full h-full rounded-full flex items-center justify-center">
                        <div className="w-1.5 h-1.5 rounded-full bg-zec-dark" />
                      </div>
                    )}
                  </div>
                </div>
                <p className="text-xs text-zec-muted mt-0.5">{choice.oneLiner}</p>
              </button>
            );
          })}
        </div>

        {/* Detail panel for selected mode */}
        <div className="border border-zec-border/50 rounded-xl p-4 space-y-3">
          <div className="space-y-1.5">
            {selectedChoice.benefits.map((b, i) => (
              <div key={i} className="flex items-start gap-2">
                <span className="text-emerald-400 text-xs mt-px shrink-0">+</span>
                <span className="text-xs text-emerald-400/80">{b}</span>
              </div>
            ))}
          </div>
          <div className="space-y-1.5">
            {selectedChoice.tradeoffs.map((t, i) => (
              <div key={i} className="flex items-start gap-2">
                <span className="text-zec-muted text-xs mt-px shrink-0">-</span>
                <span className="text-xs text-zec-muted">{t}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Helper install prompt */}
        {needsHelper && !stealthDisabled && (
          <div className="border border-zec-yellow/20 rounded-xl p-4 space-y-3">
            <p className="text-xs text-zec-muted">
              Requires a one-time system helper install for Tor firewall rules.
            </p>
            <button
              onClick={handleInstall}
              disabled={installing}
              className="px-4 py-2 bg-zec-yellow text-zec-dark rounded-lg text-xs font-medium hover:brightness-110 disabled:opacity-50 disabled:cursor-wait transition-all"
            >
              {installing ? "Installing..." : "Install Helper"}
            </button>
            {error && <p className="text-xs text-red-400/80">{error}</p>}
          </div>
        )}

        {/* Proxy note */}
        {(selected === "proxy" || selected === "shield") && (
          <p className="text-xs text-zec-muted/50 text-center">
            Proxy setup continues after onboarding.
          </p>
        )}

        <button
          onClick={() => onSelect(selected)}
          disabled={!canContinue}
          className={`w-full py-3.5 rounded-xl font-semibold transition-all ${
            canContinue
              ? "bg-zec-yellow text-zec-dark hover:brightness-110"
              : "bg-zec-border/50 text-zec-muted cursor-not-allowed"
          }`}
        >
          Continue
        </button>
      </div>
    </div>
  );
}
