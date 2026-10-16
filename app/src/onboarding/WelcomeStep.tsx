import { FileSearch, type LucideIcon, ShieldCheck, Sparkles } from "lucide-react";
import { Button } from "../ui/Button";

const POINTS: { Icon: LucideIcon; title: string; detail: string }[] = [
  {
    Icon: Sparkles,
    title: "By meaning, not just by name",
    detail: "“The dentist invoice from March” finds it, even if the file is called scan_0312.pdf.",
  },
  {
    Icon: FileSearch,
    title: "Inside documents, code and pictures",
    detail: "PDFs, Word files, notes and source code, at the page or line that matches.",
  },
  {
    Icon: ShieldCheck,
    title: "Stays on this computer",
    detail: "Your files are read here and never uploaded anywhere.",
  },
];

/** Onboarding's first step: what the app does, before anything is asked. */
export function WelcomeStep({ onStart }: { onStart: () => void }) {
  return (
    <section className="flex flex-col gap-6">
      <div className="flex flex-col gap-2">
        <h1 className="text-2xl font-semibold tracking-[-0.03em]">
          Find any file by what you remember of it
        </h1>
        <p className="text-sm text-muted">
          Describe it the way you remember it, in English or Indonesian, and search brings it up.
        </p>
      </div>
      <ul className="flex flex-col divide-y divide-line rounded-xl border border-line bg-panel/40">
        {POINTS.map(({ Icon, title, detail }) => (
          <li key={title} className="flex items-start gap-3 px-4 py-3">
            <span className="grid size-8 shrink-0 place-items-center rounded-lg border border-line bg-raised text-accent">
              <Icon aria-hidden className="size-4" strokeWidth={1.75} />
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
