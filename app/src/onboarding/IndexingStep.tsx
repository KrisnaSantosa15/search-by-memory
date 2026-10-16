import { ReadingProgress, useIndexingProgress } from "../IndexingProgress";
import type { Dashboard } from "../Settings";
import { Button } from "../ui/Button";
import { ProgressBar } from "../ui/ProgressBar";
import { gigabytes } from "./DownloadStep";
import { timeLeft } from "./timeLeft";

type Props = {
  progress: () => Promise<Dashboard>;
  /** Milliseconds between looks at the progress. */
  every?: number;
  /** The clock, in milliseconds. */
  now?: () => number;
  /** The download content reading waits for, while it runs. */
  download?: Download | null;
  /** Set when that download stopped; starts it again from where it stopped. */
  onRetryDownload?: () => void;
  onContinue: () => void;
};

/** Onboarding step that shows indexing as it happens. */
export function IndexingStep({
  progress,
  every = 1000,
  now = Date.now,
  download = null,
  onRetryDownload,
  onContinue,
}: Props) {
  const { status, speed } = useIndexingProgress(progress, every, now);

  // Names are searchable once the first pass over the location has ended.
  const namesFound = status !== null && !status.scanning;

  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold tracking-[-0.02em]">Getting your files ready</h1>
        <p className="text-sm text-muted">
          File names come first, then what is inside them, newest files first.
        </p>
      </div>
      <p className="text-sm font-medium">
        {namesFound ? `${status.files.toLocaleString()} files found` : "Finding files…"}
      </p>
      {download && <DownloadProgress {...download} />}
      {onRetryDownload && (
        <div className="flex items-center justify-between gap-4 rounded-xl border border-warning/40 bg-warning/10 p-4">
          <p className="text-sm">
            The download stopped, so what is inside your files waits. Check your internet
            connection; it picks up where it left off.
          </p>
          <Button variant="primary" onClick={onRetryDownload} className="shrink-0">
            Try again
          </Button>
        </div>
      )}
      {namesFound && (
        <ReadingProgress
          status={status}
          speed={speed}
          waitsForDownload={download !== null || onRetryDownload !== undefined}
        />
      )}
      <Button variant="primary" disabled={!namesFound} onClick={onContinue} className="self-end">
        Continue
      </Button>
    </section>
  );
}

/** Bytes on disk so far, out of `total`; `secondsLeft` is `null` until the speed is known. */
export type Download = { done: number; total: number; secondsLeft: number | null };

function DownloadProgress({ done, total, secondsLeft }: Download) {
  return (
    <div className="flex flex-col gap-2 rounded-xl border border-line bg-panel/40 p-4">
      <span className="text-sm">Downloading what search needs to read your files</span>
      <ProgressBar label="Download" value={total > 0 ? done / total : 0} />
      <div className="flex justify-between gap-3 text-xs text-muted">
        <span>{`${gigabytes(done)} of ${gigabytes(total)} GB`}</span>
        <span>{timeLeft(secondsLeft)}</span>
      </div>
    </div>
  );
}
