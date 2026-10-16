import { Check, Hourglass, LoaderCircle, TriangleAlert } from "lucide-react";
import type { ReactNode } from "react";
import { gigabytes } from "../onboarding/DownloadStep";
import { timeLeft } from "../onboarding/timeLeft";
import type { Dashboard } from "../Settings";
import { Button } from "../ui/Button";
import { ProgressBar } from "../ui/ProgressBar";

/** Where a step of getting ready stands. */
type Stage = "done" | "active" | "waiting" | "failed";

type Step = {
  name: string;
  stage: Stage;
  detail: string;
  /** Smaller lines under the detail, such as the time left or the file being read. */
  notes?: ReactNode[];
  /** How far an active step has got, from 0 to 1, with what the bar measures. */
  bar?: { label: string; value: number };
  /** What the user can do about it, such as trying again. */
  action?: ReactNode;
};

/**
 * A download the reading model needs: bytes so far of `total`; `secondsLeft`
 * is `null` while the speed is being worked out, and left out where unknown.
 */
export type PipelineDownload = { done: number; total: number; secondsLeft?: number | null };

type Props = {
  status: Dashboard;
  /** Files read per millisecond since reading began; `null` until known. */
  speed?: number | null;
  /** The download the model waits for; by default, the one the status reports. */
  download?: PipelineDownload | null;
  /** That download stopped, and waits to be tried again. */
  downloadStopped?: boolean;
  /** Fetches the reading model again, offered when it could not be loaded. */
  onDownloadModel?: () => void;
};

/**
 * What search goes through before every file can be found by what is inside
 * it: the reading model, the files' names, then their contents.
 */
export function Pipeline({
  status,
  speed = null,
  download = status.download,
  downloadStopped = false,
  onDownloadModel,
}: Props) {
  const steps = [
    modelStep(status, download, downloadStopped, onDownloadModel),
    namesStep(status),
    contentsStep(status, speed, download !== null || downloadStopped),
  ];
  return (
    <ol aria-label="Getting ready" className="flex flex-col">
      {steps.map((step, index) => (
        <StepRow key={step.name} step={step} last={index === steps.length - 1} />
      ))}
    </ol>
  );
}

function modelStep(
  { model }: Dashboard,
  download: PipelineDownload | null,
  stopped: boolean,
  onDownloadModel?: () => void,
): Step {
  const name = "Reading model";
  if (stopped) {
    return {
      name,
      stage: "failed",
      detail: "The download stopped, so reading what is inside files waits",
    };
  }
  if (download) {
    return {
      name,
      stage: "active",
      detail: `Downloading: ${gigabytes(download.done)} of ${gigabytes(download.total)} GB`,
      notes: download.secondsLeft === undefined ? [] : [timeLeft(download.secondsLeft)],
      bar: { label: "Download", value: download.total > 0 ? download.done / download.total : 0 },
    };
  }
  if (model.state === "ready") {
    return { name, stage: "done", detail: `Ready on ${deviceName(model.device)}` };
  }
  if (model.state === "failed") {
    return {
      name,
      stage: "failed",
      detail: "Could not be loaded, so files are found by their names only",
      notes: [<span className="font-mono break-all">{model.reason}</span>],
      action: onDownloadModel && (
        <Button variant="primary" size="sm" onClick={onDownloadModel} className="self-start">
          Download again
        </Button>
      ),
    };
  }
  return { name, stage: "active", detail: "Loading…" };
}

/** Where the model runs, in plain words: `cuda` and `directml:1` are graphics cards. */
export function deviceName(device: string): string {
  return device === "cpu" ? "your processor" : "your graphics card";
}

function namesStep({ files, scanning }: Dashboard): Step {
  if (scanning) return { name: "File names", stage: "active", detail: "Finding files…" };
  return { name: "File names", stage: "done", detail: `${files.toLocaleString()} files found` };
}

function contentsStep(status: Dashboard, speed: number | null, waitsForDownload: boolean): Step {
  const name = "Contents";
  if (status.model.state === "failed") {
    return { name, stage: "waiting", detail: "Waits for the reading model" };
  }
  if (waitsForDownload) {
    return { name, stage: "waiting", detail: "Starts once the download finishes" };
  }
  if (status.model.state === "loading") {
    return { name, stage: "waiting", detail: "Starts once the reading model has loaded" };
  }
  // Only files with content to read count: not online-only files or unknown formats.
  const total = status.indexed + status.pending + status.failed;
  if (status.pending === 0) {
    const detail =
      status.failed > 0
        ? `${status.indexed.toLocaleString()} files read, ${status.failed.toLocaleString()} could not be`
        : `All ${total.toLocaleString()} files read`;
    return { name, stage: "done", detail };
  }
  const notes: ReactNode[] = [];
  if (speed !== null) {
    notes.push(
      <span className="flex justify-between gap-3">
        <span>{`${Math.round(speed * 60_000).toLocaleString()} files a minute`}</span>
        <span>{timeLeft(status.pending / (speed * 1000))}</span>
      </span>,
    );
  }
  if (status.current) {
    notes.push(
      <span className="block truncate font-mono" title={status.current}>
        {fileName(status.current)}
      </span>,
    );
  } else if (status.state) {
    // While nothing is read, say why: indexing waits, or is paused.
    notes.push(status.state);
  }
  if (status.failed > 0) notes.push(`${status.failed.toLocaleString()} could not be read`);
  return {
    name,
    stage: "active",
    detail: `${status.indexed.toLocaleString()} of ${total.toLocaleString()} files read`,
    notes,
    // Files given up on are done with too, so the bar fills once nothing is pending.
    bar: { label: "Files read", value: (status.indexed + status.failed) / total },
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
    <li aria-labelledby={id} className="relative flex gap-3 pb-5 last:pb-0">
      {/* The line down to the next step. */}
      {!last && <span aria-hidden className="absolute top-8 bottom-1.5 left-[13.5px] w-px bg-line" />}
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
        {step.bar && <ProgressBar label={step.bar.label} value={step.bar.value} />}
        {step.notes?.map((note, index) => (
          <span key={index} className="min-w-0 text-xs text-muted/90">
            {note}
          </span>
        ))}
        {step.action && <span className="flex pt-1">{step.action}</span>}
      </span>
    </li>
  );
}

/** The last part of a Windows or POSIX path. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
