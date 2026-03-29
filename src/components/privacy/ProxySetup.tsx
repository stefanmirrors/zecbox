import { useState } from "react";
import type { ProxySetupConfig } from "../../lib/types";
import {
  startProxySetup,
  getProxySetupConfig,
  enableProxyMode,
  verifyProxyConnection,
} from "../../lib/tauri";
import ProviderDirectory from "./ProviderDirectory";

interface Props {
  onComplete: () => void;
}

type SetupStep = "providers" | "vps_ip" | "deploy" | "connecting" | "verifying" | "done";

export default function ProxySetup({ onComplete }: Props) {
  const [step, setStep] = useState<SetupStep>("providers");
  const [vpsIp, setVpsIp] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [setupConfig, setSetupConfig] = useState<ProxySetupConfig | null>(null);
  const [copied, setCopied] = useState(false);

  const handleHaveVps = () => {
    setStep("vps_ip");
  };

  const handleSubmitIp = async () => {
    setError(null);
    try {
      await startProxySetup(vpsIp);
      const config = await getProxySetupConfig();
      setSetupConfig(config);
      setStep("deploy");
    } catch (e) {
      setError(String(e));
    }
  };

  const handleCopyCommand = async () => {
    if (setupConfig) {
      await navigator.clipboard.writeText(setupConfig.installCommand);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleConnect = async () => {
    setStep("connecting");
    setError(null);
    try {
      await enableProxyMode();
      setStep("verifying");
      const reachable = await verifyProxyConnection();
      if (reachable) {
        setStep("done");
      } else {
        setStep("verifying");
        setError("Relay is not yet reachable. Make sure the Docker container is running on your VPS, then try again.");
      }
    } catch (e) {
      setError(String(e));
      setStep("deploy");
    }
  };

  if (step === "providers") {
    return (
      <div className="space-y-6">
        <div className="text-center space-y-2">
          <h2 className="text-lg font-semibold text-zec-text">Get a VPS</h2>
          <p className="text-sm text-zec-muted">
            You need a small VPS to run your relay. These providers accept Zcash and don't require identity verification.
          </p>
        </div>

        <ProviderDirectory tier="proxy" />

        <div className="space-y-2 text-xs text-zec-muted">
          <p className="font-medium text-zec-text">What you need:</p>
          <ul className="list-disc list-inside space-y-1">
            <li>Any Linux VPS with Docker</li>
            <li>512 MB RAM, minimal storage</li>
            <li>Pay with shielded ZEC for full anonymity</li>
          </ul>
        </div>

        <button
          onClick={handleHaveVps}
          className="w-full py-3 rounded-xl font-semibold bg-zec-yellow text-zec-dark hover:brightness-110 transition-all"
        >
          I have a VPS — Enter IP
        </button>
      </div>
    );
  }

  if (step === "vps_ip") {
    return (
      <div className="space-y-6">
        <div className="text-center space-y-2">
          <h2 className="text-lg font-semibold text-zec-text">Enter VPS IP</h2>
          <p className="text-sm text-zec-muted">
            Enter the public IP address of your VPS.
          </p>
        </div>

        <div>
          <input
            type="text"
            value={vpsIp}
            onChange={(e) => setVpsIp(e.target.value)}
            placeholder="203.0.113.50"
            className="w-full px-4 py-3 bg-zec-surface border border-zec-border rounded-xl text-sm text-zec-text placeholder:text-zec-muted/40 focus:border-zec-yellow/60 focus:outline-none transition-colors"
          />
          {error && <p className="text-xs text-red-400/80 mt-2">{error}</p>}
        </div>

        <button
          onClick={handleSubmitIp}
          disabled={!vpsIp.trim()}
          className={`w-full py-3 rounded-xl font-semibold transition-all ${
            vpsIp.trim()
              ? "bg-zec-yellow text-zec-dark hover:brightness-110"
              : "bg-zec-border/50 text-zec-muted cursor-not-allowed"
          }`}
        >
          Generate Config
        </button>
      </div>
    );
  }

  if (step === "deploy") {
    return (
      <div className="space-y-6">
        <div className="text-center space-y-2">
          <h2 className="text-lg font-semibold text-zec-text">Deploy Relay</h2>
          <p className="text-sm text-zec-muted">
            Run this command on your VPS to set up the relay container.
          </p>
        </div>

        {setupConfig && (
          <div className="space-y-3">
            <div className="relative">
              <pre className="bg-zec-surface border border-zec-border rounded-xl p-4 text-xs text-zec-text overflow-x-auto whitespace-pre-wrap break-all max-h-32 overflow-y-auto">
                {setupConfig.installCommand}
              </pre>
              <button
                onClick={handleCopyCommand}
                className="absolute top-2 right-2 px-2 py-1 bg-zec-border/50 rounded text-[10px] text-zec-muted hover:text-zec-text transition-colors"
              >
                {copied ? "Copied!" : "Copy"}
              </button>
            </div>
          </div>
        )}

        {error && <p className="text-xs text-red-400/80">{error}</p>}

        <button
          onClick={handleConnect}
          className="w-full py-3 rounded-xl font-semibold bg-zec-yellow text-zec-dark hover:brightness-110 transition-all"
        >
          I've deployed — Connect
        </button>
      </div>
    );
  }

  if (step === "connecting") {
    return (
      <div className="space-y-6 text-center">
        <div className="space-y-2">
          <h2 className="text-lg font-semibold text-zec-text">Connecting...</h2>
          <p className="text-sm text-zec-muted">
            Establishing WireGuard tunnel to your VPS.
          </p>
        </div>
        <div className="flex justify-center">
          <div className="w-8 h-8 border-2 border-zec-yellow/30 border-t-zec-yellow rounded-full animate-spin" />
        </div>
      </div>
    );
  }

  if (step === "verifying") {
    return (
      <div className="space-y-6 text-center">
        <div className="space-y-2">
          <h2 className="text-lg font-semibold text-zec-text">Verifying Relay</h2>
          <p className="text-sm text-zec-muted">
            Checking that your VPS relay is reachable through the tunnel.
          </p>
        </div>
        {error ? (
          <>
            <p className="text-xs text-red-400/80">{error}</p>
            <button
              onClick={handleConnect}
              className="px-6 py-2 bg-zec-yellow text-zec-dark rounded-lg text-sm font-medium hover:brightness-110 transition-all"
            >
              Retry
            </button>
          </>
        ) : (
          <div className="flex justify-center">
            <div className="w-8 h-8 border-2 border-zec-yellow/30 border-t-zec-yellow rounded-full animate-spin" />
          </div>
        )}
      </div>
    );
  }

  // done
  return (
    <div className="space-y-6 text-center">
      <div className="space-y-2">
        <div className="text-3xl">✓</div>
        <h2 className="text-lg font-semibold text-zec-text">Proxy Mode Active</h2>
        <p className="text-sm text-zec-muted">
          Your node is now accepting connections through your VPS relay.
          The Zcash network sees your VPS IP, not your home IP.
        </p>
      </div>
      <button
        onClick={onComplete}
        className="px-6 py-2 bg-zec-yellow text-zec-dark rounded-lg text-sm font-medium hover:brightness-110 transition-all"
      >
        Done
      </button>
    </div>
  );
}
