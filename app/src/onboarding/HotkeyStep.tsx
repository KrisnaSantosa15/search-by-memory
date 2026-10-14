import { useEffect, useState } from "react";

/** The shortcut offered first, in the form the Rust side registers. */
const DEFAULT_HOTKEY = "Alt+Space";

type Props = {
  /** Registers `hotkey` for search; `false` when another app already uses it. */
  tryHotkey: (hotkey: string) => Promise<boolean>;
  onContinue: (hotkey: string) => void;
};

/** Onboarding step that sets the keys which bring up search from anywhere. */
export function HotkeyStep({ tryHotkey, onContinue }: Props) {
  const [free, setFree] = useState<boolean | null>(null);

  useEffect(() => {
    tryHotkey(DEFAULT_HOTKEY).then(setFree);
  }, [tryHotkey]);

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        onContinue(DEFAULT_HOTKEY);
      }}
    >
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold">Search from anywhere</h1>
        <p className="text-sm text-muted">Press these keys in any app to bring up search.</p>
      </div>
      <div className="flex justify-center rounded-xl border border-line bg-panel p-6">
        <kbd className="rounded-lg border border-line bg-raised px-4 py-2 text-lg font-medium">
          {spoken(DEFAULT_HOTKEY)}
        </kbd>
      </div>
      {free === false && (
        <p className="rounded-xl border border-accent bg-accent-soft p-4 text-sm">
          Another app already uses these keys. Press other keys you would like to use.
        </p>
      )}
      <button
        type="submit"
        disabled={!free}
        className="self-end rounded-lg bg-accent px-5 py-2 font-medium text-white transition-colors hover:bg-accent-hover disabled:opacity-40"
      >
        Continue
      </button>
    </form>
  );
}

/** `Alt+Space` as people read it: `Alt + Space`. */
function spoken(hotkey: string): string {
  return hotkey.split("+").join(" + ");
}
