type Props = {
  /** Bytes on disk so far, out of `total`. */
  done: number;
  total: number;
  /** `null` until the download speed is known. */
  secondsLeft: number | null;
  /** Set when the download stopped; offers to try again from where it stopped. */
  onRetry?: () => void;
};

/** Onboarding step that fetches what search needs, as one progress bar. */
export function DownloadStep({ done, total, secondsLeft, onRetry }: Props) {
  const percent = total > 0 ? Math.floor((done / total) * 100) : 0;

  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold">Getting search ready</h1>
        <p className="text-sm text-muted">
          Downloading what search needs to understand your files. This happens once.
        </p>
      </div>
      <div className="flex flex-col gap-2 rounded-xl border border-line bg-panel p-4">
        <div
          role="progressbar"
          aria-valuemin={0}
          aria-valuemax={100}
          aria-valuenow={percent}
          className="h-2 overflow-hidden rounded-full bg-raised"
        >
          <div className="h-full rounded-full bg-accent transition-all" style={{ width: `${percent}%` }} />
        </div>
        <div className="flex justify-between text-sm text-muted">
          <span>{`${gigabytes(done)} of ${gigabytes(total)} GB`}</span>
          <span>{timeLeft(secondsLeft)}</span>
        </div>
      </div>
      {onRetry && (
        <div className="flex items-center justify-between gap-4 rounded-xl border border-accent bg-accent-soft p-4">
          <p className="text-sm">
            The download stopped. Check your internet connection; it picks up where it left off.
          </p>
          <button
            type="button"
            onClick={onRetry}
            className="shrink-0 rounded-lg bg-accent px-4 py-2 font-medium text-canvas transition-colors hover:bg-accent-hover"
          >
            Try again
          </button>
        </div>
      )}
    </section>
  );
}

function gigabytes(bytes: number): string {
  return (bytes / 1e9).toFixed(1);
}

function timeLeft(seconds: number | null): string {
  if (seconds === null) return "Working out how long this takes";
  if (seconds < 60) return "Less than a minute left";
  const minutes = Math.round(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const parts = [
    hours > 0 && plural(hours, "hour"),
    minutes % 60 > 0 && plural(minutes % 60, "minute"),
  ].filter(Boolean);
  return `About ${parts.join(" ")} left`;
}

function plural(count: number, unit: string): string {
  return `${count} ${unit}${count === 1 ? "" : "s"}`;
}
