import { useEffect, useRef, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getLogs } from "../lib/tauri";

const MAX_LINES = 5000;

export function useLogs() {
  const [lines, setLines] = useState<string[]>([]);
  const linesRef = useRef<string[]>([]);

  useEffect(() => {
    getLogs()
      .then((initial) => {
        linesRef.current = initial;
        setLines(initial);
      })
      .catch(() => {});

    const unlisten = listen<string>("log_line", (event) => {
      const updated = [...linesRef.current, event.payload];
      if (updated.length > MAX_LINES) {
        updated.splice(0, updated.length - MAX_LINES);
      }
      linesRef.current = updated;
      setLines(updated);
    });

    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const clear = useCallback(() => {
    linesRef.current = [];
    setLines([]);
  }, []);

  return { lines, clear };
}
