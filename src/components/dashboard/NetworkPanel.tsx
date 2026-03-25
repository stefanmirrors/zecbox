import { useNodeStatus } from "../../hooks/useNodeStatus";
import { InfoTip } from "../shared/InfoTip";

export function NetworkPanel() {
  const ns = useNodeStatus();
  const isRunning = ns.status === "running";

  return (
    <div className="border border-zec-border rounded-xl p-5 space-y-4">
      <h3 className="text-xs font-medium text-zec-muted flex items-center gap-1.5">
        Network <InfoTip text="Your node connects to the peer-to-peer Zcash network to download and broadcast transactions. It communicates with other nodes worldwide." />
      </h3>

      <div className="space-y-3">
        <Row
          label="Peers"
          value={isRunning ? String(ns.peerCount ?? 0) : "--"}
          tip="Nodes your computer is directly connected to. They share blocks and transactions with you."
        />
        <Row
          label="Chain"
          value={isRunning ? (ns.chain ?? "main") : "--"}
          tip="The Zcash network you're connected to. 'main' is the real network with real ZEC."
        />
        <Row
          label="Status"
          value={isRunning ? "Connected" : "Offline"}
          dot={isRunning ? "bg-emerald-400" : "bg-zec-muted/40"}
        />
      </div>
    </div>
  );
}

function Row({ label, value, dot, tip }: { label: string; value: string; dot?: string; tip?: string }) {
  return (
    <div className="flex items-center justify-between">
      <span className="text-sm text-zec-muted flex items-center gap-1.5">
        {label}
        {tip && <InfoTip text={tip} />}
      </span>
      <div className="flex items-center gap-2">
        {dot && <span className={`w-1.5 h-1.5 rounded-full ${dot}`} />}
        <span className="text-sm text-zec-text tabular-nums">{value}</span>
      </div>
    </div>
  );
}
