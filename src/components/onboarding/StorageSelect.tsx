import { useEffect, useState } from "react";
import { getVolumes } from "../../lib/tauri";
import { formatBytes } from "../../lib/format";
import type { Volume } from "../../lib/types";

const MIN_RECOMMENDED_BYTES = 350_000_000_000;
const MIN_USABLE_BYTES = 10_000_000_000;

interface Props {
  onSelect: (path: string) => void;
}

export function StorageSelect({ onSelect }: Props) {
  const [volumes, setVolumes] = useState<Volume[]>([]);
  const [loading, setLoading] = useState(true);
  const [selected, setSelected] = useState<string | null>(null);

  useEffect(() => {
    getVolumes()
      .then((vols) => {
        setVolumes(vols);
        const recommended = findRecommended(vols);
        if (recommended) {
          setSelected(recommended.mountPoint);
        }
        setLoading(false);
      })
      .catch(() => setLoading(false));
  }, []);

  const handleContinue = () => {
    if (selected) {
      onSelect(selected);
    }
  };

  if (loading) {
    return (
      <div className="flex min-h-[90vh] items-center justify-center">
        <p className="text-zec-muted">Scanning volumes...</p>
      </div>
    );
  }

  const recommended = findRecommended(volumes);

  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="w-full max-w-md space-y-8">
        <div className="text-center space-y-2">
          <h2 className="text-2xl font-bold text-zec-text">Storage</h2>
          <p className="text-sm text-zec-muted">
            The Zcash blockchain needs about 300 GB of disk space.
          </p>
        </div>

        <div className="space-y-2">
          {volumes.map((vol) => {
            const isRecommended =
              recommended?.mountPoint === vol.mountPoint;
            const tooSmall = vol.availableBytes < MIN_USABLE_BYTES;
            const isSelected = selected === vol.mountPoint;
            const usedPercent = Math.round(
              ((vol.totalBytes - vol.availableBytes) / vol.totalBytes) * 100
            );

            return (
              <button
                key={vol.mountPoint}
                disabled={tooSmall}
                onClick={() => setSelected(vol.mountPoint)}
                className={`w-full text-left p-4 rounded-xl border transition-all ${
                  tooSmall
                    ? "border-zec-border/50 opacity-30 cursor-not-allowed"
                    : isSelected
                      ? "border-zec-yellow/60 bg-zec-yellow/5"
                      : "border-zec-border hover:border-zec-border hover:bg-zec-surface-hover"
                }`}
              >
                <div className="flex items-center justify-between mb-3">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm text-zec-text">
                      {vol.name || vol.mountPoint}
                    </span>
                    {vol.isRemovable && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-border/50 text-zec-muted">
                        External
                      </span>
                    )}
                    {isRecommended && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-yellow/10 text-zec-yellow">
                        Recommended
                      </span>
                    )}
                  </div>
                  {tooSmall && (
                    <span className="text-[10px] text-red-400/80">Too small</span>
                  )}
                </div>

                <div className="flex items-center gap-3">
                  <div className="flex-1 h-1 rounded-full bg-zec-border overflow-hidden">
                    <div
                      className="h-full rounded-full bg-zec-muted/40"
                      style={{ width: `${usedPercent}%` }}
                    />
                  </div>
                  <span className="text-[11px] text-zec-muted whitespace-nowrap">
                    {formatBytes(vol.availableBytes)} free
                  </span>
                </div>
              </button>
            );
          })}
        </div>

        {volumes.length === 0 && (
          <p className="text-center text-sm text-red-400/80">
            No suitable storage volumes found.
          </p>
        )}

        <button
          onClick={handleContinue}
          disabled={!selected}
          className={`w-full py-3.5 rounded-xl font-semibold transition-all ${
            selected
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

function findRecommended(volumes: Volume[]): Volume | undefined {
  const suitable = volumes
    .filter((v) => v.availableBytes >= MIN_RECOMMENDED_BYTES)
    .sort((a, b) => b.availableBytes - a.availableBytes);
  if (suitable.length > 0) return suitable[0];
  return volumes
    .filter((v) => v.availableBytes >= MIN_USABLE_BYTES)
    .sort((a, b) => b.availableBytes - a.availableBytes)[0];
}
