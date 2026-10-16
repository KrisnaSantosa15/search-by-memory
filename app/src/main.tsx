import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { Home } from "./home/Home";
import type {
  AppSettings,
  Dashboard,
  ExclusionList,
  ExclusionRow,
  FailedFile,
  IndexedFolder,
} from "./Settings";
import "@fontsource-variable/inter";
import "./index.css";

const home = {
  dashboard: () => invoke<Dashboard>("dashboard"),
  failedFiles: () => invoke<FailedFile[]>("failed_files"),
  folders: () => invoke<IndexedFolder[]>("indexed_folders"),
  addFolder: () => invoke<boolean>("pick_and_add_folder"),
  removeFolder: (path: string) => invoke<void>("remove_indexed_folder", { path }),
  exclusions: () => invoke<ExclusionList>("exclusions"),
  removeExclusion: (row: ExclusionRow) => invoke<void>("remove_exclusion", { row }),
  restoreDefaults: () => invoke<void>("restore_default_exclusions"),
  excludeNames: (pattern: string) => invoke<void>("exclude_names", { pattern }),
  excludeFolder: () => invoke<boolean>("pick_and_exclude_folder"),
  pause: () => invoke<void>("pause_indexing"),
  resume: () => invoke<void>("resume_indexing"),
  purge: () => invoke<void>("purge_index"),
  close: () => getCurrentWindow().close(),
  retryFailed: () => invoke<void>("retry_failed"),
  settings: () => invoke<AppSettings | null>("settings"),
  openSearch: (typed: string) => invoke<void>("open_search", { typed }),
  changeHotkey: (hotkey: string) => invoke<boolean>("change_hotkey", { hotkey }),
  version: getVersion,
};

// The Home window opens this page with `?view=settings`; the main window without.
const homeWindow = new URLSearchParams(location.search).get("view") === "settings";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {homeWindow ? (
      <div className="h-full surface">
        <Home {...home} />
      </div>
    ) : (
      <App />
    )}
  </StrictMode>,
);
