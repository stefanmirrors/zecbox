import { NodeStatus } from "./NodeStatus";
import { NetworkPanel } from "./NetworkPanel";
import { StoragePanel } from "./StoragePanel";
import { QuickActions } from "./QuickActions";

export function Dashboard() {
  return (
    <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
      <div className="lg:col-span-2">
        <NodeStatus />
      </div>
      <NetworkPanel />
      <StoragePanel />
      <div className="lg:col-span-2">
        <QuickActions />
      </div>
    </div>
  );
}
