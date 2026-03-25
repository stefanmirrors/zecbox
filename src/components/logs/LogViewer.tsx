import { useEffect, useRef, useState, useMemo } from "react";
import { useLogs } from "../../hooks/useLogs";
import { parseLine, levelStyles, levelDot, type LogLevel, type ParsedLine } from "../../lib/logParser";

export function LogViewer() {
  const { lines, clear, loading } = useLogs();
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [copied, setCopied] = useState(false);
  const [filter, setFilter] = useState<LogLevel | "all">("all");

  const copyLogs = () => {
    navigator.clipboard.writeText(lines.join("\n"));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const parsed = useMemo(() => lines.map(parseLine), [lines]);
  const filtered = useMemo(
    () => filter === "all" ? parsed : parsed.filter((l) => l.level === filter),
    [parsed, filter]
  );

  const errorCount = useMemo(() => parsed.filter((l) => l.level === "error").length, [parsed]);
  const warnCount = useMemo(() => parsed.filter((l) => l.level === "warn").length, [parsed]);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTo({
        top: containerRef.current.scrollHeight,
        behavior: "smooth",
      });
    }
  }, [filtered, autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 50);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <p className="text-xs text-zec-muted">
            {lines.length > 0 ? `${filtered.length.toLocaleString()} lines` : "No logs yet"}
          </p>
          {lines.length > 0 && (
            <div className="flex items-center gap-1">
              <FilterBtn active={filter === "all"} onClick={() => setFilter("all")} label="All" />
              <FilterBtn active={filter === "error"} onClick={() => setFilter("error")} label={`Errors${errorCount > 0 ? ` (${errorCount})` : ""}`} color="text-red-400" />
              <FilterBtn active={filter === "warn"} onClick={() => setFilter("warn")} label={`Warnings${warnCount > 0 ? ` (${warnCount})` : ""}`} color="text-yellow-400" />
            </div>
          )}
        </div>
        <div className="flex items-center gap-2">
          {!autoScroll && (
            <button
              onClick={() => {
                setAutoScroll(true);
                if (containerRef.current) {
                  containerRef.current.scrollTop = containerRef.current.scrollHeight;
                }
              }}
              className="text-[11px] px-2.5 py-1 rounded-lg border border-zec-yellow/20 text-zec-yellow hover:bg-zec-yellow/5 transition-colors"
            >
              Jump to bottom
            </button>
          )}
          {lines.length > 0 && (
            <button
              onClick={copyLogs}
              className="text-[11px] px-2.5 py-1 rounded-lg border border-zec-border text-zec-muted hover:text-zec-text transition-colors"
            >
              {copied ? "Copied" : "Copy"}
            </button>
          )}
          {lines.length > 0 && (
            <button
              onClick={clear}
              className="text-[11px] px-2.5 py-1 rounded-lg border border-zec-border text-zec-muted hover:text-zec-text transition-colors"
            >
              Clear
            </button>
          )}
        </div>
      </div>

      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto rounded-xl border border-zec-border p-3 text-[11px] leading-6 min-h-0 space-y-0"
      >
        {loading ? (
          <p className="text-zec-muted/40 p-2">Loading...</p>
        ) : filtered.length === 0 ? (
          <p className="text-zec-muted/40 p-2">
            {filter !== "all" ? `No ${filter} logs.` : "Start the node to see logs."}
          </p>
        ) : (
          filtered.map((line, i) => (
            <LogLine key={i} line={line} />
          ))
        )}
      </div>
    </div>
  );
}

function LogLine({ line }: { line: ParsedLine }) {
  const [copied, setCopied] = useState(false);
  const handleClick = () => {
    navigator.clipboard.writeText(line.raw);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };
  return (
    <div
      onClick={handleClick}
      className={`flex items-start gap-2 px-2 py-0.5 rounded cursor-pointer ${levelStyles[line.level]} hover:bg-zec-border/20 transition-colors`}
      title="Click to copy"
    >
      <span className={`w-1.5 h-1.5 rounded-full mt-2 shrink-0 ${levelDot[line.level]}`} />
      {line.time && (
        <span className="text-zec-muted/30 shrink-0 font-mono tabular-nums">{line.time}</span>
      )}
      <span className="break-words min-w-0">{copied ? "Copied!" : line.summary}</span>
    </div>
  );
}

function FilterBtn({ active, onClick, label, color }: { active: boolean; onClick: () => void; label: string; color?: string }) {
  return (
    <button
      onClick={onClick}
      className={`text-[10px] px-2 py-0.5 rounded-full transition-colors ${
        active
          ? "bg-zec-border text-zec-text"
          : `border border-zec-border/50 ${color || "text-zec-muted/50"} hover:border-zec-border`
      }`}
    >
      {label}
    </button>
  );
}
