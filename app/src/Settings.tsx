import { useCallback, useEffect, useState } from "react";

/** What the Status tab shows; see `Dashboard` in `src-tauri`. */
export type Dashboard = {
  files: number;
  indexed: number;
  pending: number;
  /** Every file modified after this time (Unix milliseconds) has its content indexed. */
  readySince: number | null;
  /** What indexing is doing, and why it waits, in plain words. */
  state: string;
  /** Stopped by the user, not held back by the computer. */
  paused: boolean;
};

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

type Props = {
  dashboard: () => Promise<Dashboard>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
};

/** The Settings window: indexing status, Indexed Folders, and Exclusions. */
export function Settings({ dashboard, pause, resume }: Props) {
  const [status, setStatus] = useState<Dashboard | null>(null);

  const refresh = useCallback(() => dashboard().then(setStatus), [dashboard]);

  useEffect(() => {
    refresh();
    const every = setInterval(refresh, REFRESH_EVERY_MS);
    return () => clearInterval(every);
  }, [refresh]);

  if (!status) return null;

  const toggle = status.paused ? resume : pause;

  return (
    <section className="flex flex-col gap-1 p-6">
      <h1 className="text-xl font-semibold">{status.files.toLocaleString()} files found</h1>
      <p className="text-sm text-muted">{contentProgress(status)}</p>
      <p className="text-sm">{status.state}</p>
      <button
        type="button"
        onClick={() => toggle().then(refresh)}
        className="self-start rounded-lg border border-line bg-raised px-4 py-1.5 text-sm transition-colors hover:border-accent"
      >
        {status.paused ? "Resume indexing" : "Pause indexing"}
      </button>
    </section>
  );
}

/** `Content index 34% — files since March 2025 are ready`. */
function contentProgress({ indexed, pending, readySince }: Dashboard): string {
  const percent = Math.floor((indexed * 100) / (indexed + pending));
  const since =
    readySince === null
      ? ""
      : ` — files since ${new Date(readySince).toLocaleDateString(undefined, {
          month: "long",
          year: "numeric",
        })} are ready`;
  return `Content index ${percent}%${since}`;
}
