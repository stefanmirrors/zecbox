import { useEffect, useState } from "react";
import { getNodeStats } from "../../lib/tauri";
import type { NodeStats } from "../../lib/types";
import { InfoTip } from "../shared/InfoTip";

function formatUptime(secs: number): string {
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.floor(secs / 60)}m`;
  if (secs < 86400) {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return m > 0 ? `${h}h ${m}m` : `${h}h`;
  }
  const d = Math.floor(secs / 86400);
  const h = Math.floor((secs % 86400) / 3600);
  return h > 0 ? `${d}d ${h}h` : `${d}d`;
}

function formatNumber(n: number): string {
  if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
  if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
  return n.toLocaleString();
}

export function NodeStatsPanel() {
  const [stats, setStats] = useState<NodeStats | null>(null);

  useEffect(() => {
    getNodeStats().then(setStats).catch(() => {});
    const interval = setInterval(() => {
      getNodeStats().then(setStats).catch(() => {});
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  if (!stats) return null;

  return (
    <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
      <Stat
        label="Uptime"
        value={formatUptime(stats.totalUptimeSecs)}
        tip="Total time your node has been running across all sessions."
      />
      <Stat
        label="Blocks Verified"
        value={formatNumber(stats.blocksValidated)}
        tip="Total blocks your node has independently verified. Every block proves the integrity of the Zcash blockchain."
      />
      <Stat
        label="Streak"
        value={`${stats.currentStreakDays}d`}
        sub={stats.bestStreakDays > stats.currentStreakDays ? `Best: ${stats.bestStreakDays}d` : undefined}
        tip="Consecutive days your node has been online. Keep it running to grow your streak!"
      />
      <Stat
        label="Wallets Served"
        value={formatNumber(stats.walletsServed)}
        tip="Number of light wallet connections served by your Zaino wallet server."
      />
    </div>
  );
}

function Stat({ label, value, sub, tip }: { label: string; value: string; sub?: string; tip: string }) {
  return (
    <div className="border border-zec-border rounded-xl p-4">
      <p className="text-xs text-zec-muted flex items-center gap-1.5 mb-2">
        {label} <InfoTip text={tip} />
      </p>
      <p className="text-xl font-bold text-zec-text tabular-nums">{value}</p>
      {sub && <p className="text-[10px] text-zec-muted/50 mt-1">{sub}</p>}
    </div>
  );
}
