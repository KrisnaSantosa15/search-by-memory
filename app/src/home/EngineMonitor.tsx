import { BrainCircuit, FileSearch, ScanText } from "lucide-react";
import { useIndexingProgress } from "../IndexingProgress";
import type { Dashboard } from "../Settings";

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

type Tone = "ok" | "busy" | "waiting" | "failed";

/** One part of the Engine, as the sidebar lists it. */
type Part = {
  Icon: typeof BrainCircuit;
  name: string;
  /** What it is doing, in a few words. */
  doing: string;
  tone: Tone;
  /** How far it has got, where that is known. */
  amount?: string;
};

const thousands = new Intl.NumberFormat("en-US");

function parts(status: Dashboard): Part[] {
  const total = status.indexed + status.pending;
  return [
    { Icon: BrainCircuit, name: "Reading model", ...model(status) },
    {
      Icon: FileSearch,
      name: "File names",
      doing: status.scanning ? "Finding files…" : `${thousands.format(status.files)} files`,
      tone: status.scanning ? "busy" : "ok",
    },
    {
      Icon: ScanText,
      name: "Contents",
      doing:
        status.pending > 0 ? `${thousands.format(status.pending)} to go` : "Every file is read",
      tone: status.pending > 0 ? "busy" : "ok",
      amount: total > 0 ? `${Math.floor((status.indexed * 100) / total)}%` : undefined,
    },
  ];
}

function model(status: Dashboard): Pick<Part, "doing" | "tone"> {
  const { model } = status;
  if (model.state === "failed") return { doing: "Could not be loaded", tone: "failed" };
  if (model.state === "loading") return { doing: "Getting ready…", tone: "busy" };
  const where = model.device === "cpu" ? "the processor" : "the graphics card";
  return { doing: `EmbeddingGemma on ${where}`, tone: "ok" };
}

const DOTS: Record<Tone, string> = {
  ok: "bg-emerald-400",
  busy: "bg-accent motion-safe:animate-pulse",
  waiting: "bg-warning",
  failed: "bg-danger",
};

/** The sidebar's Engine panel: what each part of search is doing, as VoiceStudio lists its engines. */
export function EngineMonitor({ dashboard }: { dashboard: () => Promise<Dashboard> }) {
  const { status } = useIndexingProgress(dashboard, REFRESH_EVERY_MS, Date.now);
  return (
    <ul aria-label="Engine" className="flex flex-col gap-0.5">
      {status &&
        parts(status).map(({ Icon, name, doing, tone, amount }) => (
          <li key={name} className="flex items-center gap-2.5 rounded-[10px] px-2 py-1.5">
            <span className="grid size-7 shrink-0 place-items-center rounded-lg border border-line bg-raised text-muted">
              <Icon aria-hidden className="size-3.5" strokeWidth={1.75} />
            </span>
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-[13px] font-medium">{name}</span>
              <span className="truncate text-[11px] text-muted" title={doing}>
                {doing}
              </span>
            </span>
            {amount && <span className="shrink-0 text-[11px] tabular-nums text-muted">{amount}</span>}
            <span aria-hidden className={`size-1.5 shrink-0 rounded-full ${DOTS[tone]}`} />
          </li>
        ))}
    </ul>
  );
}
