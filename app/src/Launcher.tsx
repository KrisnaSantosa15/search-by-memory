import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { Preview } from "./Preview";

/** One Result, as the Rust side sends it. */
export type ResultRow = {
  name: string;
  path: string;
  /** The folder holding `path`. */
  folder: string;
  /** Other paths with the same content. */
  copies: string[];
  /** False while the file's drive is a Disconnected Drive. */
  connected: boolean;
  /** Bytes; `null` when the file cannot be read now. */
  size: number | null;
  /** Last change, in milliseconds since 1970; `null` when the file cannot be read now. */
  modified: number | null;
  snippet: string | null;
  /** Where the snippet is, e.g. `Page 14`. */
  location: string | null;
  matchReason: "name" | "content" | "both";
};

type Props = {
  search: (query: string) => Promise<ResultRow[]>;
  open: (path: string) => void;
  reveal: (path: string) => void;
  copyPath: (path: string) => void;
  hide: () => void;
};

const MATCH_REASONS = { name: "name", content: "content", both: "name + content" };

/** The search window: a Query box over its Results. */
export function Launcher({ search, open, reveal, copyPath, hide }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ResultRow[]>([]);
  const [selected, setSelected] = useState(0);
  // The Copy picked in the open Detail Panel; `null` while it is closed.
  const [copy, setCopy] = useState<number | null>(null);
  const box = useRef<HTMLInputElement>(null);

  // The hotkey or the tray brought the window back: ready for a Query.
  useEffect(() => {
    const focus = () => box.current?.focus();
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  }, []);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      return;
    }
    // Only the newest Query's Results are shown, whichever answers last.
    let current = true;
    search(query).then((results) => {
      if (!current) return;
      setResults(results);
      setSelected(0);
      setCopy(null);
    });
    return () => {
      current = false;
    };
  }, [search, query]);

  const result = results[selected];
  const paths = result ? [result.path, ...result.copies] : [];
  // The file an action applies to; none while the user has yet to pick a Copy.
  const target = copy !== null ? paths[copy] : paths.length === 1 ? paths[0] : undefined;

  // A Disconnected Drive's file can be named but not opened or shown.
  const reachable = target !== undefined && (target !== result?.path || result.connected);

  /** Runs `action` on the target, or opens the Detail Panel to pick a Copy. */
  const act = (action: (path: string) => void, needsDrive = true) => {
    if (!target) {
      if (result) setCopy(0);
    } else if (reachable || !needsDrive) action(target);
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    const box = event.currentTarget;
    const textSelected = box.selectionStart !== box.selectionEnd;
    const caretAtEnd = !textSelected && box.selectionStart === box.value.length;
    const move = (step: number) => {
      if (copy !== null) setCopy(clamp(copy + step, paths.length));
      else setSelected((index) => clamp(index + step, results.length));
    };
    if (event.key === "Escape") hide();
    else if (event.key === "ArrowDown") move(1);
    else if (event.key === "ArrowUp") move(-1);
    else if (event.key === "ArrowRight" && caretAtEnd && result && copy === null) setCopy(0);
    else if (event.key === "ArrowLeft" && copy !== null) setCopy(null);
    else if (event.key === "Enter") act(event.ctrlKey || event.shiftKey ? reveal : open);
    else if (event.key === "c" && event.ctrlKey && !textSelected) act(copyPath, false);
    else return;
    event.preventDefault();
  };

  return (
    <section className="flex flex-col gap-3">
      <div className="flex gap-2">
        <input
          type="search"
          ref={box}
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Type what you remember about the file…"
          className="min-w-0 flex-1 rounded-xl border border-line bg-panel px-4 py-3 text-lg outline-none focus:border-accent"
        />
      </div>
      <div className="grid grid-cols-[minmax(0,5fr)_minmax(0,6fr)] gap-4">
        <ul role="listbox" aria-label="Results" className="flex flex-col gap-1">
          {results.map((result, index) => (
            <li
              key={`${result.path}#${index}`}
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
              <span className="flex gap-1 text-xs text-muted">
                <Path path={result.path} />
                {!result.connected && <span className="shrink-0">· drive not connected</span>}
              </span>
              {index === selected && copy !== null && (
                <ul role="listbox" aria-label="Copies" className="mt-2 flex flex-col gap-0.5">
                  {paths.map((path, index) => (
                    <li
                      key={path}
                      role="option"
                      aria-selected={index === copy}
                      className="flex rounded-lg px-3 py-1.5 text-sm aria-selected:bg-raised"
                    >
                      <Path path={path} />
                    </li>
                  ))}
                </ul>
              )}
            </li>
          ))}
        </ul>
        {result && <Preview result={result} />}
      </div>
    </section>
  );
}

/** A path that, when too long, loses its start: the folder and name stay readable. */
function Path({ path }: { path: string }) {
  return (
    <span className="min-w-0 truncate text-left [direction:rtl]">
      <bdi>{path}</bdi>
    </span>
  );
}

/** `index` kept within a list of `length` items. */
function clamp(index: number, length: number): number {
  return Math.max(0, Math.min(index, length - 1));
}
