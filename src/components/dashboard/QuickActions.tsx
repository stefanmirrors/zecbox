import { useShieldMode } from "../../hooks/useShieldMode";

export function QuickActions() {
  const { status, toggling, toggle } = useShieldMode();

  return (
    <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
      <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider mb-4">
        Features
      </h3>
      <div className="flex items-center gap-4">
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
          description="Enable light wallet connections"
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
      className={`flex-1 flex items-center justify-between p-4 rounded-lg border ${
        disabled
          ? "border-zec-border/50 opacity-50"
          : "border-zec-border"
      }`}
    >
      <div>
        <div className="flex items-center gap-2">
          <p className="text-sm font-medium text-zec-text">{label}</p>
          {disabled && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-zec-border text-zec-muted">
              Coming Soon
            </span>
          )}
          {loading && statusText && (
            <span className="text-[10px] px-1.5 py-0.5 rounded bg-zec-yellow/20 text-zec-yellow">
              {statusText}
            </span>
          )}
        </div>
        <p className="text-xs text-zec-muted mt-0.5">{description}</p>
      </div>
      <button
        onClick={onToggle}
        role="switch"
        aria-label={`Toggle ${label}`}
        aria-checked={enabled}
        disabled={disabled || loading}
        className={`relative w-10 h-6 rounded-full transition-colors ${
          disabled || loading
            ? loading
              ? "bg-zec-yellow/30 cursor-wait"
              : "bg-zec-border cursor-not-allowed"
            : enabled
              ? "bg-emerald-500"
              : "bg-zec-border hover:bg-zec-muted/30"
        }`}
      >
        <span
          className={`absolute top-1 left-1 w-4 h-4 rounded-full bg-white transition-transform duration-200 ${
            enabled ? "translate-x-4" : ""
          }`}
        />
      </button>
    </div>
  );
}
