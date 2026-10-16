import { Pipeline } from "../home/Pipeline";
import { useIndexingProgress } from "../IndexingProgress";
import type { Dashboard } from "../Settings";
import { Button } from "../ui/Button";

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

/** What there is to show before the first look at the progress: names are being found. */
const STARTING: Dashboard = {
  files: 0,
  indexed: 0,
  pending: 0,
  readySince: null,
  state: "",
  paused: false,
  sizeOnDisk: 0,
  scanning: true,
  current: null,
  failed: 0,
  model: { state: "loading" },
  download: null,
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
      <div className="rounded-2xl border border-line bg-panel/40 p-5">
        <Pipeline
          status={status ?? STARTING}
          speed={speed}
          download={download}
          downloadStopped={onRetryDownload !== undefined}
        />
      </div>
      {onRetryDownload && (
        <div className="flex items-center justify-between gap-4 rounded-xl border border-warning/40 bg-warning/10 p-4">
          <p className="text-sm">
            Check your internet connection; the download picks up where it left off.
          </p>
          <Button variant="primary" onClick={onRetryDownload} className="shrink-0">
            Try again
          </Button>
        </div>
      )}
      <Button variant="primary" disabled={!namesFound} onClick={onContinue} className="self-end">
        Continue
      </Button>
    </section>
  );
}

/** Bytes on disk so far, out of `total`; `secondsLeft` is `null` until the speed is known. */
export type Download = { done: number; total: number; secondsLeft: number | null };
