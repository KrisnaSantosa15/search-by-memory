import { FolderOpen } from "lucide-react";
import { useState } from "react";
import { Button } from "../ui/Button";

type Props = {
  /** The personal folder, searched unless the user picks another. */
  defaultLocation: string;
  /** Asks the user for a folder; `null` when they picked none. */
  pickFolder: () => Promise<string | null>;
  onContinue: (location: string) => void;
};

/** Onboarding step that asks where to search, the way an installer asks where to install. */
export function LocationStep({ defaultLocation, pickFolder, onContinue }: Props) {
  const [location, setLocation] = useState(defaultLocation);

  const change = async () => {
    const picked = await pickFolder();
    if (picked) setLocation(picked);
  };

  return (
    <form
      className="flex flex-col gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        onContinue(location);
      }}
    >
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold tracking-[-0.02em]">Where are the files you want to find?</h1>
        <p className="text-sm text-muted">
          Everything in this folder becomes searchable. You can add more folders later in Settings.
        </p>
      </div>
      <div className="flex items-center gap-3 rounded-xl border border-line bg-panel/40 p-3">
        <span className="grid size-9 shrink-0 place-items-center rounded-lg border border-line bg-raised text-accent">
          <FolderOpen aria-hidden className="size-4.5" strokeWidth={1.75} />
        </span>
        <span className="min-w-0 flex-1 truncate font-mono text-xs" title={location}>
          {location}
        </span>
        {location !== defaultLocation && (
          <Button variant="ghost" size="sm" onClick={() => setLocation(defaultLocation)}>
            Reset to default
          </Button>
        )}
        <Button size="sm" onClick={change}>
          Change…
        </Button>
      </div>
      <Button variant="primary" type="submit" className="self-end">
        Continue
      </Button>
    </form>
  );
}
