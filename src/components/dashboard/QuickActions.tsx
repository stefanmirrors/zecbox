import { useStealthMode } from "../../hooks/useStealthMode";

export function QuickActions() {
  const { status, toggling, toggle } = useStealthMode();

  return (
    <div className="flex items-center gap-6 border border-zec-border rounded-xl px-5 py-3">
      <Toggle
        label="Stealth Mode"
        enabled={status.enabled}
        loading={toggling || status.status === "bootstrapping"}
        onToggle={toggle}
      />
      <div className="w-px h-6 bg-zec-border" />
      <Toggle
        label="Wallet Server"
        enabled={false}
        disabled
      />
    </div>
  );
}

function Toggle({
  label,
  enabled,
  disabled,
  loading,
  onToggle,
}: {
  label: string;
  enabled: boolean;
  disabled?: boolean;
  loading?: boolean;
  onToggle?: () => void;
}) {
  return (
    <div className={`flex items-center gap-3 ${disabled ? "opacity-40" : ""}`}>
      <span className="text-xs text-zec-muted">{label}</span>
      <button
        onClick={onToggle}
        role="switch"
        aria-label={`Toggle ${label}`}
        aria-checked={enabled}
        disabled={disabled || loading}
        className={`relative w-8 h-[18px] rounded-full transition-colors shrink-0 ${
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
          className={`absolute top-[2px] left-[2px] w-3.5 h-3.5 rounded-full bg-white transition-transform duration-200 ${
            enabled ? "translate-x-3.5" : ""
          }`}
        />
      </button>
    </div>
  );
}
