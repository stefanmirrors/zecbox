import { useEffect, useRef, useState } from "react";
import { useLogs } from "../../hooks/useLogs";

export function LogViewer() {
  const { lines, clear } = useLogs();
  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
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
        <p className="text-sm text-zec-muted">
          {lines.length > 0
            ? `${lines.length.toLocaleString()} lines`
            : "No log lines yet"}
        </p>
        <div className="flex items-center gap-2">
          {!autoScroll && (
            <button
              onClick={() => {
                setAutoScroll(true);
                if (containerRef.current) {
                  containerRef.current.scrollTop =
                    containerRef.current.scrollHeight;
                }
              }}
              className="text-xs px-3 py-1 rounded bg-zec-yellow/20 text-zec-yellow hover:bg-zec-yellow/30 transition-colors"
            >
              Jump to bottom
            </button>
          )}
          {lines.length > 0 && (
            <button
              onClick={clear}
              className="text-xs px-3 py-1 rounded bg-zec-border text-zec-muted hover:text-zec-text transition-colors"
            >
              Clear
            </button>
          )}
        </div>
      </div>

      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="flex-1 overflow-y-auto rounded-lg bg-zec-dark border border-zec-border p-4 font-mono text-xs leading-5 min-h-0"
      >
        {lines.length === 0 ? (
          <p className="text-zec-muted/60">
            No log lines yet. Start the node to see logs.
          </p>
        ) : (
          lines.map((line, i) => (
            <div key={i} className="text-zec-muted hover:text-zec-text transition-colors">
              {line}
            </div>
          ))
        )}
      </div>
    </div>
  );
}
