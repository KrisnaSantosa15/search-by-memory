import { useCallback, useEffect, useState } from "react";
import { Path } from "./Path";
import { size } from "./size";

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
  /** Bytes the indexes take on disk. */
  sizeOnDisk: number;
};

/** A file whose content could not be indexed. */
export type FailedFile = { path: string; reason: string };

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

/** A folder whose files are indexed; see `IndexedFolder` in `sbm-index`. */
export type IndexedFolder = {
  path: string;
  /** False while the folder's drive is a Disconnected Drive. */
  connected: boolean;
};

type StatusProps = {
  dashboard: () => Promise<Dashboard>;
  failedFiles: () => Promise<FailedFile[]>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
};

type FoldersProps = {
  folders: () => Promise<IndexedFolder[]>;
};

const TABS = ["Status", "Folders"] as const;

/** The Settings window: indexing status, Indexed Folders, and Exclusions. */
export function Settings(props: StatusProps & FoldersProps) {
  const [tab, setTab] = useState<(typeof TABS)[number]>("Status");

  return (
    <div className="flex flex-col">
      <div role="tablist" className="flex gap-1 border-b border-line px-6 pt-4">
        {TABS.map((name) => (
          <button
            key={name}
            type="button"
            role="tab"
            aria-selected={tab === name}
            onClick={() => setTab(name)}
            className={`-mb-px border-b-2 px-3 py-2 text-sm transition-colors ${
              tab === name
                ? "border-accent text-ink"
                : "border-transparent text-muted hover:text-ink"
            }`}
          >
            {name}
          </button>
        ))}
      </div>
      <div role="tabpanel" aria-label={tab}>
        {tab === "Status" ? <StatusTab {...props} /> : <FoldersTab {...props} />}
      </div>
    </div>
  );
}

function FoldersTab({ folders }: FoldersProps) {
  const [list, setList] = useState<IndexedFolder[]>([]);

  useEffect(() => {
    folders().then(setList);
  }, [folders]);

  return (
    <section className="flex flex-col gap-2 p-6">
      <h2 id="indexed-folders" className="font-semibold">
        Indexed Folders
      </h2>
      <ul
        aria-labelledby="indexed-folders"
        className="flex flex-col divide-y divide-line rounded-xl border border-line bg-panel"
      >
        {list.map(({ path, connected }) => (
          <li key={path} className="flex items-center gap-3 px-4 py-2 text-sm">
            <Path path={path} />
            {!connected && (
              <span className="shrink-0 rounded-full bg-raised px-2 py-0.5 text-xs text-muted">
                Drive not connected
              </span>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function StatusTab({ dashboard, failedFiles, pause, resume }: StatusProps) {
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
      <p className="text-sm text-muted">The index takes {size(status.sizeOnDisk)} on disk</p>
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
