import { Cpu, Gauge, type LucideIcon, Zap } from "lucide-react";
import { useState } from "react";
import { Button } from "../ui/Button";

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
        <h1 className="text-xl font-semibold tracking-[-0.02em]">
          How fast should your files be read?
        </h1>
        <p className="text-sm text-muted">You can change this later in Settings.</p>
      </div>
      <div className="flex flex-col divide-y divide-line overflow-hidden rounded-xl border border-line bg-panel/40">
        {choices.map((choice) => {
          const { Icon, title, detail } = describe(choice);
          return (
            <label
              key={choice.id}
              className="flex cursor-pointer items-center gap-3 px-4 py-3 transition-colors hover:bg-hover has-checked:bg-accent-soft"
            >
              <span className="grid size-8 shrink-0 place-items-center rounded-lg border border-line bg-raised text-accent">
                <Icon aria-hidden className="size-4" strokeWidth={1.75} />
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="flex items-center gap-2 text-sm font-medium">
                  {title}
                  {choice.recommended && (
                    <span className="rounded-full bg-accent-soft px-2 py-0.5 text-[11px] font-medium text-accent">
                      Recommended
                    </span>
                  )}
                </span>
                <span className="text-[13px] text-muted">{detail}</span>
              </span>
              <input
                type="radio"
                name="speed"
                className="size-4 shrink-0 cursor-pointer appearance-none rounded-full border border-muted/60 transition-[border] checked:border-[5px] checked:border-accent focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent"
                checked={picked === choice.id}
                onChange={() => setPicked(choice.id)}
              />
            </label>
          );
        })}
      </div>
      <Button variant="primary" type="submit" className="self-end">
        Continue
      </Button>
    </form>
  );
}

function describe(choice: SpeedChoice): { Icon: LucideIcon; title: string; detail: string } {
  const download =
    choice.extraDownload > 0
      ? ` Needs a one-time ${(choice.extraDownload / 1e9).toFixed(1)} GB download.`
      : "";
  if (choice.id === "cpu") {
    return {
      Icon: Cpu,
      title: "Works on any computer",
      detail: "Uses the processor only. Reading your files takes longer.",
    };
  }
  return {
    Icon: choice.id === "cuda" ? Zap : Gauge,
    title: choice.id === "cuda" ? "Fastest" : "Fast",
    detail: `Uses your ${choice.graphicsCard ?? "graphics card"}.${download}`,
  };
}
