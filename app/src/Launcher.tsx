import { type KeyboardEvent, useEffect, useRef, useState } from "react";
import { Marked } from "./Marked";
import { Path } from "./Path";
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
  kind: Kind;
  /** A Weaker Match, folded away below the others. */
  weaker: boolean;
};

type Kind = "document" | "code" | "image" | "audio" | "video" | "other";

const GROUP_NAMES: Record<Kind, string> = {
  document: "Documents",
  code: "Code",
  image: "Images",
  audio: "Audio",
  video: "Video",
  other: "Other files",
};

type Props = {
  search: (query: string) => Promise<ResultRow[]>;
  open: (path: string) => void;
  reveal: (path: string) => void;
  copyFile: (path: string) => void;
  hide: () => void;
};

/** The search window: a Query box over its Results. */
export function Launcher({ search, open, reveal, copyFile, hide }: Props) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ResultRow[]>([]);
  const [selected, setSelected] = useState(0);
  const [searching, setSearching] = useState(false);
  // The Copy being picked in the Preview Pane; `null` while none is being picked.
  const [copy, setCopy] = useState<number | null>(null);
  const [unfolded, setUnfolded] = useState(false);
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
      setSearching(false);
      return;
    }
    // Only the newest Query's Results are shown, whichever answers last.
    let current = true;
    setSearching(true);
    search(query).then((results) => {
      if (!current) return;
      setSearching(false);
      setResults(inGroupOrder(results));
      setSelected(0);
      setCopy(null);
      setUnfolded(false);
    });
    return () => {
      current = false;
    };
  }, [search, query]);

  const strong = results.filter((result) => !result.weaker);
  const faint = results.filter((result) => result.weaker);
  // What ↑/↓ move through, in the order on screen; FOLD is the "weaker matches" row.
  const entries: (ResultRow | typeof FOLD)[] = [
    ...strong,
    ...(faint.length > 0 ? [FOLD] : []),
    ...(unfolded ? faint : []),
  ];
  const entry = entries[selected];
  const result = entry === FOLD ? undefined : entry;
  const paths = result ? [result.path, ...result.copies] : [];
  // The file an action applies to; none while the user has yet to pick a Copy.
  const target = copy !== null ? paths[copy] : paths.length === 1 ? paths[0] : undefined;

  // A Disconnected Drive's file can be named but not opened or shown.
  const reachable = target !== undefined && (target !== result?.path || result.connected);

  /** Runs `action` on the target, or asks which Copy to use. */
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
      else setSelected((index) => clamp(index + step, entries.length));
    };
    if (event.key === "Escape") hide();
    else if (event.key === "ArrowDown") move(1);
    else if (event.key === "ArrowUp") move(-1);
    else if (entry === FOLD && (event.key === "Enter" || (event.key === "ArrowRight" && caretAtEnd))) {
      setUnfolded(true);
    } else if (event.key === "ArrowRight" && caretAtEnd && result && copy === null) setCopy(0);
    else if (event.key === "ArrowLeft" && copy !== null) setCopy(null);
    else if (event.key === "Enter") act(event.ctrlKey || event.shiftKey ? reveal : open);
    else if (event.key === "c" && event.ctrlKey && !textSelected) act(copyFile, false);
    else return;
    event.preventDefault();
  };

  return (
    <section className="flex h-full flex-col">
      <label data-tauri-drag-region className="flex shrink-0 items-center gap-3 border-b border-line px-5 py-4">
        <SearchIcon />
        <input
          type="search"
          ref={box}
          autoFocus
          value={query}
          onChange={(event) => setQuery(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Describe the file you remember"
          className="min-w-0 flex-1 bg-transparent font-display text-xl font-light outline-none placeholder:text-muted/70 [&::-webkit-search-cancel-button]:hidden"
        />
      </label>
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,5fr)_minmax(0,6fr)]">
        <div
          role="listbox"
          aria-label="Results"
          className="flex flex-col gap-3 overflow-y-auto border-r border-line p-2"
        >
          {groups(strong).map(({ kind, first, members }) => (
            <div key={kind} role="group" aria-label={GROUP_NAMES[kind]} className="flex flex-col gap-0.5">
              <p aria-hidden className="px-3 pt-1 text-xs font-medium text-muted">
                {GROUP_NAMES[kind]}
              </p>
              {members.map((result, offset) => (
                <Row
                  key={`${result.path}#${first + offset}`}
                  result={result}
                  query={query}
                  selected={first + offset === selected}
                  onSelect={() => setSelected(first + offset)}
                />
              ))}
            </div>
          ))}
          {faint.length > 0 && (
            <div className="flex flex-col gap-0.5">
              <div
                role="option"
                aria-selected={entry === FOLD}
                aria-expanded={unfolded}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  setSelected(strong.length);
                  setUnfolded((unfolded) => !unfolded);
                }}
                className="flex cursor-default items-center gap-2 rounded-md border-l-2 border-transparent px-3 py-2 text-sm text-muted aria-selected:border-accent aria-selected:bg-accent-soft"
              >
                <span aria-hidden className={unfolded ? "rotate-90" : ""}>
                  ›
                </span>
                {faint.length === 1 ? "1 weaker match" : `${faint.length} weaker matches`}
              </div>
              {unfolded &&
                faint.map((result, offset) => (
                  <Row
                    key={`${result.path}#weaker`}
                    result={result}
                    query={query}
                    selected={strong.length + 1 + offset === selected}
                    onSelect={() => setSelected(strong.length + 1 + offset)}
                  />
                ))}
            </div>
          )}
        </div>
        <div className="min-h-0 overflow-y-auto p-5">
          {result ? (
            <Preview
              result={result}
              query={query}
              copy={copy}
              onOpen={() => act(open)}
              onReveal={() => act(reveal)}
            />
          ) : (
            !query.trim() && (
              <p className="pt-16 text-center text-sm leading-relaxed text-muted">
                Describe it the way you remember it,
                <br />
                like “the dentist invoice from March” or “slow-cooked beef recipe”.
              </p>
            )
          )}
        </div>
      </div>
      <footer className="flex shrink-0 items-center justify-between gap-4 border-t border-line px-5 py-2.5 text-xs text-muted">
        <p role="status">{status(query, searching, strong.length, faint.length)}</p>
        <ul aria-label="Keys" className="flex gap-4">
          {KEYS.map(([keys, action]) => (
            <li key={action}>
              <kbd className="mr-1 font-sans text-ink">{keys}</kbd>
              {action}
            </li>
          ))}
        </ul>
      </footer>
    </section>
  );
}

/** The magnifier in front of the Query box. */
function SearchIcon() {
  return (
    <svg aria-hidden viewBox="0 0 24 24" className="size-6 shrink-0 text-accent" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <circle cx="10.5" cy="10.5" r="6.5" />
      <path d="m15.5 15.5 5 5" />
    </svg>
  );
}

/** One Result in the list: its name, Location, and snippet or folder. */
function Row(props: { result: ResultRow; query: string; selected: boolean; onSelect: () => void }) {
  const { result, query } = props;
  return (
    <div
      role="option"
      aria-selected={props.selected}
      onMouseDown={(event) => event.preventDefault()}
      onClick={props.onSelect}
      className="flex cursor-default flex-col gap-0.5 rounded-md border-l-2 border-transparent px-3 py-2 aria-selected:border-accent aria-selected:bg-accent-soft"
    >
      <span className="flex min-w-0 items-baseline gap-2">
        <span className="truncate font-medium">{result.name}</span>
        {result.location && (
          <span className="shrink-0 rounded bg-raised px-1.5 text-xs text-muted">
            {result.location}
          </span>
        )}
      </span>
      {result.snippet ? (
        <span className="truncate text-sm text-muted">
          <Marked text={result.snippet} query={query} />
        </span>
      ) : (
        <span className="flex text-sm text-muted">
          <Path path={result.folder} />
        </span>
      )}
      {!result.connected && <span className="text-xs text-accent">Drive not connected</span>}
    </div>
  );
}

/** The "weaker matches" row, among the Results that ↑/↓ move through. */
const FOLD = "fold" as const;

/**
 * `results` with each kind kept together, kinds in the order of their best
 * Result; Weaker Matches last, as they came.
 */
function inGroupOrder(results: ResultRow[]): ResultRow[] {
  const strong = results.filter((result) => !result.weaker);
  return [
    ...groups(strong).flatMap((group) => group.members),
    ...results.filter((result) => result.weaker),
  ];
}

/** Runs of Results of one kind; `first` is the index of a group's first Result. */
function groups(results: ResultRow[]) {
  const byKind = new Map<Kind, ResultRow[]>();
  for (const result of results) byKind.set(result.kind, [...(byKind.get(result.kind) ?? []), result]);
  let first = 0;
  return [...byKind].map(([kind, members]) => {
    const group = { kind, first, members };
    first += members.length;
    return group;
  });
}

const KEYS = [
  ["Enter", "Open"],
  ["Ctrl+Enter", "Show in folder"],
  ["Ctrl+C", "Copy"],
];

/** What the search is doing, in a few words. */
function status(query: string, searching: boolean, found: number, weaker: number): string {
  if (!query.trim()) return "Type what you remember about a file";
  if (searching) return "Searching…";
  if (found + weaker === 0) return "Nothing matches yet";
  const results = found === 1 ? "1 result" : `${found} results`;
  return weaker > 0 ? `${results}, ${weaker} weaker` : results;
}

/** `index` kept within a list of `length` items. */
function clamp(index: number, length: number): number {
  return Math.max(0, Math.min(index, length - 1));
}
