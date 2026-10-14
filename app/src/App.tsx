import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { SpeedStep, type SpeedChoice } from "./onboarding/SpeedStep";

export default function App() {
  const [choices, setChoices] = useState<SpeedChoice[] | null>(null);

  useEffect(() => {
    invoke<SpeedChoice[]>("speed_choices").then(setChoices);
  }, []);

  return (
    <main className="mx-auto max-w-xl px-6 py-10">
      {choices && <SpeedStep choices={choices} onContinue={() => {}} />}
    </main>
  );
}
