import { useState } from "react";
import { completeOnboarding } from "../../lib/tauri";

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

  return (
    <div className="flex min-h-[90vh] items-center justify-center px-6">
      <div className="max-w-sm w-full text-center space-y-10">
        <div className="space-y-2">
          <h2 className="text-2xl font-bold text-zec-text">Ready</h2>
          <p className="text-sm text-zec-muted">
            Your node will begin syncing the Zcash blockchain. This can take
            several hours depending on your connection.
          </p>
        </div>

        <div className="border border-zec-border rounded-xl p-4 text-left space-y-3">
          <div>
            <span className="text-xs text-zec-muted">Storage location</span>
            <p className="text-sm text-zec-text font-mono mt-1 break-all">
              {selectedPath}
            </p>
          </div>
          <div>
            <span className="text-xs text-zec-muted">Network privacy</span>
            <p className="text-sm text-zec-text mt-1">
              {shieldMode ? "Shielded (Tor)" : "Standard"}
            </p>
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
            ? shieldMode ? "Starting shielded..." : "Starting..."
            : shieldMode ? "Start Shielded Node" : "Start Node"}
        </button>
      </div>
    </div>
  );
}
