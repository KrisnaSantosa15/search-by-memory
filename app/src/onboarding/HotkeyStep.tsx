import { type KeyboardEvent, useEffect, useState } from "react";
import { Button } from "../ui/Button";

/** The shortcut offered first, in the form the Rust side registers. */
const DEFAULT_HOTKEY = "Ctrl+Shift+Space";

type Props = {
  /** Registers `hotkey` for search; `false` when another app already uses it. */
  tryHotkey: (hotkey: string) => Promise<boolean>;
  /** Calls `pressed` each time the registered keys are pressed; returns how to stop. */
  listenPressed?: (pressed: () => void) => () => void;
  onContinue: (hotkey: string) => void;
};

/** Onboarding step that sets the keys which bring up search from anywhere. */
export function HotkeyStep({ tryHotkey, listenPressed, onContinue }: Props) {
  const [hotkey, setHotkey] = useState(DEFAULT_HOTKEY);
  const [free, setFree] = useState<boolean | null>(null);
  // The user pressed the keys and search heard them.
  const [works, setWorks] = useState(false);

  useEffect(() => listenPressed?.(() => setWorks(true)), [listenPressed]);

  useEffect(() => {
    let current = true;
    setFree(null);
    setWorks(false);
    tryHotkey(hotkey).then((free) => current && setFree(free));
    return () => {
      current = false;
    };
  }, [tryHotkey, hotkey]);

  const record = (event: KeyboardEvent) => {
    const pressed = shortcut(event);
    if (!pressed) return;
    event.preventDefault();
    setHotkey(pressed);
  };

  return (
    <form
     className="flex flex-col gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        onContinue(hotkey);
      }}
    >
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold">Search from anywhere</h1>
        <p className="text-sm text-muted">
          Press these keys in any app to bring up search. To use other keys, click below and
          press them.
        </p>
      </div>
      <button
        type="button"
        aria-label="Keys for search"
        onKeyDown={record}
       className="flex justify-center rounded-xl border border-line bg-panel p-6 transition-colors hover:bg-raised focus:border-accent focus:outline-none"
      >
        <kbd className="rounded-lg border border-line bg-raised px-4 py-2 text-lg font-medium">
          {spoken(hotkey)}
        </kbd>
      </button>
      {free && (
        <p className={`text-sm ${works ? "text-ink" : "text-muted"}`}>
          {works ? "They work." : "Try them now: press the keys."}
        </p>
      )}
      {free === false && (
        <p className="rounded-xl border border-accent bg-accent-soft p-4 text-sm">
          Another app already uses these keys. Press other keys you would like to use.
        </p>
      )}
      <Button
        variant="primary"
        type="submit"
        disabled={!free}
        className="self-end"
      >
        Continue
      </Button>
    </form>
  );
}

const MODIFIER_KEYS = ["Control", "Alt", "Shift", "Meta"];

/**
 * The shortcut a key press makes, e.g. `Ctrl+Shift+KeyK`; `null` while only
 * modifiers are down, or without any modifier (that would take over typing).
 */
function shortcut(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.includes(event.key)) return null;
  const modifiers = [
    event.ctrlKey && "Ctrl",
    event.altKey && "Alt",
    event.shiftKey && "Shift",
    event.metaKey && "Super",
  ].filter(Boolean);
  if (modifiers.length === 0) return null;
  return [...modifiers, event.code].join("+");
}

/** `Ctrl+Shift+KeyK` as people read it: `Ctrl + Shift + K`. */
export function spoken(hotkey: string): string {
  return hotkey
    .split("+")
    .map((key) => key.replace(/^(Key|Digit)/, ""))
    .join(" + ");
}
