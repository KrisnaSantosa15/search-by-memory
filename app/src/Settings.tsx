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
  /** Asks the user for a folder and indexes it; false when they picked none. */
  addFolder: () => Promise<boolean>;
  /** Stops indexing `path`; its files leave both indexes. */
  removeFolder: (path: string) => Promise<void>;
  exclusions: () => Promise<ExclusionList>;
  /** Forgets one of the user's Exclusions, or turns off a default one. */
  removeExclusion: (row: ExclusionRow) => Promise<void>;
  /** Turns every Default Exclusion back on. */
  restoreDefaults: () => Promise<void>;
  /** Excludes files and folders whose name matches `pattern`; `*` stands for anything. */
  excludeNames: (pattern: string) => Promise<void>;
  /** Asks the user for a folder and excludes it; false when they picked none. */
  excludeFolder: () => Promise<boolean>;
};

/** One Exclusion; see `ExclusionRow` in `src-tauri`. */
export type ExclusionRow =
  | { kind: "name"; pattern: string; default: boolean }
  | { kind: "folder"; path: string };

export type ExclusionList = {
  rows: ExclusionRow[];
  /** Some Default Exclusion is off, so they can be restored. */
  defaultsTurnedOff: boolean;
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

function FoldersTab(props: FoldersProps) {
  const { folders, addFolder, removeFolder } = props;
  const [list, setList] = useState<IndexedFolder[]>([]);
  // The folder whose removal waits for the user to confirm it.
  const [removing, setRemoving] = useState<string | null>(null);

  const refresh = useCallback(() => folders().then(setList), [folders]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const add = async () => {
    if (await addFolder()) await refresh();
  };

  const remove = async (path: string) => {
    setRemoving(null);
    await removeFolder(path);
    await refresh();
  };

  return (
    <section className="flex flex-col gap-2 p-6">
      <div className="flex items-center justify-between">
        <h2 id="indexed-folders" className="font-semibold">
          Indexed Folders
        </h2>
        <button
          type="button"
          onClick={add}
          className="rounded-lg bg-accent px-4 py-1.5 text-sm font-medium text-canvas transition-colors hover:bg-accent-hover"
        >
          Add folder…
        </button>
      </div>
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
            <span className="ml-auto flex shrink-0 items-center gap-2">
              {removing === path ? (
                <>
                  <span className="text-muted">Its files leave the search.</span>
                  <button
                    type="button"
                    onClick={() => remove(path)}
                    className="rounded-lg bg-accent px-3 py-1 font-medium text-canvas transition-colors hover:bg-accent-hover"
                  >
                    Stop searching it
                  </button>
                  <button
                    type="button"
                    onClick={() => setRemoving(null)}
                    className="rounded-lg px-3 py-1 text-muted transition-colors hover:text-ink"
                  >
                    Keep
                  </button>
                </>
              ) : (
                <button
                  type="button"
                  aria-label={`Remove ${path}`}
                  onClick={() => setRemoving(path)}
                  className="rounded-lg px-3 py-1 text-muted transition-colors hover:text-ink"
                >
                  Remove
                </button>
              )}
            </span>
          </li>
        ))}
      </ul>
      <Exclusions {...props} />
    </section>
  );
}

function Exclusions(props: FoldersProps) {
  const { exclusions, removeExclusion, restoreDefaults, excludeNames, excludeFolder } = props;
  const [list, setList] = useState<ExclusionList>({ rows: [], defaultsTurnedOff: false });
  // A name pattern being typed, not yet excluded.
  const [pattern, setPattern] = useState("");

  const refresh = useCallback(() => exclusions().then(setList), [exclusions]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  // Changes the Exclusions, then lists them again.
  const change = async (action: () => Promise<void>) => {
    await action();
    await refresh();
  };

  return (
    <>
      <div className="mt-4 flex items-center justify-between">
        <h2 id="exclusions" className="font-semibold">
          Exclusions
        </h2>
        {list.defaultsTurnedOff && (
          <button
            type="button"
            onClick={() => change(restoreDefaults)}
            className="rounded-lg border border-line bg-raised px-4 py-1.5 text-sm transition-colors hover:border-accent"
          >
            Restore defaults
          </button>
        )}
      </div>
      <p className="text-sm text-muted">Files here are never opened or searched.</p>
      <ul
        aria-labelledby="exclusions"
        className="flex flex-col divide-y divide-line rounded-xl border border-line bg-panel"
      >
        {list.rows.map((row) => (
          <li key={label(row)} className="flex items-center gap-3 px-4 py-2 text-sm">
            {row.kind === "folder" ? (
              <Path path={row.path} />
            ) : (
              <span className="font-mono">{row.pattern}</span>
            )}
            {row.kind === "name" && row.default && (
              <span className="shrink-0 rounded-full bg-raised px-2 py-0.5 text-xs text-muted">
                Default
              </span>
            )}
            <button
              type="button"
              aria-label={`${isDefault(row) ? "Turn off" : "Remove"} ${label(row)}`}
              onClick={() => change(() => removeExclusion(row))}
              className="ml-auto shrink-0 rounded-lg px-3 py-1 text-muted transition-colors hover:text-ink"
            >
              {isDefault(row) ? "Turn off" : "Remove"}
            </button>
          </li>
        ))}
      </ul>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          const typed = pattern.trim();
          setPattern("");
          change(() => excludeNames(typed));
        }}
        className="flex items-center gap-2"
      >
        <input
          aria-label="Exclude names like"
          placeholder="Exclude names like *.bak or Old Projects"
          value={pattern}
          onChange={(event) => setPattern(event.target.value)}
          className="min-w-0 flex-1 rounded-lg border border-line bg-panel px-3 py-1.5 text-sm placeholder:text-muted"
        />
        <button
          type="submit"
          disabled={!pattern.trim()}
          className="rounded-lg border border-line bg-raised px-4 py-1.5 text-sm transition-colors hover:border-accent disabled:opacity-50 disabled:hover:border-line"
        >
          Exclude
        </button>
        <button
          type="button"
          onClick={async () => {
            if (await excludeFolder()) await refresh();
          }}
          className="rounded-lg border border-line bg-raised px-4 py-1.5 text-sm transition-colors hover:border-accent"
        >
          Exclude a folder…
        </button>
      </form>
    </>
  );
}

function isDefault(row: ExclusionRow): boolean {
  return row.kind === "name" && row.default;
}

/** What an Exclusion row shows: its name pattern or folder. */
function label(row: ExclusionRow): string {
  return row.kind === "name" ? row.pattern : row.path;
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
  const total = indexed + pending;
  // Nothing to index yet is nothing left to do.
  const percent = total === 0 ? 100 : Math.floor((indexed * 100) / total);
  const since =
    readySince === null
      ? ""
      : ` — files since ${new Date(readySince).toLocaleDateString(undefined, {
          month: "long",
          year: "numeric",
        })} are ready`;
  return `Content index ${percent}%${since}`;
}
