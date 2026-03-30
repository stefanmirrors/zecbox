import { useNodeStatus } from "../../hooks/useNodeStatus";
import { useShieldMode } from "../../hooks/useShieldMode";
import { useStorage } from "../../hooks/useStorage";
import { formatBytes } from "../../lib/format";

type Health = "green" | "yellow" | "red" | "gray";

export function StatusIndicators() {
  const ns = useNodeStatus();
  const shield = useShieldMode();
  const { storageInfo } = useStorage();

  // Sync status
  const syncHealth: Health =
    ns.status === "running" && ns.syncPercentage != null && ns.syncPercentage >= 99.9 ? "green"
    : ns.status === "running" ? "yellow"
    : ns.status === "starting" ? "yellow"
    : ns.status === "error" ? "red"
    : ns.status === "stopped" ? "red"
    : "yellow";
  const syncValue =
    ns.status === "running" && ns.syncPercentage != null && ns.syncPercentage >= 99.9 ? "Synced"
    : ns.status === "running" && ns.syncPercentage != null ? `${ns.syncPercentage.toFixed(1)}%`
    : ns.status === "starting" ? "Starting"
    : ns.status === "stopping" ? "Stopping"
    : ns.status === "error" ? "Error"
    : "Offline";

  // Shield
  const shieldHealth: Health =
    shield.status.status === "active" ? "green"
    : shield.status.status === "bootstrapping" ? "yellow"
    : shield.status.status === "error" || shield.status.status === "interrupted" ? "red"
    : "gray";
  const shieldValue =
    shield.status.status === "active" ? "Active"
    : shield.status.status === "bootstrapping" ? "Connecting"
    : shield.status.status === "error" ? "Error"
    : shield.status.status === "interrupted" ? "Interrupted"
    : "Off";

  // Storage
  const freeBytes = storageInfo?.availableBytes ?? 0;
  const storageHealth: Health =
    !storageInfo ? "gray"
    : storageInfo.warningLevel === "critical" || storageInfo.warningLevel === "paused" ? "red"
    : storageInfo.warningLevel === "warning" ? "yellow"
    : "green";

  return (
    <div className="grid grid-cols-3 gap-2">
      <Card label="Sync" value={syncValue} health={syncHealth} />
      <Card label="Shield" value={shieldValue} health={shieldHealth} />
      <Card label="Storage" value={storageInfo ? formatBytes(freeBytes) : "-"} sub="free" health={storageHealth} />
    </div>
  );
}

function Card({ label, value, sub, health }: { label: string; value: string; sub?: string; health: Health }) {
  const bg =
    health === "green" ? "bg-emerald-400/10 border-emerald-400/20"
    : health === "yellow" ? "bg-yellow-400/10 border-yellow-400/20"
    : health === "red" ? "bg-red-400/10 border-red-400/20"
    : "bg-zec-surface border-zec-border";

  const valueColor =
    health === "green" ? "text-emerald-400"
    : health === "yellow" ? "text-yellow-400"
    : health === "red" ? "text-red-400"
    : "text-zec-muted";

  return (
    <div className={`rounded-xl px-3 py-2 border ${bg}`}>
      <span className="text-[10px] text-zec-muted uppercase tracking-wider block">{label}</span>
      <div className="flex items-baseline gap-1">
        <span className={`text-sm font-bold tabular-nums truncate ${valueColor}`}>{value}</span>
        {sub && <span className="text-[10px] text-zec-muted">{sub}</span>}
      </div>
    </div>
  );
}
