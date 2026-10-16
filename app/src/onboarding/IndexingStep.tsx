import { useEffect, useState } from "react";
import type { Dashboard } from "../Settings";
import { Button } from "../ui/Button";

type Props = {
  progress: () => Promise<Dashboard>;
  /** Milliseconds between looks at the progress. */
  every?: number;
  onContinue: () => void;
};

/** Onboarding step that shows indexing as it happens. */
export function IndexingStep({ progress, every = 1000, onContinue }: Props) {
  const [status, setStatus] = useState<Dashboard | null>(null);

  useEffect(() => {
    let current = true;
    const look = () => progress().then((status) => current && setStatus(status));
    look();
    const timer = setInterval(look, every);
    return () => {
      current = false;
      clearInterval(timer);
    };
  }, [progress, every]);

  // Names are searchable once the first pass over the location has ended.
  const namesFound = status !== null && !status.scanning;

  return (
    <section className="flex flex-col gap-5">
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold tracking-[-0.02em]">Getting your files ready</h1>
        <p className="text-sm text-muted">
          File names come first, then what is inside them, newest files first.
        </p>
      </div>
      <p className="text-sm font-medium">
        {namesFound ? `${status.files.toLocaleString()} files found` : "Finding files…"}
      </p>
      <Button variant="primary" disabled={!namesFound} onClick={onContinue} className="self-end">
        Continue
      </Button>
    </section>
  );
}
