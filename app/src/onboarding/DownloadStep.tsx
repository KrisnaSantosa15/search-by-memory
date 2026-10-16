import { Download, RotateCw, TriangleAlert } from "lucide-react";
import { Button } from "../ui/Button";
import { ProgressBar } from "../ui/ProgressBar";
import { timeLeft } from "./timeLeft";

type Props = {
  /** Bytes on disk so far, out of `total`. */
  done: number;
  total: number;
  /** `null` until the download speed is known. */
  secondsLeft: number | null;
  /** Set when the download stopped; offers to try again from where it stopped. */
  onRetry?: () => void;
  /** Stops the download, keeping what is on disk, to pick another speed. */
  onCancel: () => void;
  /** Goes on with onboarding while the download runs. */
  onContinue: () => void;
};

/** Onboarding step that fetches what search needs, as one progress bar. */
export function DownloadStep({ done, total, secondsLeft, onRetry, onCancel, onContinue }: Props) {
  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold tracking-[-0.02em]">Getting search ready</h1>
        <p className="text-sm text-muted">
          Downloading what search needs to understand your files. This happens once.
        </p>
      </div>
      <div className="flex items-center gap-3 rounded-xl border border-line bg-panel/40 p-4">
        <span className="grid size-9 shrink-0 place-items-center rounded-lg border border-line bg-raised text-accent">
          <Download aria-hidden className="size-4.5" strokeWidth={1.75} />
        </span>
        <div className="flex min-w-0 flex-1 flex-col gap-2">
          <div className="flex items-baseline justify-between gap-3 text-sm">
            <span className="font-medium">{`${gigabytes(done)} of ${gigabytes(total)} GB`}</span>
            <span className="text-[13px] text-muted">{onRetry ? "Stopped" : timeLeft(secondsLeft)}</span>
          </div>
          <ProgressBar label="Download" value={total > 0 ? done / total : 0} />
        </div>
      </div>
      {onRetry && (
        <div className="flex items-center gap-3 rounded-xl border border-accent/40 bg-accent-soft p-4">
          <TriangleAlert aria-hidden className="size-4.5 shrink-0 text-accent" strokeWidth={1.75} />
          <p className="flex-1 text-sm">
            The download stopped. Check your internet connection; it picks up where it left off.
          </p>
          <Button variant="primary" onClick={onRetry}>
            <RotateCw aria-hidden strokeWidth={1.75} />
            Try again
          </Button>
        </div>
      )}
      <div className="flex items-center justify-between gap-3">
        <Button variant="ghost" onClick={onCancel}>
          Cancel
        </Button>
        {!onRetry && (
          <Button variant="primary" onClick={onContinue}>
            Continue
          </Button>
        )}
      </div>
    </section>
  );
}

export function gigabytes(bytes: number): string {
  return (bytes / 1e9).toFixed(1);
}
