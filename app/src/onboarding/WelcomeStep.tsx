import { FileSearch, type LucideIcon, ShieldCheck, Sparkles } from "lucide-react";
import { BrandMark } from "../ui/BrandMark";
import { Button } from "../ui/Button";
import { TONES, type Tone } from "../ui/tones";

const POINTS: { Icon: LucideIcon; tone: Tone; title: string; detail: string }[] = [
  {
    Icon: Sparkles,
    tone: "rose",
    title: "By meaning, not just by name",
    detail: "“The dentist invoice from March” finds it, even if the file is called scan_0312.pdf.",
  },
  {
    Icon: FileSearch,
    tone: "violet",
    title: "Inside documents, code and pictures",
    detail: "PDFs, Word files, notes and source code, at the page or line that matches.",
  },
  {
    Icon: ShieldCheck,
    tone: "teal",
    title: "Stays on this computer",
    detail: "Your files are read here and never uploaded anywhere.",
  },
];

/** Onboarding's first step: what the app does, before anything is asked. */
export function WelcomeStep({ onStart }: { onStart: () => void }) {
  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-4">
        <span className="relative grid size-14 place-items-center">
          <span
            aria-hidden
            className="absolute inset-1 rounded-full bg-violet-500/35 blur-xl motion-safe:animate-pulse"
          />
          <BrandMark className="relative size-14" />
        </span>
        <div className="flex flex-col gap-2">
          <h1 className="text-2xl font-semibold tracking-[-0.03em]">
            Find any file by what you remember of it
          </h1>
          <p className="text-sm text-muted">
            Describe it the way you remember it, in English or Indonesian, and search brings it up.
          </p>
        </div>
      </div>
      <ul className="flex flex-col gap-2">
        {POINTS.map(({ Icon, tone, title, detail }) => (
          <li key={title} className={`destination tone-${tone} flex items-start gap-3 px-4 py-3`}>
            <span
              className={`grid size-9 shrink-0 place-items-center rounded-xl ring-1 ring-inset ${TONES[tone]}`}
            >
              <Icon aria-hidden className="destination-icon size-4.5" strokeWidth={1.75} />
            </span>
            <span className="flex flex-col gap-0.5">
              <span className="text-sm font-medium">{title}</span>
              <span className="text-[13px] text-muted">{detail}</span>
            </span>
          </li>
        ))}
      </ul>
      <Button variant="primary" size="lg" onClick={onStart} className="self-end">
        Get started
      </Button>
    </section>
  );
}
