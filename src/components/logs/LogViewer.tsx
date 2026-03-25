import { useEffect, useRef, useState } from "react";
import { useLogs } from "../../hooks/useLogs";

export function LogViewer() {
  const { lines, clear, loading } = useLogs();
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [copied, setCopied] = useState(false);

  const copyLogs = () => {
    navigator.clipboard.writeText(lines.join("\n"));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTo({
        top: containerRef.current.scrollHeight,
        behavior: "smooth",
      });
    }
  }, [lines, autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    setAutoScroll(scrollHeight - scrollTop - clientHeight < 50);
  };

  return (
    <div className="flex flex-col h-full">
      <div className="flex items-center justify-between mb-3">
        <p className="text-xs text-zec-muted">
          {lines.length > 0 ? `${lines.length.toLocaleString()} lines` : "No logs yet"}
        </p>
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
        className="flex-1 overflow-y-auto rounded-xl border border-zec-border p-4 font-mono text-[11px] leading-5 min-h-0"
      >
        {loading ? (
          <p className="text-zec-muted/40">Loading...</p>
        ) : lines.length === 0 ? (
          <p className="text-zec-muted/40">Start the node to see logs.</p>
        ) : (
          lines.map((line, i) => (
            <div key={i} className="text-zec-muted/70 hover:text-zec-text transition-colors">
              {line}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
