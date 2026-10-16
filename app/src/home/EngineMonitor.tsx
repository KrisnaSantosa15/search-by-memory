import { BrainCircuit, ChevronUp, Cpu, FileSearch, FileWarning, ScanText } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { useIndexingProgress } from "../IndexingProgress";
import type { Dashboard } from "../Settings";
import { DevicePanel, type DeviceUsage } from "./DevicePanel";

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
  /** How far it has got, or what to do about it, where that is known. */
  amount?: string;
  /** The page that shows more of it. */
  page: EnginePage;
};

/** The Home pages a part of the Engine opens. */
export type EnginePage = "Home" | "Activity" | "Folders";

const thousands = new Intl.NumberFormat("en-US");

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
      detail: "Reading waits for it",
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

function parts(status: Dashboard): Part[] {
  const total = status.indexed + status.pending;
  const list: Part[] = [
    { Icon: BrainCircuit, name: "Reading model", page: "Home", ...model(status) },
    {
      Icon: FileSearch,
      name: "File names",
      doing: status.scanning ? "Finding files…" : `${thousands.format(status.files)} files`,
      tone: status.scanning ? "busy" : "ok",
      page: "Folders",
    },
    {
      Icon: ScanText,
      name: "Contents",
      doing:
        status.pending > 0 ? `${thousands.format(status.pending)} to go` : "Every file is read",
      tone: status.pending > 0 ? "busy" : "ok",
      amount: total > 0 ? `${Math.floor((status.indexed * 100) / total)}%` : undefined,
      page: "Activity",
    },
  ];
  if (status.failed > 0) {
    list.push({
      Icon: FileWarning,
      name: "Unreadable files",
      doing: `${thousands.format(status.failed)} could not be read`,
      tone: "failed",
      amount: "Check",
      page: "Activity",
    });
  }
  return list;
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

export type EngineMonitorProps = {
  dashboard: () => Promise<Dashboard>;
  deviceUsage: () => Promise<DeviceUsage>;
};

type Props = EngineMonitorProps & {
  /** Goes to the page that shows more of a part. */
  onOpen: (page: EnginePage) => void;
};

/**
 * The sidebar's Engine panel, as VoiceStudio shows its engines: what each part
 * of search is doing, and how busy This computer is, opened above it.
 */
export function EngineMonitor({ dashboard, deviceUsage, onOpen }: Props) {
  const { status } = useIndexingProgress(dashboard, REFRESH_EVERY_MS, Date.now);
  const [open, setOpen] = useState(false);
  const now = status ? summary(status) : null;
  const dot = DOTS[now?.tone ?? "busy"];
  const panel = useRef<HTMLDivElement>(null);

  // Closes on Escape and on a click anywhere else, as a popover does.
  useEffect(() => {
    if (!open) return;
    const away = (event: PointerEvent) => {
      if (!panel.current?.contains(event.target as Node)) setOpen(false);
    };
    const escape = (event: KeyboardEvent) => event.key === "Escape" && setOpen(false);
    document.addEventListener("pointerdown", away);
    document.addEventListener("keydown", escape);
    return () => {
      document.removeEventListener("pointerdown", away);
      document.removeEventListener("keydown", escape);
    };
  }, [open]);

  return (
    <div ref={panel} className="relative flex flex-col gap-1 rounded-xl border border-line bg-panel/40 p-1.5">
      {open && (
        <div
          role="dialog"
          aria-label="This computer"
          className="absolute bottom-full left-0 z-30 mb-2 flex w-[300px] flex-col gap-3 rounded-2xl border border-line bg-panel p-3 shadow-2xl shadow-black/40"
        >
          <div className="flex items-center gap-2.5">
            <span aria-hidden className={`size-2 shrink-0 rounded-full ${dot}`} />
            <span className="flex min-w-0 flex-1 flex-col">
              <span className="truncate text-sm font-semibold">This computer</span>
              <span className="truncate text-[11px] text-muted">
                Search runs here; nothing leaves it
              </span>
            </span>
            <Cpu aria-hidden className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
          </div>
          <DevicePanel deviceUsage={deviceUsage} open={open} />
        </div>
      )}
      <button
        type="button"
        aria-expanded={open}
        aria-haspopup="dialog"
        onClick={() => setOpen((open) => !open)}
        className="group flex h-8 items-center gap-2 rounded-lg border border-line bg-canvas/40 px-2.5 text-[13px] text-muted transition-colors hover:bg-hover/60 hover:text-ink aria-expanded:bg-hover aria-expanded:text-ink"
      >
        <span aria-hidden className={`size-1.5 shrink-0 rounded-full ${dot}`} />
        <Cpu aria-hidden className="size-3.5 shrink-0" strokeWidth={1.75} />
        <span className="min-w-0 flex-1 truncate text-left">This computer</span>
        <ChevronUp
          aria-hidden
          className="size-3.5 shrink-0 transition-transform group-aria-expanded:rotate-180 motion-reduce:transition-none"
          strokeWidth={1.75}
        />
      </button>
      <div role="status" aria-label="Search status" className="flex flex-col px-2 pt-1.5 pb-1">
        {now && (
          <>
            <span className="flex items-baseline gap-2 text-[13px]">
              <span className="min-w-0 flex-1 truncate font-semibold">{now.title}</span>
              {now.amount && (
                <span className="shrink-0 text-xs font-medium tabular-nums text-accent">{now.amount}</span>
              )}
            </span>
            {(now.detail ?? status?.state) && (
              <span className="truncate text-[11px] text-muted" title={now.detail ?? status?.state}>
                {now.detail ?? status?.state}
              </span>
            )}
          </>
        )}
      </div>
      <ul aria-label="Engine" className="flex flex-col gap-0.5">
        {status &&
          parts(status).map(({ Icon, name, doing, tone, amount, page }) => (
            <li key={name}>
              <button
                type="button"
                onClick={() => onOpen(page)}
                className="flex w-full items-center gap-2.5 rounded-[10px] px-2 py-1.5 text-left transition-colors hover:bg-hover/60"
              >
                <span className="grid size-7 shrink-0 place-items-center rounded-lg border border-line bg-raised text-muted">
                  <Icon aria-hidden className="size-3.5" strokeWidth={1.75} />
                </span>
                <span className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-[13px] font-medium">{name}</span>
                  <span className="truncate text-[11px] text-muted" title={doing}>
                    {doing}
                  </span>
                </span>
                {amount && (
                  <span
                    className={`shrink-0 text-[11px] tabular-nums ${tone === "failed" ? "text-danger" : "text-muted"}`}
                  >
                    {amount}
                  </span>
                )}
                <span aria-hidden className={`size-1.5 shrink-0 rounded-full ${DOTS[tone]}`} />
              </button>
            </li>
          ))}
      </ul>
    </div>
  );
}
