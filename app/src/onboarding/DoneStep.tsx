import { spoken } from "./HotkeyStep";

type Props = {
  hotkey: string;
  onFinish: () => void;
};

/** Last onboarding step: search is ready, and how to bring it up. */
export function DoneStep({ hotkey, onFinish }: Props) {
  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold">You are all set</h1>
        <p className="text-sm text-muted">
          File names are searchable now. What is inside your files becomes searchable in the
          background, newest files first.
        </p>
      </div>
      <div className="flex flex-col items-center gap-2 rounded-xl border border-line bg-panel p-6">
        <span className="text-sm text-muted">To search, press</span>
        <kbd className="rounded-lg border border-line bg-raised px-4 py-2 text-lg font-medium">
          {spoken(hotkey)}
        </kbd>
      </div>
      <button
        type="button"
        onClick={onFinish}
        className="self-end rounded-lg bg-accent px-5 py-2 font-medium text-accent-ink transition-colors hover:bg-accent-hover"
      >
        Start searching
      </button>
    </section>
  );
}
