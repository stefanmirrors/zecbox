import { useEffect, useState } from "react";
import type { VpsProvider } from "../../lib/types";
import { getVpsProviders } from "../../lib/tauri";

interface Props {
  tier: string;
}

export default function ProviderDirectory({ tier }: Props) {
  const [providers, setProviders] = useState<VpsProvider[]>([]);

  useEffect(() => {
    getVpsProviders(tier)
      .then(setProviders)
      .catch(() => setProviders([]));
  }, [tier]);

  if (providers.length === 0) return null;

  return (
    <div className="border border-zec-border rounded-xl divide-y divide-zec-border">
      {providers.map((p) => {
        const tierInfo = p.tiers.find((t) => t.useCase === tier);
        return (
          <div key={p.name} className="p-4 space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-sm font-medium text-zec-text">{p.name}</span>
              {tierInfo && (
                <span className="text-xs text-zec-muted">from {tierInfo.estimatedCost}</span>
              )}
            </div>
            <p className="text-xs text-zec-muted">{p.description}</p>
            <div className="flex items-center gap-2 pt-1">
              {p.acceptsZec && (
                <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-yellow/10 text-zec-yellow">
                  Accepts ZEC
                </span>
              )}
              {p.noKyc && (
                <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-emerald-400/10 text-emerald-400">
                  No KYC
                </span>
              )}
              <span className="text-[10px] text-zec-muted">{p.locations.join(", ")}</span>
              <a
                href={p.url}
                target="_blank"
                rel="noopener noreferrer"
                className="ml-auto text-[10px] text-zec-yellow hover:underline"
              >
                Visit ↗
              </a>
            </div>
          </div>
        );
      })}
    </div>
  );
}
