import { CircleCheck, TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "../ui/Button";
import { KeysPicker } from "../ui/KeysPicker";

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
      <KeysPicker hotkey={hotkey} onChange={setHotkey} />
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
