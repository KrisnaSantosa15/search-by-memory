import { useEffect, useState } from "react";
import { timeLeft } from "./onboarding/timeLeft";
import type { Dashboard } from "./Settings";
import { ProgressBar } from "./ui/ProgressBar";

/** A look at the progress, and when it was taken. */
type Look = { status: Dashboard; at: number };

/**
 * The newest indexing status, looked at every `every` milliseconds, and how
 * fast files are read since reading began (files per millisecond; `null` until
 * some were read).
 */
export function useIndexingProgress(
  progress: () => Promise<Dashboard>,
  every: number,
  now: () => number,
): { status: Dashboard | null; speed: number | null; refresh: () => Promise<void> } {
  const [latest, setLatest] = useState<Look | null>(null);
  // The first look once reading began, which the speed is measured from.
  const [first, setFirst] = useState<Look | null>(null);
  const [refresh, setRefresh] = useState<() => Promise<void>>(() => async () => {});

  useEffect(() => {
    let current = true;
    const look = () =>
      progress().then((status) => {
        if (!current) return;
        const look = { status, at: now() };
        setLatest(look);
        const reading = status.current !== null || status.indexed > 0;
        setFirst((first) => first ?? (reading ? look : null));
      });
    setRefresh(() => look);
    look();
    const timer = setInterval(look, every);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [progress, every, now]);

  return { status: latest?.status ?? null, speed: speed(first, latest), refresh };
}

function speed(first: Look | null, latest: Look | null): number | null {
  if (!first || !latest || latest.at <= first.at) return null;
  const read = latest.status.indexed - first.status.indexed;
  return read > 0 ? read / (latest.at - first.at) : null;
}

/** How far reading what is inside the files has got. */
export function ReadingProgress({ status, speed }: { status: Dashboard; speed: number | null }) {
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
      {/* While nothing is read, say why: the model is loading, or indexing waits. */}
      {!status.current && status.state && (
        <span className="text-xs text-muted">{status.state}</span>
      )}
    </div>
  );
}

/** The last part of a Windows or POSIX path. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
