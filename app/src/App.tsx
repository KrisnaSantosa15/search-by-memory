import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { DownloadStep } from "./onboarding/DownloadStep";
import { SpeedStep, type SpeedChoice } from "./onboarding/SpeedStep";

type DownloadProgress = { done: number; total: number; secondsLeft: number | null };

export default function App() {
  const [choices, setChoices] = useState<SpeedChoice[] | null>(null);
  const [choice, setChoice] = useState<string | null>(null);
  const [progress, setProgress] = useState<DownloadProgress>({
    done: 0,
    total: 0,
    secondsLeft: null,
  });

  useEffect(() => {
    invoke<SpeedChoice[]>("speed_choices").then(setChoices);
  }, []);

  useEffect(() => {
    if (!choice) return;
    // Listen before starting, so no progress report is missed.
    const unlisten = listen<DownloadProgress>("setup-progress", (event) =>
      setProgress(event.payload),
    ).then((unlisten) => {
      invoke("prepare", { choice });
      return unlisten;
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, [choice]);

  return (
    <main className="mx-auto max-w-xl px-6 py-10">
      {choice ? (
        <DownloadStep {...progress} />
      ) : (
        choices && <SpeedStep choices={choices} onContinue={setChoice} />
      )}
    </main>
  );
}
