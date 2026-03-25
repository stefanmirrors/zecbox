import { useEffect, useRef, useMemo } from "react";
import { useLogs } from "../../hooks/useLogs";
import { parseLine, levelDot, levelStyles } from "../../lib/logParser";

export function LiveLogPreview({ onViewAll }: { onViewAll?: () => void }) {
  const { lines } = useLogs();
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [lines]);

  const recent = useMemo(() => lines.slice(-5).map(parseLine), [lines]);

  if (lines.length === 0) return null;

  return (
    <div className="border border-zec-border rounded-xl p-4 space-y-3">
      <div className="flex items-center justify-between">
        <h3 className="text-xs font-medium text-zec-muted">Live Activity</h3>
        {onViewAll && (
          <button
            onClick={onViewAll}
            className="text-[11px] text-zec-muted/50 hover:text-zec-yellow transition-colors"
          >
            View all logs →
          </button>
        )}
      </div>
      <div ref={containerRef} className="space-y-1">
        {recent.map((line, i) => (
          <div
            key={lines.length - 5 + i}
            className={`flex items-start gap-2 px-2 py-0.5 rounded text-[11px] leading-5 ${levelStyles[line.level]}`}
          >
            <span className={`w-1.5 h-1.5 rounded-full mt-2 shrink-0 ${levelDot[line.level]}`} />
            {line.time && (
              <span className="text-zec-muted/25 shrink-0 font-mono tabular-nums">{line.time}</span>
            )}
            <span className="truncate">{line.summary}</span>
          </div>
        ))}
      </div>
    </div>
  );
}
