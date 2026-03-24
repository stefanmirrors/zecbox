export function QuickActions() {
  return (
    <div className="bg-zec-surface border border-zec-border rounded-lg p-6">
      <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider mb-4">
        Features
      </h3>
      <div className="flex items-center gap-4">
        <FeatureToggle
          label="Shield Mode"
          description="Route traffic through Tor"
          enabled={false}
          disabled
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
}: {
  label: string;
  description: string;
  enabled: boolean;
  disabled?: boolean;
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
        </div>
        <p className="text-xs text-zec-muted mt-0.5">{description}</p>
      </div>
      <button
        disabled={disabled}
        className={`relative w-10 h-6 rounded-full transition-colors ${
          disabled
            ? "bg-zec-border cursor-not-allowed"
            : enabled
              ? "bg-zec-yellow"
              : "bg-zec-border"
        }`}
      >
        <span
          className={`absolute top-1 left-1 w-4 h-4 rounded-full bg-white transition-transform ${
            enabled ? "translate-x-4" : ""
          }`}
        />
      </button>
    </div>
  );
}
