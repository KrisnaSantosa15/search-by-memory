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
      <ul aria-label="Ways to search" className="grid grid-cols-3 gap-3">
        {WAYS.map(({ typed, title, description, Icon, tone }) => (
          <li key={typed} className="flex">
            <button
              type="button"
              onClick={() => onSearch(typed)}
              className={`destination tone-${tone} group flex w-full flex-col items-start gap-3 p-4 text-left`}
            >
              <span className="flex w-full items-start justify-between gap-2">
                <span
                  className={`grid size-10 shrink-0 place-items-center rounded-xl ring-1 ring-inset shadow-[inset_0_1px_0_rgb(255_255_255/10%)] ${TONES[tone]}`}
                >
                  <Icon aria-hidden className="destination-icon size-5" strokeWidth={1.75} />
                </span>
                <code className="truncate rounded-md border border-line bg-raised/70 px-1.5 py-0.5 font-mono text-[11px] text-muted">
                  {typed.trim()}
                </code>
              </span>
              <span className="flex w-full items-end gap-2">
                <span className="flex min-w-0 flex-1 flex-col gap-0.5">
                  <span className="text-sm font-semibold tracking-[-0.015em]">{title}</span>
                  <span className="text-xs leading-relaxed text-muted">{description}</span>
                </span>
                <ArrowRight aria-hidden className="destination-arrow mb-0.5 size-4 shrink-0" />
              </span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
