export type View = "dashboard" | "shield" | "wallet" | "logs" | "settings";

interface NavItem {
  id: View | string;
  label: string;
  icon: React.ReactNode;
  disabled?: boolean;
  comingSoon?: boolean;
}

interface Props {
  activeView: View;
  onNavigate: (view: View) => void;
}

const navItems: NavItem[] = [
  {
    id: "dashboard",
    label: "Dashboard",
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="1" y="1" width="7" height="7" rx="1" />
        <rect x="10" y="1" width="7" height="7" rx="1" />
        <rect x="1" y="10" width="7" height="7" rx="1" />
        <rect x="10" y="10" width="7" height="7" rx="1" />
      </svg>
    ),
  },
  {
    id: "shield",
    label: "Shield Mode",
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <path d="M9 1.5L2 4.5v4.5c0 4.1 3 7.3 7 8.5 4-1.2 7-4.4 7-8.5V4.5z" />
      </svg>
    ),
  },
  {
    id: "wallet",
    label: "Wallet Server",
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <rect x="1.5" y="4" width="15" height="11" rx="2" />
        <path d="M1.5 8h15" />
        <circle cx="13" cy="11.5" r="1" />
      </svg>
    ),
  },
  {
    id: "logs",
    label: "Logs",
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <polyline points="4 4 14 4" />
        <polyline points="4 9 14 9" />
        <polyline points="4 14 10 14" />
      </svg>
    ),
  },
  {
    id: "settings",
    label: "Settings",
    icon: (
      <svg width="18" height="18" viewBox="0 0 18 18" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round">
        <circle cx="9" cy="9" r="2.5" />
        <path d="M7.5 1.5h3l.5 2.2a6 6 0 011.5.9l2.1-.7 1.5 2.6-1.6 1.5a6 6 0 010 1.8l1.6 1.5-1.5 2.6-2.1-.7a6 6 0 01-1.5.9l-.5 2.2h-3l-.5-2.2a6 6 0 01-1.5-.9l-2.1.7-1.5-2.6 1.6-1.5a6 6 0 010-1.8L1.9 6.5l1.5-2.6 2.1.7a6 6 0 011.5-.9z" />
      </svg>
    ),
  },
];

const comingSoonItems: NavItem[] = [];

export function Sidebar({ activeView, onNavigate }: Props) {
  return (
    <aside className="flex flex-col w-56 bg-zec-surface border-r border-zec-border h-screen shrink-0">
      <div className="px-5 py-5">
        <h1 className="text-xl font-bold text-zec-yellow tracking-tight">
          ZecBox
        </h1>
      </div>

      <nav className="flex-1 px-3 space-y-1">
        {navItems.map((item) => {
          const isActive = activeView === item.id;
          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id as View)}
              className={`w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm font-medium transition-colors ${
                isActive
                  ? "bg-zec-yellow/10 text-zec-yellow"
                  : "text-zec-muted hover:text-zec-text hover:bg-zec-dark/50"
              }`}
            >
              {item.icon}
              {item.label}
            </button>
          );
        })}

        {comingSoonItems.length > 0 && (
          <>
            <div className="pt-4 pb-2">
              <p className="px-3 text-xs text-zec-muted/60 uppercase tracking-wider">
                Coming Soon
              </p>
            </div>

            {comingSoonItems.map((item) => (
              <div
                key={item.id}
                className="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-sm text-zec-muted/40 cursor-default"
              >
                {item.icon}
                {item.label}
              </div>
            ))}
          </>
        )}
      </nav>

      <div className="px-5 py-4 border-t border-zec-border">
        <p className="text-xs text-zec-muted/60">v0.1.0</p>
      </div>
    </aside>
  );
}
