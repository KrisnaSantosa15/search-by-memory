import { useEffect, useState } from "react";

/** What the Status tab shows; see `Dashboard` in `src-tauri`. */
export type Dashboard = {
  files: number;
  indexed: number;
  pending: number;
  /** Every file modified after this time (Unix milliseconds) has its content indexed. */
  readySince: number | null;
};

type Props = {
  dashboard: () => Promise<Dashboard>;
};

/** The Settings window: indexing status, Indexed Folders, and Exclusions. */
export function Settings({ dashboard }: Props) {
  const [status, setStatus] = useState<Dashboard | null>(null);

  useEffect(() => {
    dashboard().then(setStatus);
  }, [dashboard]);

  if (!status) return null;

  return (
    <section className="flex flex-col gap-1 p-6">
      <h1 className="text-xl font-semibold">{status.files.toLocaleString()} files found</h1>
      <p className="text-sm text-muted">{contentProgress(status)}</p>
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
