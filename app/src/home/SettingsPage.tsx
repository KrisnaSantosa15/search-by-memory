import { CircleCheck, Cpu, Info, Gpu, TriangleAlert } from "lucide-react";
import { useEffect, useState } from "react";
import { type AppSettings, DeleteIndex, PANEL, Section } from "../Settings";
import { BrandMark } from "../ui/BrandMark";
import { KeysPicker, spoken } from "../ui/KeysPicker";

export type SettingsPageProps = {
  settings: () => Promise<AppSettings | null>;
  /** Makes `hotkey` bring up search; `false` when another app already uses it. */
  changeHotkey: (hotkey: string) => Promise<boolean>;
  /** Erases the index from disk and starts setup again. */
  purge: () => Promise<void>;
  /** This app's version, e.g. `0.1.0`. */
  version: () => Promise<string>;
};

/** The Settings page: the search keys, and what search runs on. */
export function SettingsPage({ settings, changeHotkey, purge, version }: SettingsPageProps) {
  const [saved, setSaved] = useState<AppSettings | null>(null);
  const [release, setRelease] = useState<string | null>(null);

  useEffect(() => {
    settings().then(setSaved);
  }, [settings]);

  useEffect(() => {
    version().then(setRelease);
  }, [version]);

  if (!saved) return null;
  return (
    <>
      <SearchKeys
        hotkey={saved.hotkey}
        change={changeHotkey}
        onChanged={(hotkey) => setSaved({ ...saved, hotkey })}
      />
      <Section
        title="Reading speed"
        description="Picked at setup. Content is read and searched where it runs fastest on this computer."
      >
        <div className={`${PANEL} px-4 py-3`}>
          <span className="flex items-center gap-3 text-sm">
            {saved.accelerator === "cpu" ? (
              <Cpu aria-hidden className="size-4 text-muted" strokeWidth={1.75} />
            ) : (
              <Gpu aria-hidden className="size-4 text-muted" strokeWidth={1.75} />
            )}
            <span className="font-medium">
              {saved.accelerator === "cpu" ? "Processor" : "Graphics card"}
            </span>
          </span>
        </div>
      </Section>
      <Section
        title="Start over"
        description="Forget every file and set search up again. Your files themselves stay as they are."
      >
        <DeleteIndex purge={purge} />
      </Section>
      <Section title="About">
        <div className="flex items-center gap-3 rounded-xl border border-line bg-panel/40 px-4 py-3">
          <BrandMark className="size-9 shrink-0" />
          <span className="flex flex-col">
            <span className="text-sm font-semibold">Search by Memory</span>
            {release && <span className="text-xs text-muted">{`Version ${release}`}</span>}
          </span>
          <span className="ml-auto flex items-center gap-1.5 text-xs text-muted">
            <Info aria-hidden className="size-3.5" strokeWidth={1.75} />
            Your files and searches never leave this computer.
          </span>
        </div>
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
