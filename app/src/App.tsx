import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { Launcher, type ResultRow } from "./Launcher";
import { DoneStep } from "./onboarding/DoneStep";
import { DownloadStep } from "./onboarding/DownloadStep";
import { FoldersStep, type FolderChoice } from "./onboarding/FoldersStep";
import { HotkeyStep } from "./onboarding/HotkeyStep";
import { SpeedStep, type SpeedChoice } from "./onboarding/SpeedStep";

type DownloadProgress = { done: number; total: number; secondsLeft: number | null };

const tryHotkey = (hotkey: string) => invoke<boolean>("try_hotkey", { hotkey });

const launcher = {
  search: (query: string) => invoke<ResultRow[]>("search", { query, view: "file" }),
  open: (path: string) => invoke("open_file", { path }),
  reveal: (path: string) => invoke("reveal_file", { path }),
  copyFile: (path: string) => invoke("copy_file", { path }),
  hide: () => invoke("hide_launcher"),
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
  const [folders, setFolders] = useState<FolderChoice[] | null>(null);
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
          if (complete) return invoke<FolderChoice[]>("folder_choices").then(setFolders);
        })
        .catch(() => setStopped(true));
      return unlisten;
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [choice, attempt]);

  const step = () => {
    if (finished) return <Launcher {...launcher} />;
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
    if (folders) {
      return (
        <FoldersStep
          choices={folders}
          onContinue={(paths) =>
            invoke("start_indexing", { choice, folders: paths }).then(() => setIndexing(true))
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
        />
      );
    }
    return choices && <SpeedStep choices={choices} onContinue={setChoice} />;
  };

  return <main className="mx-auto max-w-xl px-6 py-10">{step()}</main>;
}
