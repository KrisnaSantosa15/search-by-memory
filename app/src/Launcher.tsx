import { type KeyboardEvent, useEffect, useState } from "react";

/** One Result, as the Rust side sends it. */
export type ResultRow = {
  name: string;
  path: string;
  /** Other paths with the same content. */
  copies: string[];
  /** False while the file's drive is a Disconnected Drive. */
  connected: boolean;
  snippet: string | null;
  /** Where the snippet is, e.g. `Page 14`. */
  location: string | null;
  matchReason: "name" | "content" | "both";
};

export type View = "file" | "passage";

type Props = {
  search: (query: string, view: View) => Promise<ResultRow[]>;
  open: (path: string) => void;
  reveal: (path: string) => void;
  copyPath: (path: string) => void;
  hide: () => void;
};

const MATCH_REASONS = { name: "name", content: "content", both: "name + content" };

/** The search window: a Query box over its Results. */
export function Launcher({ search, open, reveal, copyPath }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ResultRow[]>([]);
  const [selected, setSelected] = useState(0);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      return;
    }
    // Only the newest Query's Results are shown, whichever answers last.
    let current = true;
    search(query, "file").then((results) => {
      if (!current) return;
      setResults(results);
      setSelected(0);
    });
    return () => {
      current = false;
    };
  }, [search, query]);

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const box = event.currentTarget;
    const textSelected = box.selectionStart !== box.selectionEnd;
    const last = results.length - 1;
    const result = results[selected];
    if (event.key === "ArrowDown") setSelected((index) => Math.min(index + 1, last));
    else if (event.key === "ArrowUp") setSelected((index) => Math.max(index - 1, 0));
    else if (event.key === "Enter" && result) {
      if (event.ctrlKey || event.shiftKey) reveal(result.path);
      else open(result.path);
    } else if (event.key === "c" && event.ctrlKey && !textSelected && result) {
      copyPath(result.path);
    }
    else return;
    event.preventDefault();
  };

  return (
    <section className="flex flex-col gap-3">
      <input
        type="search"
        autoFocus
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        onKeyDown={onKeyDown}
        placeholder="Type what you remember about the file…"
        className="rounded-xl border border-line bg-panel px-4 py-3 text-lg outline-none focus:border-accent"
      />
      <ul role="listbox" aria-label="Results" className="flex flex-col gap-1">
        {results.map((result, index) => (
          <li
            key={result.path}
            role="option"
            aria-selected={index === selected}
            className="flex flex-col gap-0.5 rounded-xl border border-transparent p-3 aria-selected:border-accent aria-selected:bg-accent-soft"
          >
            <span className="flex items-baseline gap-2">
              <span className="font-medium">{result.name}</span>
              {result.location && <span className="text-sm text-muted">{result.location}</span>}
              <span className="ml-auto text-xs text-muted">
                {MATCH_REASONS[result.matchReason]}
              </span>
            </span>
            {result.snippet && <span className="line-clamp-2 text-sm">{result.snippet}</span>}
            <span className="truncate text-xs text-muted">{result.path}</span>
          </li>
        ))}
      </ul>
    </section>
  );
}
