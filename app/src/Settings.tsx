import { RotateCw } from "lucide-react";
import { type ReactNode, useCallback, useEffect, useState } from "react";
import { ReadingProgress, useIndexingProgress } from "./IndexingProgress";
import { Path } from "./Path";
import { size } from "./size";
import { UNREADABLE, type Unreadable } from "./unreadable";
import { Button } from "./ui/Button";

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
  /** The names in a new location are still being found. */
  scanning: boolean;
  /** The file whose content is being read right now. */
  current: string | null;
  /** Files whose content could not be read. */
  failed: number;
  /** The model that reads what is inside files. */
  model: ModelState;
  /** The download content reading waits for, while it runs. */
  download: { done: number; total: number } | null;
};

/** Whether the model that reads what is inside files can be used yet; see `ModelRow`. */
export type ModelState =
  | { state: "loading" }
  /** `device` is the Accelerator's name, e.g. `cuda`, `directml`, or `cpu`. */
  | { state: "ready"; device: string }
  | { state: "failed"; reason: string };

/** What onboarding decided; see `Settings` in `src-tauri`. */
export type AppSettings = {
  /** The picked Accelerator's name, e.g. `cuda`. */
  accelerator: string;
  /** The keys that bring up search anywhere, e.g. `Ctrl+Shift+Space`. */
  hotkey: string;
};

/** A file whose content could not be indexed. */
export type FailedFile = { path: string; reason: Unreadable; detail: string };


/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

/** A folder whose files are indexed; see `IndexedFolder` in `sbm-index`. */
export type IndexedFolder = {
  path: string;
  /** False while the folder's drive is a Disconnected Drive. */
  connected: boolean;
};

export type StatusProps = {
  dashboard: () => Promise<Dashboard>;
  failedFiles: () => Promise<FailedFile[]>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  /** Erases the index from disk and starts setup again. */
  purge: () => Promise<void>;
  /** Reads the files that could not be read once more. */
  retryFailed: () => Promise<void>;
};

export type FoldersProps = {
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
  | { kind: "folder"; path: string; default?: boolean };

export type ExclusionList = {
  rows: ExclusionRow[];
  /** Some Default Exclusion is off, so they can be restored. */
  defaultsTurnedOff: boolean;
};

/** A titled group of settings, as VoiceStudio draws them: a quiet glass panel. */
export function Section(props: {
  id?: string;
  title: string;
  action?: ReactNode;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-2">
      <div className="flex min-h-8 items-center justify-between gap-3">
        <h2 id={props.id} className="text-sm font-semibold">
          {props.title}
        </h2>
        {props.action}
      </div>
      {props.description && <p className="text-[13px] text-muted">{props.description}</p>}
      {props.children}
    </section>
  );
}

/** The panel the rows of a Section sit in. */
export const PANEL = "flex flex-col divide-y divide-line/70 rounded-xl border border-line bg-panel/40";

/** The Folders page: Indexed Folders and Exclusions. */
export function FoldersPage(props: FoldersProps) {
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
    <>
      <Section
        id="indexed-folders"
        title="Indexed Folders"
        action={
          <Button variant="primary" size="sm" onClick={add}>
            Add folder…
          </Button>
        }
      >
        <ul aria-labelledby="indexed-folders" className={PANEL}>
          {list.map(({ path, connected }) => (
            <li key={path} className="flex min-h-12 items-center gap-3 px-4 py-2 text-sm">
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
                  <Button
                    variant="danger"
                    onClick={() => remove(path)}
                  >
                    Stop searching it
                  </Button>
                  <Button
                    variant="ghost"
                    onClick={() => setRemoving(null)}
                  >
                    Keep
                  </Button>
                </>
              ) : (
                <Button
                  variant="ghost"
                  aria-label={`Remove ${path}`}
                  onClick={() => setRemoving(path)}
                >
                  Remove
                </Button>
              )}
            </span>
          </li>
        ))}
        </ul>
      </Section>
      <Exclusions {...props} />
    </>
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
    <Section
      id="exclusions"
      title="Exclusions"
      description="Files here are never opened or searched."
      action={
        list.defaultsTurnedOff && (
          <Button variant="outline" size="sm" onClick={() => change(restoreDefaults)}>
            Restore defaults
          </Button>
        )
      }
    >
      <ul aria-labelledby="exclusions" className={`${PANEL} max-h-80 overflow-y-auto`}>
        {list.rows.map((row) => (
          <li key={label(row)} className="flex min-h-11 items-center gap-3 px-4 py-1.5 text-sm">
            {row.kind === "folder" ? (
              <Path path={row.path} />
            ) : (
              <span className="font-mono">{row.pattern}</span>
            )}
            {isDefault(row) && (
              <span className="shrink-0 rounded-full bg-raised px-2 py-0.5 text-xs text-muted">
                Default
              </span>
            )}
            <Button
              variant="ghost"
              aria-label={`${isDefault(row) ? "Turn off" : "Remove"} ${label(row)}`}
              onClick={() => change(() => removeExclusion(row))}
              className="ml-auto shrink-0"
            >
              {isDefault(row) ? "Turn off" : "Remove"}
            </Button>
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
          className="h-8 min-w-0 flex-1 rounded-lg border border-line bg-panel/40 px-3 text-sm placeholder:text-muted"
        />
        <Button
          variant="outline"
          type="submit"
          disabled={!pattern.trim()}
        >
          Exclude
        </Button>
        <Button
          variant="outline"
          onClick={async () => {
            if (await excludeFolder()) await refresh();
          }}
        >
          Exclude a folder…
        </Button>
      </form>
    </Section>
  );
}

function isDefault(row: ExclusionRow): boolean {
  return row.default === true;
}

/** What an Exclusion row shows: its name pattern or folder. */
function label(row: ExclusionRow): string {
  return row.kind === "name" ? row.pattern : row.path;
}

/** The Activity page: reading progress, pausing, and files that could not be read. */
export function ActivityPage({ dashboard, failedFiles, pause, resume, purge, retryFailed }: StatusProps) {
  const { status, speed, refresh: refreshStatus } = useIndexingProgress(
    dashboard,
    REFRESH_EVERY_MS,
    Date.now,
  );
  const [failed, setFailed] = useState<FailedFile[]>([]);

  const refreshFailed = useCallback(() => failedFiles().then(setFailed), [failedFiles]);
  const refresh = () => Promise.all([refreshStatus(), refreshFailed()]);

  useEffect(() => {
    refreshFailed();
    const every = setInterval(refreshFailed, REFRESH_EVERY_MS);
    return () => clearInterval(every);
  }, [refreshFailed]);

  if (!status) return null;

  const toggle = status.paused ? resume : pause;

  return (
    <>
      <section className="flex flex-col gap-1">
        <h1 className="text-2xl font-semibold tracking-[-0.03em]">
          {status.files.toLocaleString()} files found
        </h1>
        <p className="text-sm text-muted">{contentProgress(status)}</p>
        <p className="text-sm text-muted">The index takes {size(status.sizeOnDisk)} on disk</p>
      </section>
      <ReadingProgress status={status} speed={speed} showState={false} />
      <div className={`${PANEL} px-4 py-3`}>
        <div className="flex items-center justify-between gap-3">
          <p className="text-sm">{status.state}</p>
          <Button variant="outline" size="sm" onClick={() => toggle().then(refresh)}>
            {status.paused ? "Resume indexing" : "Pause indexing"}
          </Button>
        </div>
      </div>
      {failed.length > 0 && (
        <FailedFiles files={failed} onRetry={() => retryFailed().then(refresh)} />
      )}
      <DeleteIndex purge={purge} />
    </>
  );
}

/** Erases the index after the user confirms it (FR-06). */
function DeleteIndex({ purge }: { purge: () => Promise<void> }) {
  const [asking, setAsking] = useState(false);

  if (!asking) {
    return (
      <Button
        variant="ghost"
        onClick={() => setAsking(true)}
        className="self-start"
      >
        Delete index…
      </Button>
    );
  }
  return (
    <div className="flex flex-col gap-3 rounded-xl border border-danger/30 bg-danger/5 p-4 text-sm">
      <p>
        This erases everything Search by Memory knows about your files. Your files themselves are
        not touched. Setup starts again.
      </p>
      <span className="flex gap-2">
        <Button
          variant="danger"
          onClick={purge}
        >
          Delete index and start over
        </Button>
        <Button
          variant="ghost"
          onClick={() => setAsking(false)}
        >
          Keep
        </Button>
      </span>
    </div>
  );
}

function FailedFiles({ files, onRetry }: { files: FailedFile[]; onRetry: () => void }) {
  const count = files.length === 1 ? "1 file" : `${files.length.toLocaleString()} files`;
  return (
    <div className="flex flex-col gap-2">
      <div className="flex min-h-8 items-center justify-between gap-3">
        <h2 className="text-sm font-semibold">{count} could not be read</h2>
        <Button variant="outline" size="sm" onClick={onRetry}>
          <RotateCw aria-hidden />
          Try again
        </Button>
      </div>
      <ul className={`${PANEL} max-h-80 overflow-y-auto`}>
        {files.map((file) => (
          <FailedFileRow key={file.path} file={file} />
        ))}
      </ul>
    </div>
  );
}

function FailedFileRow({ file: { path, reason, detail } }: { file: FailedFile }) {
  const [open, setOpen] = useState(false);
  return (
    <li className="flex flex-col gap-0.5 px-4 py-2 text-sm">
      <span className="font-medium">{fileName(path)}</span>
      <span className="flex text-xs text-muted">
        <Path path={folderOf(path)} />
      </span>
      <span className="flex items-baseline gap-2 text-muted">
        {UNREADABLE[reason] ?? UNREADABLE.other}
        <button
          type="button"
          aria-expanded={open}
          onClick={() => setOpen(!open)}
          className="text-xs underline-offset-2 transition-colors hover:text-ink hover:underline"
        >
          Details
        </button>
      </span>
      {open && <span className="font-mono text-xs break-all text-muted">{detail}</span>}
    </li>
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
