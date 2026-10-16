import { CircleCheck, TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { type AppSettings, DeleteIndex, Section } from "../Settings";
import { KeysPicker, spoken } from "../ui/KeysPicker";

export type SettingsPageProps = {
  settings: () => Promise<AppSettings | null>;
  /** Makes `hotkey` bring up search; `false` when another app already uses it. */
  changeHotkey: (hotkey: string) => Promise<boolean>;
  /** Erases the index from disk and starts setup again. */
  purge: () => Promise<void>;
};

/** The Settings page: the search keys, and what search runs on. */
export function SettingsPage({ settings, changeHotkey, purge }: SettingsPageProps) {
  const [saved, setSaved] = useState<AppSettings | null>(null);

  useEffect(() => {
    settings().then(setSaved);
  }, [settings]);

  if (!saved) return null;
  return (
    <>
      <SearchKeys
        hotkey={saved.hotkey}
        change={changeHotkey}
        onChanged={(hotkey) => setSaved({ ...saved, hotkey })}
      />
      <Section
        title="Start over"
        description="Forget every file and set search up again. Your files themselves stay as they are."
      >
        <DeleteIndex purge={purge} />
      </Section>
    </>
  );
}

/** What the last change of keys came to. */
type Outcome = { hotkey: string; free: boolean };

function SearchKeys(props: {
  hotkey: string;
  change: (hotkey: string) => Promise<boolean>;
  onChanged: (hotkey: string) => void;
}) {
  const [outcome, setOutcome] = useState<Outcome | null>(null);

  const change = async (hotkey: string) => {
    const free = await props.change(hotkey);
    setOutcome({ hotkey, free });
    if (free) props.onChanged(hotkey);
  };

  return (
    <Section
      title="Search keys"
      description="Press these keys in any app to bring up search. Click them and press others to change them."
    >
      <KeysPicker hotkey={props.hotkey} onChange={change} />
      {outcome?.free && (
        <p className="flex items-center gap-2 text-sm">
          <CircleCheck aria-hidden className="size-4 text-accent" strokeWidth={1.75} />
          {`Saved. Press ${spoken(outcome.hotkey)} in any app to search.`}
        </p>
      )}
      {outcome && !outcome.free && (
        <p className="flex items-center gap-3 rounded-xl border border-accent/40 bg-accent-soft p-4 text-sm">
          <TriangleAlert aria-hidden className="size-4.5 shrink-0 text-accent" strokeWidth={1.75} />
          {`Another app already uses ${spoken(outcome.hotkey)}, so search keeps ${spoken(props.hotkey)}.`}
        </p>
      )}
    </Section>
  );
}
