import { invoke } from "@tauri-apps/api/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { Settings, type Dashboard, type FailedFile } from "./Settings";
import "./index.css";

const settings = {
  dashboard: () => invoke<Dashboard>("dashboard"),
  failedFiles: () => invoke<FailedFile[]>("failed_files"),
  pause: () => invoke<void>("pause_indexing"),
  resume: () => invoke<void>("resume_indexing"),
};

// The Settings window opens this page with `?view=settings`; the main window without.
const settingsWindow = new URLSearchParams(location.search).get("view") === "settings";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {settingsWindow ? (
      <div className="min-h-full bg-canvas">
        <Settings {...settings} />
      </div>
    ) : (
      <App />
    )}
  </StrictMode>,
);
