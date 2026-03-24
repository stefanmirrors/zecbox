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
        // Pre-select the recommended volume
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
      <div className="flex min-h-screen items-center justify-center">
        <p className="text-zec-muted text-lg">Scanning volumes...</p>
      </div>
    );
  }

  const recommended = findRecommended(volumes);

  return (
    <div className="flex min-h-screen items-center justify-center px-6">
      <div className="w-full max-w-lg space-y-6">
        <div className="text-center space-y-2">
          <h2 className="text-3xl font-bold text-zec-text">
            Choose Storage Location
          </h2>
          <p className="text-zec-muted">
            The Zcash blockchain requires approximately 300 GB of disk space.
          </p>
        </div>

        <div className="space-y-3">
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
                className={`w-full text-left p-4 rounded-lg border transition-all ${
                  tooSmall
                    ? "border-zec-border opacity-40 cursor-not-allowed"
                    : isSelected
                      ? "border-zec-yellow ring-2 ring-zec-yellow bg-zec-surface"
                      : "border-zec-border bg-zec-surface hover:border-zec-muted"
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-zec-text">
                      {vol.name || vol.mountPoint}
                    </span>
                    {vol.isRemovable && (
                      <span className="text-xs px-2 py-0.5 rounded bg-zec-border text-zec-muted">
                        External
                      </span>
                    )}
                    {isRecommended && (
                      <span className="text-xs px-2 py-0.5 rounded bg-zec-yellow/20 text-zec-yellow font-medium">
                        Recommended
                      </span>
                    )}
                  </div>
                  {tooSmall && (
                    <span className="text-xs text-red-400">Too small</span>
                  )}
                </div>

                <p className="text-sm text-zec-muted mb-2">
                  {vol.mountPoint}
                </p>

                <div className="flex items-center gap-3">
                  <div className="flex-1 h-2 rounded-full bg-zec-border overflow-hidden">
                    <div
                      className="h-full rounded-full bg-zec-muted"
                      style={{ width: `${usedPercent}%` }}
                    />
                  </div>
                  <span className="text-xs text-zec-muted whitespace-nowrap">
                    {formatBytes(vol.availableBytes)} free of{" "}
                    {formatBytes(vol.totalBytes)}
                  </span>
                </div>
              </button>
            );
          })}
        </div>

        {volumes.length === 0 && (
          <p className="text-center text-red-400">
            No suitable storage volumes found.
          </p>
        )}

        <button
          onClick={handleContinue}
          disabled={!selected}
          className={`w-full py-3 rounded-lg font-semibold text-lg transition-all ${
            selected
              ? "bg-zec-yellow text-zec-dark hover:brightness-110"
              : "bg-zec-border text-zec-muted cursor-not-allowed"
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
  // Fallback: largest volume with at least minimum usable space
  return volumes
    .filter((v) => v.availableBytes >= MIN_USABLE_BYTES)
    .sort((a, b) => b.availableBytes - a.availableBytes)[0];
}
