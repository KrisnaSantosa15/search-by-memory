import { BookOpenCheck, Database, Files, Search } from "lucide-react";
import { type ReactNode, useEffect, useState } from "react";
import { useIndexingProgress } from "../IndexingProgress";
import { keys, spoken } from "../ui/KeysPicker";
import type { AppSettings, Dashboard } from "../Settings";
import { size } from "../size";
import { HeroArt } from "./HeroArt";
import { Pipeline } from "./Pipeline";
import { ThisComputer } from "./ThisComputer";
import { WaysToSearch } from "./WaysToSearch";
import { TONES, type Tone } from "../ui/tones";

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

export type OverviewProps = {
  dashboard: () => Promise<Dashboard>;
  /** What onboarding decided, such as the search keys; `null` before it finished. */
  settings: () => Promise<AppSettings | null>;
  /** Brings up the launcher with `typed` in its Query box. */
  openSearch: (typed: string) => void;
};

/** The Home page: what can be searched now, at a glance. */
export function Overview({ dashboard, settings, openSearch }: OverviewProps) {
  const { status } = useIndexingProgress(dashboard, REFRESH_EVERY_MS, Date.now);
  const [hotkey, setHotkey] = useState<string | null>(null);

  useEffect(() => {
    settings().then((saved) => setHotkey(saved?.hotkey ?? null));
  }, [settings]);

  if (!status) return null;

  return (
    <>
    <Hero hotkey={hotkey} onSearch={() => openSearch("")} />
    <ul aria-label="At a glance" className="grid grid-cols-3 gap-3">
      <Tile Icon={Files} label="Files found" tone="sky">
        {status.files.toLocaleString()}
      </Tile>
      <Tile Icon={BookOpenCheck} label="Contents read" tone="violet">
        {`${readPercent(status)}%`}
      </Tile>
      <Tile Icon={Database} label="Index size" tone="teal">
        {size(status.sizeOnDisk)}
      </Tile>
    </ul>
    <div className="grid grid-cols-[minmax(0,3fr)_minmax(0,2fr)] gap-3">
      <section
        aria-labelledby="progress"
        className="flex flex-col gap-4 rounded-2xl border border-line bg-panel/40 p-5"
      >
        <h2 id="progress" className="text-sm font-medium">
          Progress
        </h2>
        <Pipeline status={status} />
      </section>
      <ThisComputer model={status.model} hotkey={hotkey} />
    </div>
    <WaysToSearch onSearch={openSearch} />
    </>
  );
}

/** The page's heading, with a way into search that looks like the launcher's Query box. */
function Hero({ hotkey, onSearch }: { hotkey: string | null; onSearch: () => void }) {
  return (
    <section className="relative isolate flex flex-col gap-5 pt-2">
      <HeroArt />
      <div className="flex flex-col gap-2">
        <h1 className="text-3xl font-semibold tracking-[-0.035em]">Find files by what you remember</h1>
        <p className="max-w-[60ch] text-sm leading-relaxed text-muted">
          Describe a file the way you remember it. Search by Memory reads what is inside your files,
          right here on your computer, and nothing leaves it.
        </p>
      </div>
      <button
        type="button"
        aria-label={
          hotkey ? `Search your files, or press ${spoken(hotkey)}` : "Search your files"
        }
        onClick={onSearch}
        className="group flex h-14 items-center gap-3 rounded-2xl border border-accent/30 bg-panel/90 px-5 backdrop-blur-md text-left shadow-[0_0_0_4px_var(--color-accent-soft)] transition-[border-color,box-shadow] duration-200 hover:border-accent/60 hover:shadow-[0_0_0_6px_var(--color-accent-soft),0_8px_28px_rgb(0_0_0/0.18)]"
      >
        <Search aria-hidden className="size-5 shrink-0 text-accent" strokeWidth={1.75} />
        <span className="flex-1 truncate text-base text-muted/80">Describe the file you remember</span>
        {hotkey && (
          <span className="flex shrink-0 items-center gap-1">
            {keys(hotkey).map((key) => (
              <kbd
                key={key}
                className="min-w-7 rounded-md border border-line bg-raised px-1.5 py-0.5 text-center font-sans text-[11px] font-medium text-ink shadow-[inset_0_-1px_0_rgb(0_0_0/0.25)]"
              >
                {key}
              </kbd>
            ))}
          </span>
        )}
      </button>
    </section>
  );
}


function Tile(props: {
  Icon: typeof Files;
  label: string;
  tone: Tone;
  children: ReactNode;
}) {
  const { Icon } = props;
  return (
    <li className="flex items-center gap-3 rounded-2xl border border-line bg-panel/40 p-4">
      <span
        className={`grid size-10 shrink-0 place-items-center rounded-xl ring-1 ring-inset shadow-[inset_0_1px_0_rgb(255_255_255/10%)] ${TONES[props.tone]}`}
      >
        <Icon aria-hidden className="size-5" strokeWidth={1.75} />
      </span>
      <span className="flex min-w-0 flex-col">
        <span className="truncate text-xl font-semibold tracking-[-0.02em] tabular-nums">
          {props.children}
        </span>
        <span className="text-xs text-muted">{props.label}</span>
      </span>
    </li>
  );
}

/** The share of files with content to read whose content is read, rounded down. */
function readPercent({ indexed, pending }: Dashboard): number {
  const total = indexed + pending;
  // Nothing to read yet is nothing left to do.
  return total === 0 ? 100 : Math.floor((indexed * 100) / total);
}
