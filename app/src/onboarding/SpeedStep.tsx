import { useState } from "react";

/** One way to run the embedding model, as the Rust side offers it. */
export type SpeedChoice = {
  /** The Accelerator's name; sent back when the choice is picked. */
  id: string;
  graphicsCard: string | null;
  recommended: boolean;
  /** Bytes to download before it can be used. */
  extraDownload: number;
};

type Props = {
  choices: SpeedChoice[];
  onContinue: (id: string) => void;
};

/** Onboarding step where the user picks how fast indexing runs, in plain words. */
export function SpeedStep({ choices, onContinue }: Props) {
  const [picked, setPicked] = useState(
    () => (choices.find((choice) => choice.recommended) ?? choices[0])?.id,
  );

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        if (picked) onContinue(picked);
      }}
    >
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold">How fast should your files be read?</h1>
        <p className="text-sm text-muted">You can change this later in Settings.</p>
      </div>
      <div className="flex flex-col gap-2">
        {choices.map((choice) => {
          const { title, detail } = describe(choice);
          return (
            <label
              key={choice.id}
              className="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-panel p-4 transition-colors hover:bg-raised has-checked:border-accent has-checked:bg-accent-soft"
            >
              <input
                type="radio"
                name="speed"
                className="mt-1 accent-accent"
                checked={picked === choice.id}
                onChange={() => setPicked(choice.id)}
              />
              <span className="flex flex-col gap-0.5">
                <span className="font-medium">
                  {title}
                  {choice.recommended && (
                    <span className="ml-2 rounded-md bg-accent-soft px-1.5 py-0.5 text-xs font-medium text-accent">
                      Recommended
                    </span>
                  )}
                </span>
                <span className="text-sm text-muted">{detail}</span>
              </span>
            </label>
          );
        })}
      </div>
      <button
        type="submit"
        className="self-end rounded-lg bg-accent px-5 py-2 font-medium text-canvas transition-colors hover:bg-accent-hover"
      >
        Continue
      </button>
    </form>
  );
}

function describe(choice: SpeedChoice): { title: string; detail: string } {
  const download =
    choice.extraDownload > 0
      ? ` Needs a one-time ${(choice.extraDownload / 1e9).toFixed(1)} GB download.`
      : "";
  if (choice.id === "cpu") {
    return {
      title: "Works on any computer",
      detail: "Uses the processor only. Reading your files takes longer.",
    };
  }
  return {
    title: choice.id === "cuda" ? "Fastest" : "Fast",
    detail: `Uses your ${choice.graphicsCard ?? "graphics card"}.${download}`,
  };
}
