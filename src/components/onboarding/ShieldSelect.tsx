import { useEffect, useState } from "react";
import {
  isFirewallHelperInstalled,
  installFirewallHelper,
  isShieldSupported,
} from "../../lib/tauri";

interface Props {
  onSelect: (shieldMode: boolean) => void;
}

type Choice = "standard" | "shield";

const choices: { id: Choice; label: string; tag?: string; oneLiner: string; benefits: string[]; tradeoffs: string[] }[] = [
  {
    id: "standard",
    label: "Standard",
    oneLiner: "Direct connection, no privacy features",
    benefits: ["Fastest sync speed", "Simplest setup, no extra cost"],
    tradeoffs: ["Your home IP is visible to other Zcash nodes"],
  },
  {
    id: "shield",
    label: "Shield Mode",
    tag: "Tor",
    oneLiner: "Full privacy — hidden IP, full network participation",
    benefits: [
      "Your IP is hidden from all peers and your ISP",
      "Accept incoming connections via .onion hidden service",
      "Full network participation while staying private",
      "No VPS or extra cost — uses the Tor network directly",
    ],
    tradeoffs: ["Initial sync will be slower due to Tor overhead"],
  },
];

export function ShieldSelect({ onSelect }: Props) {
  const [selected, setSelected] = useState<Choice>("standard");
  const [helperInstalled, setHelperInstalled] = useState<boolean | null>(null);
  const [shieldSupported, setShieldSupported] = useState<boolean | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    isFirewallHelperInstalled()
      .then(setHelperInstalled)
      .catch(() => setHelperInstalled(false));
    isShieldSupported()
      .then(setShieldSupported)
      .catch(() => setShieldSupported(false));
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

  const needsHelper = selected === "shield" && helperInstalled === false;
  const shieldDisabled = shieldSupported === false;

  const canContinue =
    selected === "standard" ||
    (selected === "shield" && !shieldDisabled && helperInstalled === true);

  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="w-full max-w-sm space-y-6">
        <div className="text-center space-y-2">
          <h2 className="text-2xl font-bold text-zec-text">Privacy</h2>
          <p className="text-sm text-zec-muted">
            Select a mode to see what it does.
          </p>
        </div>

        {/* Mode cards */}
        <div className="space-y-1.5">
          {choices.map((choice) => {
            const isSelected = selected === choice.id;
            const isDisabled = choice.id === "shield" && shieldDisabled;
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

        {/* Detail panel */}
        {(() => {
          const c = choices.find((c) => c.id === selected)!;
          return (
            <div className="border border-zec-border/50 rounded-xl p-4 space-y-3">
              <div className="space-y-1.5">
                {c.benefits.map((b, i) => (
                  <div key={i} className="flex items-start gap-2">
                    <span className="text-emerald-400 text-xs mt-px shrink-0">+</span>
                    <span className="text-xs text-emerald-400/80">{b}</span>
                  </div>
                ))}
              </div>
              <div className="space-y-1.5">
                {c.tradeoffs.map((t, i) => (
                  <div key={i} className="flex items-start gap-2">
                    <span className="text-zec-muted text-xs mt-px shrink-0">-</span>
                    <span className="text-xs text-zec-muted">{t}</span>
                  </div>
                ))}
              </div>
            </div>
          );
        })()}

        {/* Helper install prompt */}
        {needsHelper && !shieldDisabled && (
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

        <button
          onClick={() => onSelect(selected === "shield")}
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
