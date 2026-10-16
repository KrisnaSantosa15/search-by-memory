import { BookOpenCheck, Database, Files } from "lucide-react";
import type { ReactNode } from "react";
import { useIndexingProgress } from "../IndexingProgress";
import type { Dashboard } from "../Settings";
import { size } from "../size";

/** How often the status is asked again while the window is open. */
const REFRESH_EVERY_MS = 2_000;

type Props = {
  dashboard: () => Promise<Dashboard>;
};

/** The Home page: what can be searched now, at a glance. */
export function Overview({ dashboard }: Props) {
  const { status } = useIndexingProgress(dashboard, REFRESH_EVERY_MS, Date.now);
  if (!status) return null;

  return (
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
  );
}

/** Tints for icon tiles, as VoiceStudio gives each destination its own colour. */
export const TONES = {
  sky: "bg-sky-100 text-sky-700 ring-sky-300/60 dark:bg-sky-400/15 dark:text-sky-300 dark:ring-sky-300/30",
  violet:
    "bg-violet-100 text-violet-700 ring-violet-300/60 dark:bg-violet-400/15 dark:text-violet-300 dark:ring-violet-300/30",
  teal: "bg-teal-100 text-teal-700 ring-teal-300/60 dark:bg-teal-400/15 dark:text-teal-300 dark:ring-teal-300/30",
  rose: "bg-rose-100 text-rose-700 ring-rose-300/60 dark:bg-rose-400/15 dark:text-rose-300 dark:ring-rose-300/30",
  amber:
    "bg-amber-100 text-amber-800 ring-amber-300/60 dark:bg-amber-400/15 dark:text-amber-200 dark:ring-amber-300/30",
  fuchsia:
    "bg-fuchsia-100 text-fuchsia-700 ring-fuchsia-300/60 dark:bg-fuchsia-400/15 dark:text-fuchsia-300 dark:ring-fuchsia-300/30",
} as const;

export type Tone = keyof typeof TONES;

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
