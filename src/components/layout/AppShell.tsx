import { useState } from "react";
import { Sidebar, type View } from "./Sidebar";
import { TitleBar } from "./TitleBar";
import { Dashboard } from "../dashboard/Dashboard";
import ShieldMode from "../shield/ShieldMode";
import { LogViewer } from "../logs/LogViewer";
import { Settings } from "../settings/Settings";

const titles: Record<View, string> = {
  dashboard: "Dashboard",
  shield: "Shield Mode",
  logs: "Logs",
  settings: "Settings",
};

export function AppShell() {
  const [activeView, setActiveView] = useState<View>("dashboard");

  return (
    <div className="flex h-screen bg-zec-dark overflow-hidden">
      <Sidebar activeView={activeView} onNavigate={setActiveView} />
      <div className="flex-1 flex flex-col min-w-0">
        <TitleBar title={titles[activeView]} />
        <main className="flex-1 overflow-y-auto p-6">
          {activeView === "dashboard" && <Dashboard />}
          {activeView === "shield" && <ShieldMode />}
          {activeView === "logs" && <LogViewer />}
          {activeView === "settings" && <Settings />}
        </main>
      </div>
    </div>
  );
}
