import { NodeStatus } from "./NodeStatus";
import { StatusIndicators } from "./StatusIndicators";
import { NodeStatsPanel } from "./NodeStatsPanel";
import { LiveLogPreview } from "./LiveLogPreview";

import type { View } from "../layout/Sidebar";

export function Dashboard({ onNavigate }: { onNavigate?: (view: View) => void }) {
  return (
    <div className="space-y-6">
      <NodeStatus />
      <StatusIndicators />
      <NodeStatsPanel />
      <LiveLogPreview onViewAll={() => onNavigate?.("logs")} />
    </div>
  );
}
