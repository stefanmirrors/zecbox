import { useNodeStatus } from "../../hooks/useNodeStatus";
import { useStorage } from "../../hooks/useStorage";
import { useShieldMode } from "../../hooks/useShieldMode";

export function Settings() {
  const nodeStatus = useNodeStatus();
  const { storageInfo } = useStorage();
  const { status: shieldStatus } = useShieldMode();

  return (
    <div className="max-w-2xl space-y-6">
      <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4">
        <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
          About
        </h3>
        <div className="space-y-3">
          <Row label="Version" value="0.1.0" />
          <Row label="Node Status" value={capitalize(nodeStatus.status)} />
          <Row
            label="Shield Mode"
            value={
              shieldStatus.enabled
                ? "Active (Tor)"
                : capitalize(shieldStatus.status)
            }
          />
          {storageInfo && (
            <Row label="Data Directory" value={storageInfo.dataDir} mono />
          )}
        </div>
      </div>

      <div className="bg-zec-surface border border-zec-border rounded-lg p-6 space-y-4 opacity-50">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium text-zec-muted uppercase tracking-wider">
            Wallet Server
          </h3>
          <span className="text-[10px] px-1.5 py-0.5 rounded bg-zec-border text-zec-muted">
            Coming Soon
          </span>
        </div>
        <p className="text-sm text-zec-muted">
          Enable light wallet connections via gRPC for mobile wallets on your local network.
        </p>
      </div>
    </div>
  );
}

function Row({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-zec-muted">{label}</span>
      <span
        className={`text-sm text-zec-text ${mono ? "font-mono" : ""} truncate max-w-72`}
        title={value}
      >
        {value}
      </span>
    </div>
  );
}

function capitalize(s: string): string {
  return s.charAt(0).toUpperCase() + s.slice(1);
}
