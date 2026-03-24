import { useEffect, useState } from "react";
import { getAppConfig } from "./lib/tauri";
import { Onboarding } from "./components/onboarding/Onboarding";
import { Dashboard } from "./components/dashboard/Dashboard";

function App() {
  const [loading, setLoading] = useState(true);
  const [firstRunComplete, setFirstRunComplete] = useState(false);

  useEffect(() => {
    getAppConfig()
      .then((config) => {
        setFirstRunComplete(config.firstRunComplete);
        setLoading(false);
      })
      .catch(() => {
        setFirstRunComplete(false);
        setLoading(false);
      });
  }, []);

  if (loading) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-zec-dark">
        <h1 className="text-4xl font-bold text-zec-yellow tracking-tight">
          ZecBox
        </h1>
      </div>
    );
  }

  if (!firstRunComplete) {
    return <Onboarding onComplete={() => setFirstRunComplete(true)} />;
  }

  return <Dashboard />;
}

export default App;
