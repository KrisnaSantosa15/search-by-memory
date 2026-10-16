import { FolderOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { Path } from "../Path";
import { size } from "../size";
import { Button } from "../ui/Button";

/** What a location would add to the index. */
export type Estimate = { files: number; bytes: number };

type Props = {
  /** The personal folder, searched unless the user picks another. */
  defaultLocation: string;
  /** Counts what `location` would add to the index. */
  estimate: (location: string) => Promise<Estimate>;
  /** Asks the user for a folder; `null` when they picked none. */
  pickFolder: () => Promise<string | null>;
  onContinue: (location: string) => void;
};

/** Onboarding step that asks where to search, the way an installer asks where to install. */
export function LocationStep({ defaultLocation, estimate, pickFolder, onContinue }: Props) {
  const [location, setLocation] = useState(defaultLocation);
  const [found, setFound] = useState<Estimate | null>(null);

  useEffect(() => {
    // Only the newest location's count is shown, whichever answers last.
    let current = true;
    setFound(null);
    estimate(location).then((found) => current && setFound(found));
    return () => {
      current = false;
    };
  }, [estimate, location]);

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
        <span className="flex min-w-0 flex-1 flex-col gap-0.5">
          <span className="flex font-mono text-xs" title={location}>
            <Path path={location} />
          </span>
          <span className="text-xs text-muted">
            {found
              ? `About ${found.files.toLocaleString()} files, ${size(found.bytes)}`
              : "Counting files…"}
          </span>
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
