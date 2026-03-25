import { useShieldMode } from "../../hooks/useShieldMode";

export function QuickActions() {
  const { status, toggling, toggle } = useShieldMode();

  return (
    <div className="space-y-3">
      <h3 className="text-xs font-medium text-zec-muted">Features</h3>
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        <FeatureToggle
          label="Shield Mode"
          description="Route traffic through Tor"
          enabled={status.enabled}
          loading={toggling || status.status === "bootstrapping"}
          statusText={
            status.status === "bootstrapping"
              ? `${status.bootstrapProgress ?? 0}%`
              : undefined
          }
          onToggle={toggle}
        />
        <FeatureToggle
          label="Wallet Server"
          description="Serve light wallets via gRPC"
          enabled={false}
          disabled
        />
      </div>
    </div>
  );
}

function FeatureToggle({
  label,
  description,
  enabled,
  disabled,
  loading,
  statusText,
  onToggle,
}: {
  label: string;
  description: string;
  enabled: boolean;
  disabled?: boolean;
  loading?: boolean;
  statusText?: string;
  onToggle?: () => void;
}) {
  return (
    <div
      className={`flex items-center justify-between p-4 rounded-xl border transition-colors ${
        disabled
          ? "border-zec-border/30 opacity-40"
          : "border-zec-border"
      }`}
    >
      <div>
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-zec-text">{label}</p>
          {loading && statusText && (
            <span className="text-[10px] px-1.5 py-0.5 rounded-full bg-zec-yellow/10 text-zec-yellow">
              {statusText}
            </span>
          )}
        </div>
        <p className="text-[11px] text-zec-muted mt-0.5">{description}</p>
      </div>
      <button
        onClick={onToggle}
        role="switch"
        aria-label={`Toggle ${label}`}
        aria-checked={enabled}
        disabled={disabled || loading}
        className={`relative w-9 h-5 rounded-full transition-colors shrink-0 ml-4 ${
          disabled || loading
            ? loading
              ? "bg-zec-yellow/20 cursor-wait"
              : "bg-zec-border/50 cursor-not-allowed"
            : enabled
              ? "bg-emerald-400"
              : "bg-zec-border hover:bg-zec-muted/20"
        }`}
      >
        <span
          className={`absolute top-0.5 left-0.5 w-4 h-4 rounded-full bg-white transition-transform duration-200 ${
            enabled ? "translate-x-4" : ""
          }`}
        />
      </button>
    </div>
  );
}
