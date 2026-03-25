import { useState, useEffect, useRef, createContext, useContext } from "react";
import { createPortal } from "react-dom";

const InfoTipContext = createContext<{
  activeId: string | null;
  setActiveId: (id: string | null) => void;
}>({ activeId: null, setActiveId: () => {} });

let idCounter = 0;

export function InfoTipProvider({ children }: { children: React.ReactNode }) {
  const [activeId, setActiveId] = useState<string | null>(null);

  useEffect(() => {
    if (!activeId) return;
    const handleClick = () => setActiveId(null);
    document.addEventListener("click", handleClick, true);
    return () => document.removeEventListener("click", handleClick, true);
  }, [activeId]);

  return (
    <InfoTipContext.Provider value={{ activeId, setActiveId }}>
      {children}
    </InfoTipContext.Provider>
  );
}

interface Props {
  text: string;
}

export function InfoTip({ text }: Props) {
  const idRef = useRef(`infotip-${++idCounter}`);
  const { activeId, setActiveId } = useContext(InfoTipContext);
  const show = activeId === idRef.current;
  const btnRef = useRef<HTMLButtonElement>(null);
  const [pos, setPos] = useState({ top: 0, left: 0 });

  useEffect(() => {
    if (show && btnRef.current) {
      const rect = btnRef.current.getBoundingClientRect();
      setPos({
        top: rect.top - 8,
        left: rect.left + rect.width / 2,
      });
    }
  }, [show]);

  const handleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    setActiveId(show ? null : idRef.current);
  };

  return (
    <span className="inline-flex">
      <button
        ref={btnRef}
        onClick={handleClick}
        className="w-3.5 h-3.5 rounded-full border border-zec-muted/30 text-zec-muted/50 hover:text-zec-muted hover:border-zec-muted/50 transition-colors inline-flex items-center justify-center text-[9px] font-medium leading-none cursor-pointer"
        aria-label="More info"
      >
        ?
      </button>
      {show && createPortal(
        <div
          onClick={(e) => e.stopPropagation()}
          className="fixed w-56 px-3 py-2 rounded-lg border border-zec-border bg-zec-dark text-[11px] text-zec-muted leading-relaxed shadow-lg"
          style={{
            top: pos.top,
            left: pos.left,
            transform: "translate(-50%, -100%)",
            zIndex: 9999,
          }}
        >
          {text}
          <div className="absolute top-full left-1/2 -translate-x-1/2 -mt-px w-2 h-2 rotate-45 border-r border-b border-zec-border bg-zec-dark" />
        </div>,
        document.body
      )}
    </span>
  );
}
