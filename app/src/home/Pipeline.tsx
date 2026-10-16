import { Check, Hourglass, LoaderCircle, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { gigabytes } from "../onboarding/DownloadStep";
import type { Dashboard } from "../Settings";
import { ProgressBar } from "../ui/ProgressBar";

/** Where a step of getting ready stands. */
type Stage = "done" | "active" | "waiting" | "failed";

type Step = {
  name: string;
  stage: Stage;
  detail: string;
  /** The underlying error, for a step that failed. */
  error?: string;
  /** How far an active step has got, from 0 to 1, with what the bar measures. */
  bar?: { label: string; value: number };
};

/**
 * What search goes through before every file can be found by what is inside
 * it: the reading model, the files' names, then their contents.
 */
export function Pipeline({ status }: { status: Dashboard }) {
  const steps = [modelStep(status), namesStep(status), contentsStep(status)];
  return (
    <ol aria-label="Getting ready" className="flex flex-col">
      {steps.map((step, index) => (
        <StepRow key={step.name} step={step} last={index === steps.length - 1} />
      ))}
    </ol>
  );
}

function modelStep({ download, model }: Dashboard): Step {
  if (model.state === "ready") {
    return { name: "Reading model", stage: "done", detail: `Ready on ${deviceName(model.device)}` };
  }
  if (model.state === "failed") {
    return {
      name: "Reading model",
      stage: "failed",
      detail: "Could not be loaded, so files are found by their names only",
      error: model.reason,
    };
  }
  if (download) {
    return {
      name: "Reading model",
      stage: "active",
      detail: `Downloading: ${gigabytes(download.done)} of ${gigabytes(download.total)} GB`,
      bar: { label: "Download", value: download.total > 0 ? download.done / download.total : 0 },
    };
  }
  return { name: "Reading model", stage: "active", detail: "Loading…" };
}

/** Where the model runs, in plain words: `cuda` and `directml:1` are graphics cards. */
export function deviceName(device: string): string {
  return device === "cpu" ? "your processor" : "your graphics card";
}

function namesStep({ files, scanning }: Dashboard): Step {
  if (scanning) return { name: "File names", stage: "active", detail: "Finding files…" };
  return { name: "File names", stage: "done", detail: `${files.toLocaleString()} files found` };
}

function contentsStep(status: Dashboard): Step {
  if (status.model.state === "failed") {
    return { name: "Contents", stage: "waiting", detail: "Waits for the reading model" };
  }
  if (status.model.state === "loading") {
    const detail = status.download
      ? "Starts once the download finishes"
      : "Starts once the reading model has loaded";
    return { name: "Contents", stage: "waiting", detail };
  }
  const total = status.indexed + status.pending + status.failed;
  return {
    name: "Contents",
    stage: "active",
    detail: `${status.indexed.toLocaleString()} of ${total.toLocaleString()} files read`,
  };
}

const MARKS: Record<Stage, ReactNode> = {
  done: <Check aria-hidden className="size-3.5" strokeWidth={2.5} />,
  active: <LoaderCircle aria-hidden className="size-3.5 motion-safe:animate-spin" strokeWidth={2} />,
  waiting: <Hourglass aria-hidden className="size-3.5" strokeWidth={1.75} />,
  failed: <TriangleAlert aria-hidden className="size-3.5" strokeWidth={2} />,
};

const MARK_STYLES: Record<Stage, string> = {
  done: "border-accent bg-accent text-accent-ink",
  active: "border-accent bg-accent-soft text-accent",
  waiting: "border-dashed border-line bg-raised text-muted",
  failed: "border-danger/50 bg-danger/15 text-danger",
};

function StepRow({ step, last }: { step: Step; last: boolean }) {
  const id = `step-${step.name.replaceAll(" ", "-").toLowerCase()}`;
  return (
    <li aria-labelledby={id} className="relative flex gap-3 pb-4 last:pb-0">
      {/* The line down to the next step. */}
      {!last && <span aria-hidden className="absolute top-7 bottom-1 left-[13px] w-px bg-line" />}
      <span
        className={`z-10 grid size-7 shrink-0 place-items-center rounded-full border ${MARK_STYLES[step.stage]}`}
      >
        {MARKS[step.stage]}
      </span>
      <span className="flex min-w-0 flex-1 flex-col gap-1 pt-0.5">
        <span id={id} className={`text-sm font-medium ${step.stage === "waiting" ? "text-muted" : ""}`}>
          {step.name}
        </span>
        <span className="text-[13px] text-muted">{step.detail}</span>
        {step.error && (
          <span className="font-mono text-xs break-all text-muted/80">{step.error}</span>
        )}
        {step.bar && <ProgressBar label={step.bar.label} value={step.bar.value} />}
      </span>
    </li>
  );
}
