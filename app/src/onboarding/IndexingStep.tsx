import { useEffect, useState } from "react";
import type { Dashboard } from "../Settings";
import { Button } from "../ui/Button";
import { ProgressBar } from "../ui/ProgressBar";

type Props = {
  progress: () => Promise<Dashboard>;
  /** Milliseconds between looks at the progress. */
  every?: number;
  onContinue: () => void;
};

/** Onboarding step that shows indexing as it happens. */
export function IndexingStep({ progress, every = 1000, onContinue }: Props) {
  const [status, setStatus] = useState<Dashboard | null>(null);

  useEffect(() => {
    let current = true;
    const look = () => progress().then((status) => current && setStatus(status));
    look();
    const timer = setInterval(look, every);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [progress, every]);

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
      {namesFound && <ContentProgress status={status} />}
      <Button variant="primary" disabled={!namesFound} onClick={onContinue} className="self-end">
        Continue
      </Button>
    </section>
  );
}

/** How far reading what is inside the files has got. */
function ContentProgress({ status }: { status: Dashboard }) {
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
      {status.current && (
        <span className="truncate font-mono text-xs text-muted" title={status.current}>
          {fileName(status.current)}
        </span>
      )}
      {status.state && <span className="text-xs text-muted">{status.state}</span>}
    </div>
  );
}

/** The last part of a Windows or POSIX path. */
function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}
