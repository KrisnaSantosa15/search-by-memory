import type { ResultRow } from "./Launcher";
import { Marked } from "./Marked";
import { Path } from "./Path";

const MATCH_REASONS = {
  name: "Its name matches",
  content: "What is inside matches",
  both: "Name and contents match",
};

/** The selected Result in full: its Content Match, where it lives, and why it matched. */
type Props = {
  result: ResultRow;
  query: string;
  /** The Copy being picked; `null` while none is being picked. */
  copy: number | null;
  onOpen: () => void;
  onReveal: () => void;
};

export function Preview({ result, query, copy, onOpen, onReveal }: Props) {
  const paths = [result.path, ...result.copies];
  const facts = [
    result.size !== null && size(result.size),
    result.modified !== null && `changed ${date(result.modified)}`,
  ].filter(Boolean);

  return (
    <section aria-label="Preview" className="flex min-w-0 flex-col gap-3">
      {result.snippet && (
        <blockquote className="rounded-lg border border-line bg-raised p-4 text-sm leading-relaxed">
          <Marked text={result.snippet} query={query} />
        </blockquote>
      )}
      <div className="flex flex-col gap-0.5">
        <h2 className="truncate text-base font-semibold">{result.name}</h2>
        <p className="truncate text-sm text-muted">{result.folder}</p>
        {facts.length > 0 && <p className="text-sm text-muted">{facts.join(", ")}</p>}
      </div>
      <p className="text-sm">
        {MATCH_REASONS[result.matchReason]}
        {result.location && `, ${result.location.toLowerCase()}`}
      </p>
      <div className="flex gap-2">
        <Action onClick={onOpen} primary>
          Open
        </Action>
        <Action onClick={onReveal}>Show in folder</Action>
      </div>
      {copy !== null ? (
        <div className="flex flex-col gap-1">
          <p className="text-sm font-medium">Which copy? Pick one with the arrow keys, then press Enter.</p>
          <ul role="listbox" aria-label="Copies" className="flex flex-col gap-0.5">
            {paths.map((path, index) => (
              <li
                key={path}
                role="option"
                aria-selected={index === copy}
                className="flex rounded-md px-2 py-1 text-sm aria-selected:bg-raised"
              >
                <Path path={path} />
              </li>
            ))}
          </ul>
        </div>
      ) : (
        result.copies.length > 0 && (
          <div className="flex flex-col gap-0.5 text-sm text-muted">
            <p>Also saved in</p>
            {result.copies.map((path) => (
              <p key={path} className="flex">
                <Path path={path} />
              </p>
            ))}
          </div>
        )
      )}
    </section>
  );
}

/** A button that leaves the keyboard in the Query box. */
function Action(props: { onClick: () => void; primary?: boolean; children: string }) {
  return (
    <button
      type="button"
      onMouseDown={(event) => event.preventDefault()}
      onClick={props.onClick}
      className={
        props.primary
          ? "rounded-md bg-accent px-3 py-1.5 text-sm font-medium text-canvas hover:bg-accent-hover"
          : "rounded-md border border-line px-3 py-1.5 text-sm hover:bg-raised"
      }
    >
      {props.children}
    </button>
  );
}

/** Bytes as people say them: `2.1 MB`. */
function size(bytes: number): string {
  const units = ["bytes", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1000 && unit < units.length - 1) {
    value /= 1000;
    unit += 1;
  }
  const digits = unit === 0 || value >= 10 ? 0 : 1;
  return `${value.toFixed(digits)} ${units[unit]}`;
}

/** A day in the user's own date format: `3 October 2026`. */
function date(millis: number): string {
  return new Date(millis).toLocaleDateString(undefined, {
    day: "numeric",
    month: "long",
    year: "numeric",
  });
}
