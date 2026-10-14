import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { DownloadStep } from "./onboarding/DownloadStep";
import { FoldersStep, type FolderChoice } from "./onboarding/FoldersStep";
import { SpeedStep, type SpeedChoice } from "./onboarding/SpeedStep";

type DownloadProgress = { done: number; total: number; secondsLeft: number | null };

export default function App() {
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

  useEffect(() => {
    invoke<SpeedChoice[]>("speed_choices").then(setChoices);
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
    if (folders) {
      return (
        <FoldersStep
          choices={folders}
          onContinue={(paths) => invoke("start_indexing", { choice, folders: paths })}
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
