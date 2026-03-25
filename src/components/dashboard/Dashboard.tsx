import { NodeStatus } from "./NodeStatus";
import { NodeStatsPanel } from "./NodeStatsPanel";
import { NetworkPanel } from "./NetworkPanel";
import { StoragePanel } from "./StoragePanel";
import { QuickActions } from "./QuickActions";

export function Dashboard() {
  return (
    <div className="space-y-6">
      <NodeStatus />
      <NodeStatsPanel />
      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <NetworkPanel />
        <StoragePanel />
      </div>
      <QuickActions />
    </div>
  );
}
