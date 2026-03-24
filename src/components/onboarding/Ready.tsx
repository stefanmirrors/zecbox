import { useState } from "react";
import { completeOnboarding } from "../../lib/tauri";

interface Props {
  selectedPath: string;
  onComplete: () => void;
}

export function Ready({ selectedPath, onComplete }: Props) {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleStart = async () => {
    setLoading(true);
    setError(null);
    try {
      await completeOnboarding(selectedPath);
      onComplete();
    } catch (e) {
      setError(typeof e === "string" ? e : "Failed to start node. Please try again.");
      setLoading(false);
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center px-6">
      <div className="max-w-md w-full text-center space-y-8">
        <div className="space-y-2">
          <h2 className="text-3xl font-bold text-zec-text">Ready to Go</h2>
          <p className="text-zec-muted">
            Your node will begin syncing the Zcash blockchain. This may take
            several hours depending on your connection.
          </p>
        </div>

        <div className="bg-zec-surface border border-zec-border rounded-lg p-4 text-left space-y-2">
          <div className="flex justify-between">
            <span className="text-zec-muted text-sm">Storage</span>
            <span className="text-zec-text text-sm font-medium">
              {selectedPath}
            </span>
          </div>
        </div>

        {error && (
          <p className="text-sm text-red-400">{error}</p>
        )}

        <button
          onClick={handleStart}
          disabled={loading}
          className={`w-full py-3 rounded-lg font-semibold text-lg transition-all ${
            loading
              ? "bg-zec-border text-zec-muted cursor-not-allowed"
              : "bg-zec-yellow text-zec-dark hover:brightness-110"
          }`}
        >
          {loading ? "Starting..." : "Start Node"}
        </button>
      </div>
    </div>
  );
}
