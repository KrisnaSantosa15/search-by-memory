import { PartyPopper } from "lucide-react";
import { Button } from "../ui/Button";
import { keys } from "../ui/KeysPicker";

type Props = {
  hotkey: string;
  onFinish: () => void;
};

/** Last onboarding step: search is ready, and how to bring it up. */
export function DoneStep({ hotkey, onFinish }: Props) {
  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="flex items-center gap-2 text-xl font-semibold tracking-[-0.02em]">
          <PartyPopper aria-hidden className="size-5 text-accent" strokeWidth={1.75} />
          You are all set
        </h1>
        <p className="text-sm text-muted">
          File names are searchable now. What is inside your files becomes searchable in the
          background, newest files first.
        </p>
      </div>
      <div className="relative isolate flex flex-col items-center gap-4 overflow-hidden rounded-2xl border border-accent/30 bg-panel/50 px-6 py-8">
        <span
          aria-hidden
          className="absolute inset-0 -z-10 bg-[radial-gradient(ellipse_at_50%_0%,var(--color-accent-soft),transparent_70%)]"
        />
        <span className="text-sm text-muted">In any app, press</span>
        <span className="flex items-center gap-2">
          {keys(hotkey).map((key, index) => (
            <span key={key} className="flex items-center gap-2">
              {index > 0 && <span className="text-muted">+</span>}
              <kbd className="min-w-12 rounded-lg border border-line bg-raised px-3.5 py-2 text-center font-sans text-lg font-medium text-ink shadow-[inset_0_-2px_0_rgb(0_0_0/0.25)]">
                {key}
              </kbd>
            </span>
          ))}
        </span>
        <span className="text-sm text-muted">and describe the file you remember.</span>
      </div>
      <p className="text-[13px] text-muted">
        Home opens next, so you can follow the reading. It is always one click away in the tray.
      </p>
      <Button variant="primary" size="lg" onClick={onFinish} className="self-end">
        Start searching
      </Button>
    </section>
  );
}
