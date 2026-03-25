import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { InfoTipProvider } from "./components/shared/InfoTip";
import "./styles/globals.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ErrorBoundary>
      <InfoTipProvider>
        <App />
      </InfoTipProvider>
    </ErrorBoundary>
  </React.StrictMode>
);
