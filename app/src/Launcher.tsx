import { CodeXml, History, Search } from "lucide-react";
import { type KeyboardEvent, useEffect, useMemo, useRef, useState } from "react";
import type { LoadLines } from "./Lines";
import { Marked } from "./Marked";
import { Path } from "./Path";
import type { LoadPdfPage } from "./PdfPage";
import { Preview } from "./Preview";
import { cachedThumbnails, type LoadThumbnail, Thumbnail } from "./Thumbnail";
import type { Unreadable } from "./unreadable";

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
  /** The matched page, 1-based, when the Location is a page. */
  page: number | null;
  /** The matched lines, 1-based and inclusive, when the Location is lines. */
  lineStart: number | null;
  lineEnd: number | null;
  matchReason: "name" | "content" | "both";
  kind: Kind;
  /** A Weaker Match, folded away below the others. */
  weaker: boolean;
  /** Why its content is not searched; `null` when it is. */
  notRead: NotRead | null;
};

/** Why a Result's content is not searched. */
export type NotRead =
  | { kind: "onlineOnly" | "format" | "pending" }
  | { kind: "failed"; reason: Unreadable };

/** A filter prefix found in what was typed, e.g. `ext:pdf`, in plain words. */
export type FilterChip = {
  kind: "code" | "name" | "ext" | "in" | "date";
  label: string;
};

/** What a search found, with the Query it searched (filter prefixes lifted out). */
export type Searched = {
  query: string;
  results: ResultRow[];
  chips: FilterChip[];
};

export type Kind = "document" | "code" | "image" | "audio" | "video" | "other";

const GROUP_NAMES: Record<Kind, string> = {
  document: "Documents",
  code: "Code",
  image: "Images",
  audio: "Audio",
  video: "Video",
  other: "Other files",
};

type Props = {
  /** Results for what was typed, within the filter prefixes in it. */
  search: (typed: string) => Promise<Searched>;
  open: (path: string) => void;
  reveal: (path: string) => void;
  copyFile: (path: string) => void;
  previewLines: LoadLines;
  thumbnail: LoadThumbnail;
  pdfPage: LoadPdfPage;
  hide: () => void;
  /** Where the Copies picked before are kept, such as `localStorage`. */
  memory?: Memory;
  /** Calls back with what another window, such as Home, asks to search for; returns how to stop. */
  listenSearchFor?: (searchFor: (typed: string) => void) => () => void;
};

/** The part of the Web Storage API the launcher remembers things in. */
export type Memory = Pick<Storage, "getItem" | "setItem">;

/** The search window: a Query box over its Results. */
export function Launcher(props: Props) {
  const { search, open, reveal, copyFile, hide, previewLines } = props;
  const thumbnails = useMemo(() => cachedThumbnails(props.thumbnail), [props.thumbnail]);
  const [typed, setTyped] = useState("");
  // The Query the shown Results were found for, and the filters they kept to.
  const [query, setQuery] = useState("");
  const [chips, setChips] = useState<FilterChip[]>([]);
  const [results, setResults] = useState<ResultRow[]>([]);
  const [selected, setSelected] = useState(0);
  const [searching, setSearching] = useState(false);
  // The Copy being picked in the Preview Pane; `null` while none is being picked.
  const [copy, setCopy] = useState<number | null>(null);
  const [unfolded, setUnfolded] = useState(false);
  const box = useRef<HTMLInputElement>(null);

  const { listenSearchFor } = props;
  useEffect(() => {
    if (!listenSearchFor) return;
    return listenSearchFor((typed) => {
      setTyped(typed);
      box.current?.focus();
    });
  }, [listenSearchFor]);

  // The hotkey or the tray brought the window back: ready for a Query.
  useEffect(() => {
    const focus = () => box.current?.focus();
    window.addEventListener("focus", focus);
    return () => window.removeEventListener("focus", focus);
  }, []);

  useEffect(() => {
    if (!typed.trim()) {
      setResults([]);
      setChips([]);
      setSearching(false);
      return;
    }
    // Only the newest Query's Results are shown, whichever answers last.
    let current = true;
    setSearching(true);
    search(typed).then(({ query, results, chips }) => {
      if (!current) return;
      setSearching(false);
      setQuery(query);
      setChips(chips);
      setResults(inGroupOrder(results));
      setSelected(0);
      setCopy(null);
      setUnfolded(false);
    });
    return () => {
      current = false;
    };
  }, [search, typed]);

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
  const foldRow = useSelectedInView(entry === FOLD);
  const paths = result ? [result.path, ...result.copies] : [];
  // The file an action applies to; none while the user has yet to pick a Copy.
  const target = copy !== null ? paths[copy] : paths.length === 1 ? paths[0] : undefined;

  // A Disconnected Drive's file can be named but not opened or shown.
  const reachable = target !== undefined && (target !== result?.path || result.connected);

  /** Starts asking which Copy, on the one picked most recently. */
  const pickCopy = () => setCopy(lastPicked(paths, props.memory));

  /** Runs `action` on the target, or asks which Copy to use. */
  const act = (action: (path: string) => void, needsDrive = true) => {
    if (!target) {
      if (result) pickCopy();
    } else if (reachable || !needsDrive) {
      if (copy !== null) rememberPicked(target, props.memory);
      rememberSearch(typed, props.memory);
      action(target);
    }
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
    else if (entry === FOLD && event.key === "Enter") setUnfolded((unfolded) => !unfolded);
    else if (entry === FOLD && event.key === "ArrowRight" && caretAtEnd) setUnfolded(true);
    else if (event.key === "ArrowRight" && caretAtEnd && result && copy === null) pickCopy();
    else if (event.key === "ArrowLeft" && copy !== null) setCopy(null);
    else if (event.key === "Enter") act(event.ctrlKey || event.shiftKey ? reveal : open);
    else if (event.key === "c" && event.ctrlKey && !textSelected) act(copyFile, false);
    else return;
    event.preventDefault();
  };

  return (
    <section className="flex h-full flex-col">
      <label data-tauri-drag-region className="flex h-14 shrink-0 items-center gap-3 border-b border-line px-5">
        <SearchIcon code={chips.some((chip) => chip.kind === "code")} />
        <input
          type="search"
          ref={box}
          autoFocus
          spellCheck={false}
          value={typed}
          onChange={(event) => setTyped(event.target.value)}
          onKeyDown={onKeyDown}
          placeholder="Describe the file you remember"
          className="min-w-0 flex-1 bg-transparent text-lg tracking-[-0.01em] outline-none placeholder:text-muted/70 [&::-webkit-search-cancel-button]:hidden"
        />
        {chips.length > 0 && (
          <ul aria-label="Filters" className="flex shrink-0 gap-1.5">
            {chips.map((chip, index) => (
              <li
                key={`${chip.kind}#${index}`}
                className="rounded-full border border-accent/30 bg-accent-soft px-2.5 py-0.5 text-xs font-medium text-accent"
              >
                {chip.label}
              </li>
            ))}
          </ul>
        )}
      </label>
      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,5fr)_minmax(0,6fr)]">
        {!typed.trim() ? (
          <div className="overflow-y-auto border-r border-line bg-chrome/40 p-2">
            <Suggestions
              recent={recentSearches(props.memory)}
              onPick={(picked) => {
                setTyped(picked);
                box.current?.focus();
              }}
            />
          </div>
        ) : (
        <div
          role="listbox"
          aria-label="Results"
          className="flex flex-col gap-3 overflow-y-auto border-r border-line bg-chrome/40 p-2"
        >
          {groups(strong).map(({ kind, first, members }) => (
            <div key={kind} role="group" aria-label={GROUP_NAMES[kind]} className="flex flex-col gap-0.5">
              <p aria-hidden className="px-3 pt-1.5 pb-0.5 text-xs font-semibold text-muted">
                {GROUP_NAMES[kind]}
              </p>
              {members.map((result, offset) => (
                <Row
                  key={`${result.path}#${first + offset}`}
                  result={result}
                  query={query}
                  selected={first + offset === selected}
                  onSelect={() => setSelected(first + offset)}
                  thumbnails={thumbnails}
                />
              ))}
            </div>
          ))}
          {faint.length > 0 && (
            <div className="flex flex-col gap-0.5">
              <div
                ref={foldRow}
                role="option"
                aria-selected={entry === FOLD}
                aria-expanded={unfolded}
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => {
                  setSelected(strong.length);
                  setUnfolded((unfolded) => !unfolded);
                }}
                className="flex cursor-default items-center gap-2 rounded-[10px] border-l-2 border-transparent px-3 py-2 text-sm text-muted hover:bg-hover/60 aria-selected:border-accent aria-selected:bg-accent-soft"
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
                    thumbnails={thumbnails}
                  />
                ))}
            </div>
          )}
        </div>
        )}
        <div className="min-h-0 overflow-y-auto p-5">
          {result ? (
            <Preview
              result={result}
              query={query}
              loadLines={previewLines}
              loadPdfPage={props.pdfPage}
              loadPicture={thumbnails}
              copy={copy}
              onOpen={() => act(open)}
              onReveal={() => act(reveal)}
            />
          ) : entry === FOLD ? (
            <p className="pt-16 text-center text-sm leading-relaxed text-muted">
              These files match less closely.
              <br />
              Press Enter to {unfolded ? "hide" : "show"} them.
            </p>
          ) : (
            !typed.trim() && <FilterTips />
          )}
        </div>
      </div>
      <footer className="flex h-10 shrink-0 items-center justify-between gap-4 border-t border-line px-5 text-xs text-muted">
        <p role="status">{status(typed, searching, strong.length, faint.length)}</p>
        <ul aria-label="Keys" className="flex gap-4">
          {KEYS.map(([keys, action]) => (
            <li key={action}>
              <kbd className="mr-1.5 rounded-md border border-line bg-raised px-1.5 py-0.5 font-sans text-[11px] text-ink">
                {keys}
              </kbd>
              {action}
            </li>
          ))}
        </ul>
      </footer>
    </section>
  );
}

/** The magnifier in front of the Query box; brackets while searching code. */
function SearchIcon({ code }: { code: boolean }) {
  const Icon = code ? CodeXml : Search;
  return (
    <Icon
      role="img"
      aria-label={code ? "Searching code" : "Search"}
      className="size-5 shrink-0 text-accent"
      strokeWidth={1.75}
    />
  );
}

/** One Result in the list: its name, Location, and snippet or folder. */
function Row(props: {
  result: ResultRow;
  query: string;
  selected: boolean;
  onSelect: () => void;
  thumbnails: LoadThumbnail;
}) {
  const { result, query } = props;
  const row = useSelectedInView(props.selected);
  return (
    <div
      ref={row}
      role="option"
      aria-selected={props.selected}
      onMouseDown={(event) => event.preventDefault()}
      onClick={props.onSelect}
      className="flex cursor-default items-center gap-3 rounded-[10px] border-l-2 border-transparent px-3 py-2 hover:bg-hover/60 aria-selected:border-accent aria-selected:bg-accent-soft"
    >
      <Thumbnail
        path={result.path}
        kind={result.kind}
        connected={result.connected}
        load={props.thumbnails}
      />
      <span className="flex min-w-0 flex-1 flex-col gap-0.5">
        <span className="flex min-w-0 items-baseline gap-2">
          <span className="truncate font-medium">{result.name}</span>
          {result.location && (
            <span className="shrink-0 rounded-full border border-line bg-raised px-2 font-mono text-[11px] tabular-nums text-muted">
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
        {!result.connected && <span className="text-xs text-warning">Drive not connected</span>}
      </span>
    </div>
  );
}

/**
 * A ref for a list row that scrolls the row into view while it is selected,
 * so the arrow keys never move the selection out of sight.
 */
function useSelectedInView(selected: boolean) {
  const row = useRef<HTMLDivElement>(null);
  useEffect(() => {
    // `nearest`: a row already in view stays put, one out of view moves just enough.
    if (selected) row.current?.scrollIntoView?.({ block: "nearest" });
  }, [selected]);
  return row;
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
function status(typed: string, searching: boolean, found: number, weaker: number): string {
  if (!typed.trim()) return "Type what you remember about a file";
  if (searching) return "Searching…";
  if (found + weaker === 0) return "Nothing matches yet";
  const results = found === 1 ? "1 result" : `${found} results`;
  return weaker > 0 ? `${results}, ${weaker} weaker` : results;
}

/** Queries the empty launcher offers to try: by meaning, by kind of file, by time. */
const EXAMPLES = [
  "the dentist invoice from March",
  "slow-cooked beef recipe",
  "ext:pdf contract renewal",
  "date:week presentation",
];

/** What the Preview Pane offers while nothing is typed: examples, and searches from before. */
function Suggestions({ recent, onPick }: { recent: string[]; onPick: (typed: string) => void }) {
  return (
    <div className="flex flex-col gap-5 px-1 pt-1">
      <div className="flex flex-col gap-2">
        <p className="px-2 pt-1.5 text-xs font-semibold text-muted">Try describing it</p>
        <ul aria-label="Examples" className="flex flex-wrap gap-1.5 px-2">
          {EXAMPLES.map((example) => (
            <li key={example}>
              <button
                type="button"
                onMouseDown={(event) => event.preventDefault()}
                onClick={() => onPick(example)}
                className="rounded-full border border-line bg-panel/50 px-3 py-1 text-[13px] transition-colors hover:border-accent/50 hover:bg-accent-soft hover:text-ink"
              >
                {example}
              </button>
            </li>
          ))}
        </ul>
      </div>
      {recent.length > 0 && (
        <section className="flex flex-col gap-2">
          <h2 id="recent-searches" className="px-2 text-xs font-semibold text-muted">
            Recent searches
          </h2>
          <ul aria-labelledby="recent-searches" className="flex flex-col gap-0.5">
            {recent.map((typed) => (
              <li key={typed}>
                <button
                  type="button"
                  onMouseDown={(event) => event.preventDefault()}
                  onClick={() => onPick(typed)}
                  className="flex w-full items-center gap-2.5 rounded-[10px] px-3 py-1.5 text-left text-sm transition-colors hover:bg-hover/60"
                >
                  <History aria-hidden className="size-4 shrink-0 text-muted" strokeWidth={1.75} />
                  <span className="truncate">{typed}</span>
                </button>
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}

/** The Filter Prefixes, explained while nothing is typed. */
const TIPS: [string, string][] = [
  ["ext:pdf", "Only one kind of file; several mean any of them"],
  ["in:taxes", "Only in folders whose name has these words"],
  ["date:month", "Changed today, this week, month, year, or in 2025-03"],
  ["code:", "Only source code and configs"],
  ["name:", "Match names only, not what is inside"],
];

function FilterTips() {
  return (
    <section aria-labelledby="filter-tips" className="flex flex-col gap-3 pt-1">
      <h2 id="filter-tips" className="text-sm font-semibold">
        Narrow it down
      </h2>
      <p className="text-[13px] text-muted">
        Add these anywhere in what you type. They combine, like{" "}
        <code className="font-mono text-ink">ext:pdf date:year invoice</code>.
      </p>
      <dl className="flex flex-col divide-y divide-line/70 rounded-xl border border-line bg-panel/40">
        {TIPS.map(([prefix, meaning]) => (
          <div key={prefix} className="flex items-baseline gap-3 px-3.5 py-2">
            <dt className="w-24 shrink-0">
              <code className="rounded-md bg-raised px-1.5 py-0.5 font-mono text-xs text-accent">
                {prefix}
              </code>
            </dt>
            <dd className="text-[13px] text-muted">{meaning}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

/** Where the searches whose Results were used are kept, newest first. */
const RECENT = "recent-searches";
/** How many recent searches are kept. */
const RECENT_KEPT = 5;

/** The searches whose Results were used, newest first. */
function recentSearches(memory: Memory | undefined): string[] {
  try {
    const kept: unknown = JSON.parse(recall(memory, RECENT) ?? "[]");
    return Array.isArray(kept) ? kept.filter((typed) => typeof typed === "string") : [];
  } catch {
    return [];
  }
}

function rememberSearch(typed: string, memory: Memory | undefined) {
  const search = typed.trim();
  if (!search) return;
  const kept = [search, ...recentSearches(memory).filter((other) => other !== search)];
  try {
    memory?.setItem(RECENT, JSON.stringify(kept.slice(0, RECENT_KEPT)));
  } catch {
    // Storage full or blocked: the search is simply not offered again.
  }
}

/** `index` kept within a list of `length` items. */
const PICKED = "picked-copy ";

/** Which of `paths` was picked most recently; the first when none was. */
function lastPicked(paths: string[], memory: Memory | undefined): number {
  let best = 0;
  let newest = 0;
  paths.forEach((path, index) => {
    const picked = Number(recall(memory, PICKED + path));
    if (picked > newest) {
      newest = picked;
      best = index;
    }
  });
  return best;
}

function rememberPicked(path: string, memory: Memory | undefined) {
  try {
    memory?.setItem(PICKED + path, String(Date.now()));
  } catch {
    // Storage full or blocked: picking simply starts on the first Copy again.
  }
}

function recall(memory: Memory | undefined, key: string): string | null {
  try {
    return memory?.getItem(key) ?? null;
  } catch {
    return null;
  }
}

function clamp(index: number, length: number): number {
  return Math.max(0, Math.min(index, length - 1));
}
