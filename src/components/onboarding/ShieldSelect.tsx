import { useEffect, useState } from "react";
import {
  isFirewallHelperInstalled,
  installFirewallHelper,
} from "../../lib/tauri";

interface Props {
  onSelect: (shieldMode: boolean) => void;
}

type Choice = "standard" | "shielded";

export function ShieldSelect({ onSelect }: Props) {
  const [selected, setSelected] = useState<Choice>("standard");
  const [helperInstalled, setHelperInstalled] = useState<boolean | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    isFirewallHelperInstalled()
      .then(setHelperInstalled)
      .catch(() => setHelperInstalled(false));
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

  const canContinue =
    selected === "standard" || (selected === "shielded" && helperInstalled === true);

  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="w-full max-w-sm space-y-8">
        <div className="text-center space-y-2">
          <h2 className="text-2xl font-bold text-zec-text">Privacy</h2>
          <p className="text-sm text-zec-muted">
            Choose how your node connects to the Zcash network.
          </p>
        </div>

        <div className="space-y-2">
          <button
            onClick={() => setSelected("standard")}
            className={`w-full text-left p-4 rounded-xl border transition-all ${
              selected === "standard"
                ? "border-zec-yellow/60 bg-zec-yellow/5"
                : "border-zec-border hover:border-zec-border hover:bg-zec-surface-hover"
            }`}
          >
            <span className="font-medium text-sm text-zec-text">Standard</span>
            <p className="text-xs text-zec-muted mt-1">
              Connect directly to the Zcash network.
            </p>
          </button>

          <button
            onClick={() => setSelected("shielded")}
            className={`w-full text-left p-4 rounded-xl border transition-all ${
              selected === "shielded"
                ? "border-zec-yellow/60 bg-zec-yellow/5"
                : "border-zec-border hover:border-zec-border hover:bg-zec-surface-hover"
            }`}
          >
            <div className="flex items-center gap-2">
              <span className="font-medium text-sm text-zec-text">Shielded</span>
              <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-yellow/10 text-zec-yellow">
                Tor
              </span>
              <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-border/50 text-zec-muted">
                macOS
              </span>
            </div>
            <p className="text-xs text-zec-muted mt-1">
              Route all traffic through Tor. Your IP stays hidden from peers
              and your ISP. Currently available on macOS. Windows and Linux
              support coming soon.
            </p>
          </button>
        </div>

        {/* Helper install prompt */}
        {selected === "shielded" && helperInstalled === false && (
          <div className="border border-zec-yellow/20 rounded-xl p-4 space-y-3">
            <div>
              <p className="text-sm text-zec-text">System helper required</p>
              <p className="text-xs text-zec-muted mt-1">
                Shield Mode uses macOS firewall rules to enforce Tor routing. A
                one-time installation is needed. You'll be asked for your admin
                password.
              </p>
            </div>
            <button
              onClick={handleInstall}
              disabled={installing}
              className="px-4 py-2 bg-zec-yellow text-zec-dark rounded-lg text-sm font-medium hover:brightness-110 disabled:opacity-50 disabled:cursor-wait transition-all"
            >
              {installing ? "Installing..." : "Install Helper"}
            </button>
            {error && <p className="text-xs text-red-400/80">{error}</p>}
          </div>
        )}

        {/* Helper installed confirmation */}
        {selected === "shielded" && helperInstalled === true && (
          <p className="text-xs text-zec-muted/60">
            Initial sync will be slower over Tor. You can change this anytime.
          </p>
        )}

        <button
          onClick={() => onSelect(selected === "shielded")}
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
