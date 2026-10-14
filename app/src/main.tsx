import { invoke } from "@tauri-apps/api/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { Settings, type Dashboard } from "./Settings";
import "./index.css";

const dashboard = () => invoke<Dashboard>("dashboard");

// The Settings window opens this page with `?view=settings`; the main window without.
const settingsWindow = new URLSearchParams(location.search).get("view") === "settings";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {settingsWindow ? (
      <div className="min-h-full bg-canvas">
        <Settings dashboard={dashboard} />
      </div>
    ) : (
      <App />
    )}
  </StrictMode>,
);
