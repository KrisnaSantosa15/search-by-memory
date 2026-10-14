import type { ResultRow } from "./Launcher";

const MATCH_REASONS = {
  name: "Its name matches",
  content: "What is inside matches",
  both: "Name and contents match",
};

/** The selected Result in full: its Content Match, where it lives, and why it matched. */
export function Preview({ result }: { result: ResultRow }) {
  const facts = [
    result.size !== null && size(result.size),
    result.modified !== null && `changed ${date(result.modified)}`,
  ].filter(Boolean);

  return (
    <section aria-label="Preview" className="flex min-w-0 flex-col gap-3">
      {result.snippet && (
        <blockquote className="rounded-lg border border-line bg-raised p-4 text-sm leading-relaxed">
          {result.snippet}
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
    </section>
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
