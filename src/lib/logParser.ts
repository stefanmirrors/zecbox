export type LogLevel = "info" | "warn" | "error" | "other";

export interface ParsedLine {
  level: LogLevel;
  time: string;
  summary: string;
  raw: string;
}

export function parseLine(raw: string): ParsedLine {
  let level: LogLevel = "other";
  if (raw.includes("error") || raw.includes("ERROR") || raw.includes("fatal")) level = "error";
  else if (raw.includes("WARN") || raw.includes("warn")) level = "warn";
  else if (raw.includes("INFO")) level = "info";

  const timeMatch = raw.match(/\d{4}-\d{2}-\d{2}T(\d{2}:\d{2}:\d{2})/);
  const time = timeMatch ? timeMatch[1] : "";

  let summary = raw
    .replace(/^\[(stderr|stdout)\]\s*/, "")
    .replace(/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d+Z\s*/, "")
    .replace(/\b\w+::\w+(?:::\w+)*::\s*/g, "")
    .replace(/^(INFO|WARN|ERROR|DEBUG|TRACE)\s+/, "")
    .trim();

  summary = summary
    .replace(/^opening database.*/, "Opening database")
    .replace(/^creating new database.*/, "Creating new database")
    .replace(/^initializing network.*/, "Initializing network")
    .replace(/^initializing verifiers.*/, "Initializing verifiers")
    .replace(/^Starting zebrad.*/, "Starting node")
    .replace(/^spawning.*task$/, (m) => m.charAt(0).toUpperCase() + m.slice(1))
    .replace(/^verified checkpoint range block_count=(\d+) current_range=\(Excluded\(Height\((\d+)\)\), Included\(Height\((\d+)\)\)\)/,
      (_, count, from, to) => `Verified ${Number(count).toLocaleString()} blocks (${Number(from).toLocaleString()} → ${Number(to).toLocaleString()})`)
    .replace(/estimated progress to chain tip sync_percent=([\d.]+)% current_height=Height\((\d+)\).*remaining_sync_blocks=(\d+).*/,
      (_, pct, height, remaining) => `Sync: ${pct}% — block ${Number(height).toLocaleString()} — ${Number(remaining).toLocaleString()} remaining`)
    .replace(/connecting to initial peer set ipv4_peer_count=(\d+) ipv6_peer_count=(\d+)/,
      (_, v4, v6) => `Connecting to ${Number(v4) + Number(v6)} peers`)
    .replace(/finished connecting.*active_initial_peer_count=(\d+)/,
      (_, count) => `Connected to ${count} peers`)
    .replace(/Opened RPC endpoint at (.+)/, "RPC ready at $1")
    .replace(/resolved seed peer IP addresses seed="([^"]+)" remote_ip count=(\d+)/,
      (_, seed, count) => `Found ${count} peers from ${seed}`);

  return { level, time, summary, raw };
}

export const levelStyles: Record<LogLevel, string> = {
  error: "text-red-400/90 bg-red-400/5",
  warn: "text-yellow-400/80",
  info: "text-zec-muted/60",
  other: "text-zec-muted/40",
};

export const levelDot: Record<LogLevel, string> = {
  error: "bg-red-400",
  warn: "bg-yellow-400",
  info: "bg-emerald-400/50",
  other: "bg-zec-muted/20",
};
