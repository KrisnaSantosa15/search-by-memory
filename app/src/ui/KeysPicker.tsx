import { Keyboard } from "lucide-react";
import type { KeyboardEvent } from "react";

/** The keys that bring up search, shown as key caps; click it and press others to change them. */
export function KeysPicker({
  hotkey,
  onChange,
}: {
  hotkey: string;
  onChange: (hotkey: string) => void;
}) {
  const record = (event: KeyboardEvent) => {
    const pressed = shortcut(event);
    if (!pressed) return;
    event.preventDefault();
    onChange(pressed);
  };

  return (
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
export function keys(hotkey: string): string[] {
  return hotkey.split("+").map((key) => key.replace(/^(Key|Digit)/, ""));
}
