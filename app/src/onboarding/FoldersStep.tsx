import { useState } from "react";
import { Button } from "../ui/Button";

/** A folder the Rust side offers to make searchable. */
export type FolderChoice = {
  path: string;
  kind: "personal" | "drive";
  picked: boolean;
};

type Props = {
  choices: FolderChoice[];
  onContinue: (paths: string[]) => void;
};

/** Onboarding step where the user picks which folders become searchable. */
export function FoldersStep({ choices, onContinue }: Props) {
  const [picked, setPicked] = useState(
    () => new Set(choices.filter((choice) => choice.picked).map((choice) => choice.path)),
  );
  const toggle = (path: string) =>
    setPicked((picked) => {
      const next = new Set(picked);
      if (!next.delete(path)) next.add(path);
      return next;
    });

  return (
    <form
     className="flex flex-col gap-5"
      onSubmit={(event) => {
        event.preventDefault();
        onContinue(choices.map((choice) => choice.path).filter((path) => picked.has(path)));
      }}
    >
      <div className="flex flex-col gap-1">
        <h1 className="text-xl font-semibold">Which files should be searchable?</h1>
        <p className="text-sm text-muted">
          Names are searchable right away; what is inside follows, newest files first.
        </p>
      </div>
      <div className="flex flex-col gap-2">
        {choices.map((choice) => (
          <label
            key={choice.path}
           className="flex cursor-pointer items-start gap-3 rounded-xl border border-line bg-panel p-4 transition-colors hover:bg-raised has-checked:border-accent has-checked:bg-accent-soft"
          >
            <input
              type="checkbox"
             className="mt-1 accent-accent"
              checked={picked.has(choice.path)}
              onChange={() => toggle(choice.path)}
            />
            <span className="flex flex-col gap-0.5">
              <span className="font-medium">{title(choice)}</span>
              <span className="text-sm text-muted">{choice.path}</span>
            </span>
          </label>
        ))}
      </div>
      <Button
        variant="primary"
        type="submit"
        disabled={picked.size === 0}
        className="self-end"
      >
        Continue
      </Button>
    </form>
  );
}

function title(choice: FolderChoice): string {
  if (choice.kind === "personal") return "Your personal folders";
  return `Drive ${choice.path.replace(/[\\/]+$/, "")}`;
}
