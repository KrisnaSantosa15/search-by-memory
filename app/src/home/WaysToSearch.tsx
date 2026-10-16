import {
  ArrowRight,
  CalendarClock,
  CodeXml,
  FileText,
  FolderSearch,
  Image,
  TextCursorInput,
} from "lucide-react";
import { TONES, type Tone } from "../ui/tones";

/** The Filter Prefixes Home suggests, each as the text it starts a search with. */
const WAYS: { typed: string; title: string; description: string; Icon: typeof FileText; tone: Tone }[] = [
  {
    typed: "ext:pdf ",
    title: "Only PDFs",
    description: "Keep to one kind of file, or several.",
    Icon: FileText,
    tone: "rose",
  },
  {
    typed: "date:week ",
    title: "Changed this week",
    description: "Also today, month, year, or 2025-03.",
    Icon: CalendarClock,
    tone: "amber",
  },
  {
    typed: "in:",
    title: "Inside a folder",
    description: "Folders whose name has these words.",
    Icon: FolderSearch,
    tone: "sky",
  },
  {
    typed: "code: ",
    title: "Code only",
    description: "Source files, scripts, and configs.",
    Icon: CodeXml,
    tone: "violet",
  },
  {
    typed: "name: ",
    title: "Names only",
    description: "Match file names, not what is inside.",
    Icon: TextCursorInput,
    tone: "teal",
  },
  {
    typed: "ext:jpg ext:png ",
    title: "Photos",
    description: "Pictures, newest first with date:.",
    Icon: Image,
    tone: "fuchsia",
  },
];

/** Cards that start a search with a Filter Prefix, so people learn they exist. */
export function WaysToSearch({ onSearch }: { onSearch: (typed: string) => void }) {
  return (
    <section aria-labelledby="ways-to-search" className="flex flex-col gap-3">
      <h2 id="ways-to-search" className="px-1 text-sm font-medium">
        Ways to search
      </h2>
      <ul aria-label="Ways to search" className="grid grid-cols-2 gap-3 lg:grid-cols-3">
        {WAYS.map(({ typed, title, description, Icon, tone }) => (
          <li key={typed} className="flex">
            <button
              type="button"
              onClick={() => onSearch(typed)}
              className={`destination tone-${tone} group flex w-full items-center gap-3 p-4 text-left`}
            >
              <span
                className={`grid size-10 shrink-0 place-items-center rounded-xl ring-1 ring-inset shadow-[inset_0_1px_0_rgb(255_255_255/10%)] ${TONES[tone]}`}
              >
                <Icon aria-hidden className="destination-icon size-5" strokeWidth={1.75} />
              </span>
              <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                <span className="flex items-baseline gap-2">
                  <span className="truncate text-sm font-semibold tracking-[-0.015em]">{title}</span>
                  <code className="shrink-0 rounded-md bg-raised px-1.5 font-mono text-[11px] text-muted">
                    {typed.trim()}
                  </code>
                </span>
                <span className="truncate text-xs text-muted">{description}</span>
              </span>
              <ArrowRight aria-hidden className="destination-arrow size-4 shrink-0" />
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
