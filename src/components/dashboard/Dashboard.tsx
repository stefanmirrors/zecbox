import { NodeStatus } from "./NodeStatus";
import { NodeStatsPanel } from "./NodeStatsPanel";
import { StorageBar } from "./StorageBar";
import { LiveLogPreview } from "./LiveLogPreview";

export function Dashboard({ onNavigate }: { onNavigate?: (view: string) => void }) {
  return (
    <div className="space-y-6">
      <NodeStatus />
      <NodeStatsPanel />
      <StorageBar />
      <LiveLogPreview onViewAll={() => onNavigate?.("logs")} />
    </div>
  );
}
