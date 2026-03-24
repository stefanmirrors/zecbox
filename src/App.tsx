import { useEffect, useState } from "react";
import { getAppConfig } from "./lib/tauri";
import { Onboarding } from "./components/onboarding/Onboarding";
import { AppShell } from "./components/layout/AppShell";

function App() {
  const [loading, setLoading] = useState(true);
  const [firstRunComplete, setFirstRunComplete] = useState(false);
  const [configError, setConfigError] = useState(false);

  useEffect(() => {
    getAppConfig()
      .then((config) => {
        setFirstRunComplete(config.firstRunComplete);
        setLoading(false);
      })
      .catch((e) => {
        console.warn("Failed to load app config:", e);
        setConfigError(true);
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

  if (configError) {
    return (
      <div className="flex min-h-screen items-center justify-center bg-zec-dark px-6">
        <div className="max-w-md text-center space-y-6">
          <h1 className="text-2xl font-bold text-zec-text">
            Configuration Error
          </h1>
          <p className="text-zec-muted">
            Could not load application settings. Starting fresh setup.
          </p>
          <button
            onClick={() => {
              setConfigError(false);
              setFirstRunComplete(false);
            }}
            className="px-6 py-2.5 rounded-lg font-medium bg-zec-yellow text-zec-dark hover:brightness-110 transition-colors"
          >
            Continue to Setup
          </button>
        </div>
      </div>
    );
  }

  if (!firstRunComplete) {
    return <Onboarding onComplete={() => setFirstRunComplete(true)} />;
  }

  return <AppShell />;
}

export default App;
