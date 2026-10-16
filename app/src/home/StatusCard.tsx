import { useIndexingProgress } from "../IndexingProgress";
import type { Dashboard } from "../Settings";

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

type Tone = "ok" | "busy" | "waiting" | "failed";

/** What search is doing, in a few words, with how far it has got where that is known. */
function summary(status: Dashboard): {
  title: string;
  amount?: string;
  /** A second line; by default, what indexing is doing. */
  detail?: string;
  tone: Tone;
} {
  if (status.download) {
    const { done, total } = status.download;
    const percent = total > 0 ? Math.floor((done * 100) / total) : 0;
    return {
      title: "Downloading",
      amount: `${percent}%`,
      detail: "Reading files starts once it finishes",
      tone: "busy",
    };
  }
  if (status.model.state === "failed") return { title: "Names only", tone: "failed" };
  if (status.scanning) return { title: "Finding files", tone: "busy" };
  if (status.paused) return { title: "Paused", tone: "waiting" };
  if (status.model.state === "loading") return { title: "Getting ready", tone: "busy" };
  if (status.pending > 0) {
    const total = status.indexed + status.pending;
    return {
      title: "Reading files",
      amount: `${Math.floor((status.indexed * 100) / total)}%`,
      tone: "busy",
    };
  }
  return { title: "Up to date", tone: "ok" };
}

const DOTS: Record<Tone, string> = {
  ok: "bg-emerald-400",
  busy: "bg-accent motion-safe:animate-pulse",
  waiting: "bg-warning",
  failed: "bg-danger",
};

/** The sidebar's last word: what search is doing, seen from every page. */
export function StatusCard({ dashboard }: { dashboard: () => Promise<Dashboard> }) {
  const { status } = useIndexingProgress(dashboard, REFRESH_EVERY_MS, Date.now);
  const now = status ? summary(status) : null;
  const detail = now?.detail ?? status?.state;
  return (
    <div
      role="status"
      aria-label="Search status"
      className="flex flex-col gap-1 rounded-xl border border-line bg-panel/40 px-3 py-2.5"
    >
      {now && (
        <>
          <span className="flex items-center gap-2 text-xs">
            <span aria-hidden className={`size-2 shrink-0 rounded-full ${DOTS[now.tone]}`} />
            <span className="min-w-0 flex-1 truncate font-medium">{now.title}</span>
            {now.amount && <span className="shrink-0 tabular-nums text-muted">{now.amount}</span>}
          </span>
          {detail && (
            <span className="truncate pl-4 text-[11px] text-muted" title={detail}>
              {detail}
            </span>
          )}
        </>
      )}
    </div>
  );
}
