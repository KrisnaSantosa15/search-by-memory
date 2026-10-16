import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Launcher, type ResultRow } from "./Launcher";
import type { TextPreview } from "./Lines";
import type { PdfPagePreview } from "./PdfPage";
import { DoneStep } from "./onboarding/DoneStep";
import { DownloadStep } from "./onboarding/DownloadStep";
import { HotkeyStep } from "./onboarding/HotkeyStep";
import { LocationStep } from "./onboarding/LocationStep";
import { SpeedStep, type SpeedChoice } from "./onboarding/SpeedStep";

type DownloadProgress = { done: number; total: number; secondsLeft: number | null };

const tryHotkey = (hotkey: string) => invoke<boolean>("try_hotkey", { hotkey });

const launcher = {
  search: (query: string, onlyCode: boolean) =>
    invoke<ResultRow[]>("search", { query, onlyCode }),
  open: (path: string) => invoke("open_file", { path }),
  reveal: (path: string) => invoke("reveal_file", { path }),
  copyFile: (path: string) => invoke("copy_file", { path }),
  hide: () => invoke("hide_launcher"),
  thumbnail: (path: string, size: number) =>
    invoke<string | null>("thumbnail", { path, size }),
  pdfPage: (path: string, page: number, query: string) =>
    invoke<PdfPagePreview | null>("pdf_page", { path, page, query }),
  previewLines: (path: string, lineStart: number | null, lineEnd: number | null) =>
    invoke<TextPreview | null>("preview_lines", { path, lineStart, lineEnd }),
  // The app's own WebView storage: kept across restarts, on this machine only.
  memory: localStorage,
};

export default function App() {
  // Onboarding finished before: the hotkey brought up the window.
  const [finished, setFinished] = useState(false);
  const [choices, setChoices] = useState<SpeedChoice[] | null>(null);
  const [choice, setChoice] = useState<string | null>(null);
  // Each attempt starts the download again; it resumes where the last one stopped.
  const [attempt, setAttempt] = useState(0);
  const [stopped, setStopped] = useState(false);
  const [progress, setProgress] = useState<DownloadProgress>({
    done: 0,
    total: 0,
    secondsLeft: null,
  });
  // Where to search: the personal folder until the user picks another.
  const [location, setLocation] = useState<string | null>(null);
  const [indexing, setIndexing] = useState(false);
  const [hotkey, setHotkey] = useState<string | null>(null);

  useEffect(() => {
    invoke<object | null>("settings").then((settings) => {
      if (settings) setFinished(true);
      else invoke<SpeedChoice[]>("speed_choices").then(setChoices);
    });
  }, []);

  useEffect(() => {
    if (!choice) return;
    // Listen before starting, so no progress report is missed.
    const unlisten = listen<DownloadProgress>("setup-progress", (event) =>
      setProgress(event.payload),
    ).then((unlisten) => {
      invoke<boolean>("prepare", { choice })
        .then((complete) => {
          if (complete) return invoke<string>("default_location").then(setLocation);
          // Cancelled: what was downloaded stays, and picking again resumes it.
          backToSpeeds();
        })
        .catch(() => setStopped(true));
      return unlisten;
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [choice, attempt]);

  const backToSpeeds = () => {
    setChoice(null);
    setStopped(false);
    setProgress({ done: 0, total: 0, secondsLeft: null });
  };

  const step = () => {
    if (hotkey) {
      return (
        <DoneStep
          hotkey={hotkey}
          onFinish={() =>
            invoke("finish_onboarding", { choice, hotkey }).then(() => setFinished(true))
          }
        />
      );
    }
    if (indexing) return <HotkeyStep tryHotkey={tryHotkey} onContinue={setHotkey} />;
    if (location) {
      return (
        <LocationStep
          defaultLocation={location}
          pickFolder={() => invoke<string | null>("pick_location")}
          onContinue={(location) =>
            invoke("start_indexing", { choice, location }).then(() => setIndexing(true))
          }
        />
      );
    }
    if (choice) {
      return (
        <DownloadStep
          {...progress}
          onRetry={
            stopped
              ? () => {
                  setStopped(false);
                  setAttempt((attempt) => attempt + 1);
                }
              : undefined
          }
          onCancel={() => (stopped ? backToSpeeds() : invoke("cancel_prepare"))}
        />
      );
    }
    return choices && <SpeedStep choices={choices} onContinue={setChoice} />;
  };

  if (finished) {
    return (
      <div className="h-full surface">
        <Launcher {...launcher} />
      </div>
    );
  }
  // The window has no frame: this strip moves it, and quits onboarding.
  return (
    <div className="flex h-full flex-col surface">
      <header data-tauri-drag-region className="flex h-10 shrink-0 items-center justify-between pl-5">
        <span data-tauri-drag-region className="text-xs text-muted">
          Search by Memory
        </span>
        <button
          type="button"
          aria-label="Quit"
          onClick={() => invoke("quit")}
          className="h-10 w-12 text-muted transition-colors hover:bg-raised hover:text-ink"
        >
          ✕
        </button>
      </header>
      <main className="w-full flex-1 overflow-y-auto">
        <div className="mx-auto max-w-xl px-6 pt-4 pb-10">{step()}</div>
      </main>
    </div>
  );
}
