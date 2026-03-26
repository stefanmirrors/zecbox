import { NodeStatus } from "./NodeStatus";
import { NodeStatsPanel } from "./NodeStatsPanel";
import { StorageBar } from "./StorageBar";
import { LiveLogPreview } from "./LiveLogPreview";

import type { View } from "../layout/Sidebar";

export function Dashboard({ onNavigate }: { onNavigate?: (view: View) => void }) {
  return (
    <div className="space-y-6">
      <NodeStatus />
      <NodeStatsPanel />
      <StorageBar />
      <LiveLogPreview onViewAll={() => onNavigate?.("logs")} />
    </div>
  );
}
