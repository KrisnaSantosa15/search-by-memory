import { CircleCheck, Keyboard, TriangleAlert } from "lucide-react";
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
        <h1 className="text-xl font-semibold tracking-[-0.02em]">Search from anywhere</h1>
        <p className="text-sm text-muted">
          Press these keys in any app to bring up search. To use other keys, click below and
          press them.
        </p>
      </div>
      <button
        type="button"
        aria-label={`Keys for search: ${spoken(hotkey)}`}
        onKeyDown={record}
        className="group flex flex-col items-center gap-4 rounded-xl border border-line bg-panel/40 px-4 py-7 transition-colors hover:bg-hover focus:border-accent focus:bg-accent-soft focus:outline-none"
      >
        <span className="flex items-center gap-2">
          {keys(hotkey).map((key, index) => (
            <span key={key} className="flex items-center gap-2">
              {index > 0 && <span className="text-muted">+</span>}
              <kbd className="min-w-11 rounded-lg border border-line bg-raised px-3 py-1.5 text-center font-sans text-base font-medium text-ink shadow-[inset_0_-2px_0_rgb(0_0_0/0.25)]">
                {key}
              </kbd>
            </span>
          ))}
        </span>
        <span className="flex items-center gap-1.5 text-xs text-muted">
          <Keyboard aria-hidden className="size-3.5" strokeWidth={1.75} />
          <span className="group-focus:hidden">Click to change</span>
          <span className="hidden group-focus:inline">Press the keys you want</span>
        </span>
      </button>
      {free && (
        <p className={`flex items-center gap-2 text-sm ${works ? "text-ink" : "text-muted"}`}>
          {works && <CircleCheck aria-hidden className="size-4 text-accent" strokeWidth={1.75} />}
          {works ? "They work." : "Try them now: press the keys."}
        </p>
      )}
      {free === false && (
        <p className="flex items-center gap-3 rounded-xl border border-accent/40 bg-accent-soft p-4 text-sm">
          <TriangleAlert aria-hidden className="size-4.5 shrink-0 text-accent" strokeWidth={1.75} />
          Another app already uses these keys. Press other keys you would like to use.
        </p>
      )}
      <Button variant="primary" type="submit" disabled={!free} className="self-end">
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
  return keys(hotkey).join(" + ");
}

/** Each key of `Ctrl+Shift+KeyK` as written on the keyboard: `Ctrl`, `Shift`, `K`. */
function keys(hotkey: string): string[] {
  return hotkey.split("+").map((key) => key.replace(/^(Key|Digit)/, ""));
}
