import { useEffect, useState } from "react";
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
  onContinue: () => void;
};

/** Onboarding step that shows indexing as it happens. */
export function IndexingStep({
  progress,
  every = 1000,
  now = Date.now,
  download = null,
  onContinue,
}: Props) {
  const [latest, setLatest] = useState<Look | null>(null);
  // The first look, which the speed is measured from.
  const [first, setFirst] = useState<Look | null>(null);
  const status = latest?.status ?? null;

  useEffect(() => {
    let current = true;
    const look = () =>
      progress().then((status) => {
        if (!current) return;
        const look = { status, at: now() };
        setLatest(look);
        setFirst((first) => first ?? look);
      });
    look();
    const timer = setInterval(look, every);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [progress, every, now]);

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
      {namesFound && <ContentProgress status={status} speed={speed(first, latest)} />}
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

/** How far reading what is inside the files has got. */
function ContentProgress({ status, speed }: { status: Dashboard; speed: number | null }) {
  // Only files with content to read count: not online-only files or unknown formats.
  const total = status.indexed + status.pending + status.failed;
  return (
    <div className="flex flex-col gap-2 rounded-xl border border-line bg-panel/40 p-4">
      <div className="flex items-baseline justify-between gap-3 text-sm">
        <span>
          {status.indexed.toLocaleString()} of {total.toLocaleString()} files read
        </span>
        {status.failed > 0 && (
          <span className="text-xs text-muted">{status.failed.toLocaleString()} could not be read</span>
        )}
      </div>
      <ProgressBar label="Files read" value={total > 0 ? status.indexed / total : 0} />
      {speed !== null && (
        <div className="flex justify-between gap-3 text-xs text-muted">
          <span>{Math.round(speed * 60_000).toLocaleString()} files a minute</span>
          <span>{timeLeft(status.pending / (speed * 1000))}</span>
        </div>
      )}
      {status.current && (
        <span className="truncate font-mono text-xs text-muted" title={status.current}>
          {fileName(status.current)}
        </span>
      )}
      {status.state && <span className="text-xs text-muted">{status.state}</span>}
    </div>
  );
}

/** A look at the progress, and when it was taken. */
type Look = { status: Dashboard; at: number };

/** Files read per millisecond since the first look; `null` until some were read. */
function speed(first: Look | null, latest: Look | null): number | null {
  if (!first || !latest || latest.at <= first.at) return null;
  const read = latest.status.indexed - first.status.indexed;
  return read > 0 ? read / (latest.at - first.at) : null;
}

/** The last part of a Windows or POSIX path. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
