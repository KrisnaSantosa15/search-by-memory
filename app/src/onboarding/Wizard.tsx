import { Check, ChevronLeft } from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "../ui/Button";

type Props = {
  steps: string[];
  /** Index of the step on screen. */
  current: number;
  /** Set where going back is safe. */
  onBack?: () => void;
  children: ReactNode;
};

/** Onboarding's frame: the steps on a rail, the current one beside it. */
export function Wizard({ steps, current, onBack, children }: Props) {
  return (
    <div className="grid h-full min-h-0 grid-cols-[200px_minmax(0,1fr)]">
      <nav className="border-r border-line bg-chrome/50 p-3">
        <ol aria-label="Steps" className="flex flex-col gap-0.5">
          {steps.map((step, index) => {
            const done = index < current;
            const here = index === current;
            return (
              <li
                key={step}
                aria-current={here ? "step" : undefined}
                className={`flex h-9 items-center gap-3 rounded-[10px] px-2.5 text-sm ${
                  here ? "bg-hover font-medium text-ink" : done ? "text-ink" : "text-muted"
                }`}
              >
                <span
                  className={`grid size-5 shrink-0 place-items-center rounded-full border text-[11px] tabular-nums ${
                    done
                      ? "border-accent bg-accent text-accent-ink"
                      : here
                        ? "border-accent text-accent"
                        : "border-line"
                  }`}
                >
                  {done ? <Check aria-hidden className="size-3" strokeWidth={2.5} /> : index + 1}
                </span>
                {step}
                {done && <span className="sr-only">Done</span>}
              </li>
            );
          })}
        </ol>
      </nav>
      <div className="min-h-0 overflow-y-auto">
        <div className="mx-auto flex max-w-xl flex-col gap-4 px-8 pt-6 pb-8">
          {children}
          {onBack && (
            <Button variant="ghost" size="sm" onClick={onBack} className="self-start">
              <ChevronLeft aria-hidden />
              Back
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
