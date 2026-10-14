import { useCallback, useEffect, useState } from "react";
import { Path } from "./Path";

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

/** A file whose content could not be indexed. */
export type FailedFile = { path: string; reason: string };

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

type Props = {
  dashboard: () => Promise<Dashboard>;
  failedFiles: () => Promise<FailedFile[]>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
};

/** The Settings window: indexing status, Indexed Folders, and Exclusions. */
export function Settings({ dashboard, failedFiles, pause, resume }: Props) {
  const [status, setStatus] = useState<Dashboard | null>(null);
  const [failed, setFailed] = useState<FailedFile[]>([]);

  const refresh = useCallback(
    () => Promise.all([dashboard().then(setStatus), failedFiles().then(setFailed)]),
    [dashboard, failedFiles],
  );

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
      {failed.length > 0 && <FailedFiles files={failed} />}
    </section>
  );
}

function FailedFiles({ files }: { files: FailedFile[] }) {
  const count = files.length === 1 ? "1 file" : `${files.length.toLocaleString()} files`;
  return (
    <div className="mt-4 flex flex-col gap-2">
      <h2 className="font-semibold">{count} could not be read</h2>
      <ul className="flex flex-col divide-y divide-line rounded-xl border border-line bg-panel">
        {files.map(({ path, reason }) => (
          <li key={path} className="flex flex-col gap-0.5 px-4 py-2 text-sm">
            <span className="font-medium">{fileName(path)}</span>
            <span className="flex text-xs text-muted">
              <Path path={folderOf(path)} />
            </span>
            <span className="text-muted">{reason}</span>
          </li>
        ))}
      </ul>
    </div>
  );
}

/** The last part of a path, whichever slash it uses. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Everything before the last part of a path. */
function folderOf(path: string): string {
  return path.slice(0, path.length - fileName(path).length).replace(/[\\/]$/, "");
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
