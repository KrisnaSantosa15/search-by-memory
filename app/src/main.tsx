import { invoke } from "@tauri-apps/api/core";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import {
  Settings,
  type Dashboard,
  type ExclusionList,
  type ExclusionRow,
  type FailedFile,
  type IndexedFolder,
} from "./Settings";
import "./index.css";

const settings = {
  dashboard: () => invoke<Dashboard>("dashboard"),
  failedFiles: () => invoke<FailedFile[]>("failed_files"),
  folders: () => invoke<IndexedFolder[]>("indexed_folders"),
  addFolder: () => invoke<boolean>("pick_and_add_folder"),
  removeFolder: (path: string) => invoke<void>("remove_indexed_folder", { path }),
  exclusions: () => invoke<ExclusionList>("exclusions"),
  removeExclusion: (row: ExclusionRow) => invoke<void>("remove_exclusion", { row }),
  restoreDefaults: () => invoke<void>("restore_default_exclusions"),
  excludeNames: (pattern: string) => invoke<void>("exclude_names", { pattern }),
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
