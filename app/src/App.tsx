import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { type ReactNode, useEffect, useState } from "react";
import { Launcher, type Searched } from "./Launcher";
import type { TextPreview } from "./Lines";
import type { PdfPagePreview } from "./PdfPage";
import type { Dashboard } from "./Settings";
import { DoneStep } from "./onboarding/DoneStep";
import { DownloadStep } from "./onboarding/DownloadStep";
import { HotkeyStep } from "./onboarding/HotkeyStep";
import { type Download, IndexingStep } from "./onboarding/IndexingStep";
import { type Estimate, LocationStep } from "./onboarding/LocationStep";
import { SpeedStep, type SpeedChoice } from "./onboarding/SpeedStep";
import { WelcomeStep } from "./onboarding/WelcomeStep";
import { Wizard } from "./onboarding/Wizard";


/** Onboarding's steps, as the rail names them. */
const STEPS = ["Welcome", "Speed", "Download", "Location", "Indexing", "Search keys", "Done"];

const tryHotkey = (hotkey: string) => invoke<boolean>("try_hotkey", { hotkey });
const estimateLocation = (location: string) => invoke<Estimate>("estimate_location", { location });
const pickLocation = () => invoke<string | null>("pick_location");
const indexingProgress = () => invoke<Dashboard>("dashboard");
const listenPressed = (pressed: () => void) => {
  const stop = listen("search-keys-pressed", pressed);
  return () => {
    stop.then((stop) => stop());
  };
};

const launcher = {
  search: (typed: string) => invoke<Searched>("search", { typed }),
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
  const [started, setStarted] = useState(false);
  const [choices, setChoices] = useState<SpeedChoice[] | null>(null);
  const [choice, setChoice] = useState<string | null>(null);
  // Each attempt starts the download again; it resumes where the last one stopped.
  const [attempt, setAttempt] = useState(0);
  const [stopped, setStopped] = useState(false);
  const [downloaded, setDownloaded] = useState(false);
  const [progress, setProgress] = useState<Download>({
    done: 0,
    total: 0,
    secondsLeft: null,
  });
  // Where to search: the personal folder until the user picks another.
  const [location, setLocation] = useState<string | null>(null);
  const [indexing, setIndexing] = useState(false);
  // The user has seen indexing start and gone on to the search keys.
  const [shown, setShown] = useState(false);
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
    const unlisten = listen<Download>("setup-progress", (event) =>
      setProgress(event.payload),
    ).then((unlisten) => {
      invoke<boolean>("prepare", { choice })
        .then((complete) => {
          if (complete) {
            setDownloaded(true);
            return askLocation();
          }
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

  // The personal folder, unless the user is already past the download.
  const askLocation = () =>
    invoke<string>("default_location").then((home) => setLocation((location) => location ?? home));

  // Starts the download again; it resumes where the last one stopped.
  const retryDownload = () => {
    setStopped(false);
    setAttempt((attempt) => attempt + 1);
  };

  const backToSpeeds = () => {
    setChoice(null);
    setStopped(false);
    setProgress({ done: 0, total: 0, secondsLeft: null });
  };

  /** The step on screen: where it sits on the rail, and how to go back from it. */
  const step = (): { at: number; onBack?: () => void; view: ReactNode } => {
    if (hotkey) {
      return {
        at: 6,
        onBack: () => setHotkey(null),
        view: (
          <DoneStep
            hotkey={hotkey}
            onFinish={() =>
              invoke("finish_onboarding", { choice, hotkey }).then(() => setFinished(true))
            }
          />
        ),
      };
    }
    if (shown) {
      return {
        at: 5,
        onBack: () => setShown(false),
        view: <HotkeyStep tryHotkey={tryHotkey} listenPressed={listenPressed} onContinue={setHotkey} />,
      };
    }
    if (indexing) {
      return {
        at: 4,
        view: (
          <IndexingStep
            progress={indexingProgress}
            download={downloaded || stopped ? null : progress}
            onRetryDownload={stopped ? retryDownload : undefined}
            onContinue={() => setShown(true)}
          />
        ),
      };
    }
    if (location) {
      return {
        at: 3,
        view: (
          <LocationStep
            defaultLocation={location}
            estimate={estimateLocation}
            pickFolder={pickLocation}
            onContinue={(location) =>
              invoke("start_indexing", { choice, location }).then(() => setIndexing(true))
            }
          />
        ),
      };
    }
    if (choice) {
      return {
        at: 2,
        view: (
          <DownloadStep
            {...progress}
            onRetry={stopped ? retryDownload : undefined}
            onCancel={() => (stopped ? backToSpeeds() : invoke("cancel_prepare"))}
            onContinue={askLocation}
          />
        ),
      };
    }
    if (!started) return { at: 0, view: <WelcomeStep onStart={() => setStarted(true)} /> };
    return {
      at: 1,
      onBack: () => setStarted(false),
      view: choices && <SpeedStep choices={choices} onContinue={setChoice} />,
    };
  };

  if (finished) {
    return (
      <div className="h-full surface">
        <Launcher {...launcher} />
      </div>
    );
  }
  const current = step();
  // The window has no frame: this strip moves it, and quits onboarding.
  return (
    <div className="flex h-full flex-col surface">
      <header
        data-tauri-drag-region
        className="flex h-10 shrink-0 items-center justify-between border-b border-line pl-5"
      >
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
      <main className="min-h-0 flex-1">
        <Wizard steps={STEPS} current={current.at} onBack={current.onBack}>
          {current.view}
        </Wizard>
      </main>
    </div>
  );
}
