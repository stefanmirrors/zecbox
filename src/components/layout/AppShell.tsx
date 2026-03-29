import { useState, useEffect, useRef } from "react";
import { Sidebar, type View } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { Dashboard } from "../dashboard/Dashboard";
import Privacy from "../privacy/Privacy";
import WalletServer from "../wallet/WalletServer";
import NetworkServe from "../network/NetworkServe";
import { LogViewer } from "../logs/LogViewer";
import { Settings } from "../settings/Settings";

const titles: Record<View, string> = {
  dashboard: "Dashboard",
  privacy: "Privacy",
  wallet: "Wallet Server",
  network: "Serve the Network",
  logs: "Logs",
  settings: "Settings",
};

interface AppShellProps {
  onResetToOnboarding: () => void;
}

export function AppShell({ onResetToOnboarding }: AppShellProps) {
  const [activeView, setActiveView] = useState<View>("dashboard");
  const [visible, setVisible] = useState(true);
  const pendingView = useRef<View | null>(null);

  const handleNavigate = (view: View) => {
    if (view === activeView) return;
    setVisible(false);
    pendingView.current = view;
  };

  useEffect(() => {
    if (!visible && pendingView.current) {
      const timer = setTimeout(() => {
        setActiveView(pendingView.current!);
        pendingView.current = null;
        setVisible(true);
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [visible]);

  return (
    <div className="flex h-screen bg-zec-dark overflow-hidden">
      <Sidebar activeView={activeView} onNavigate={handleNavigate} />
      <div className="flex-1 flex flex-col min-w-0">
        <TitleBar title={titles[activeView]} />
        <main
          role="main"
          className={`flex-1 overflow-y-auto px-8 py-6 transition-opacity duration-150 ease-out ${
            visible ? "opacity-100" : "opacity-0"
          }`}
        >
          {activeView === "dashboard" && <Dashboard onNavigate={handleNavigate} />}
          {activeView === "privacy" && <Privacy />}
          {activeView === "wallet" && <WalletServer />}
          {activeView === "network" && <NetworkServe />}
          {activeView === "logs" && <LogViewer />}
          {activeView === "settings" && <Settings onResetToOnboarding={onResetToOnboarding} />}
        </main>
      </div>
    </div>
  );
}
