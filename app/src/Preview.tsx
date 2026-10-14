import { type ReactNode, useEffect, useState } from "react";
import type { ResultRow } from "./Launcher";
import { CodeLines, type LoadLines, PageLines, type TextPreview } from "./Lines";
import { Marked } from "./Marked";
import { Path } from "./Path";
import { type LoadPdfPage, PdfPage, type PdfPagePreview } from "./PdfPage";
import type { LoadThumbnail } from "./Thumbnail";

/** How large a picture or video is asked for: the Preview Pane's width at 2× scaling. */
const PICTURE_SIZE = 840;

const MATCH_REASONS = {
  name: "Its name matches",
  content: "What is inside matches",
  both: "Name and contents match",
};

type Props = {
  result: ResultRow;
  query: string;
  loadLines: LoadLines;
  loadPdfPage: LoadPdfPage;
  loadPicture: LoadThumbnail;
  /** The Copy being picked; `null` while none is being picked. */
  copy: number | null;
  onOpen: () => void;
  onReveal: () => void;
};

/** The selected Result in full: its Content Match, where it lives, and why it matched. */
export function Preview(props: Props) {
  const { result, query, loadLines, copy, onOpen, onReveal } = props;
  const paths = [result.path, ...result.copies];
  const lines = useLines(result, loadLines);
  const pdfPage = usePdfPage(result, query, props.loadPdfPage);
  const picture = usePicture(result, props.loadPicture);
  const { lineStart, lineEnd } = result;
  const matched: [number, number] | null =
    lineStart !== null && lineEnd !== null ? [lineStart, lineEnd] : null;
  const facts = [
    result.size !== null && size(result.size),
    result.modified !== null && `changed ${date(result.modified)}`,
  ].filter(Boolean);

  return (
    <section aria-label="Preview" className="flex min-w-0 flex-col gap-4">
      {pdfPage && result.page !== null ? (
        <PdfPage preview={pdfPage} name={result.name} page={result.page} />
      ) : picture ? (
        <img
          src={picture}
          alt={result.name}
          className="max-h-72 self-center rounded-md object-contain shadow-lg shadow-black/30"
        />
      ) : lines && result.kind === "code" ? (
        <CodeLines preview={lines} query={query} matched={matched} />
      ) : lines ? (
        <Page location={result.location}>
          <PageLines preview={lines} query={query} matched={matched} />
        </Page>
      ) : (
        result.snippet && (
          <Page location={result.location}>
            <blockquote className="line-clamp-[9]">
              <Marked text={result.snippet} query={query} />
            </blockquote>
          </Page>
        )
      )}
      <div className="flex flex-col gap-0.5">
        <h2 className="truncate font-display text-lg font-semibold">{result.name}</h2>
        <p className="flex text-sm text-muted">
          <Path path={result.folder} />
        </p>
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
                className="flex rounded-md border-l-2 border-transparent px-2 py-1 text-sm aria-selected:border-accent aria-selected:bg-accent-soft"
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

/**
 * The lines to preview of a text file on a connected drive; `null` until they
 * are read, and for other files.
 */
function useLines(result: ResultRow, loadLines: LoadLines): TextPreview | null {
  const [lines, setLines] = useState<TextPreview | null>(null);
  const textual =
    result.kind === "code" || (result.kind === "document" && result.lineStart !== null);
  const readable = textual && result.connected;
  const { path, lineStart, lineEnd } = result;

  useEffect(() => {
    setLines(null);
    if (!readable) return;
    // Only the selected Result's lines are shown, whichever answers last.
    let current = true;
    loadLines(path, lineStart, lineEnd).then((lines) => current && setLines(lines));
    return () => {
      current = false;
    };
  }, [loadLines, path, lineStart, lineEnd, readable]);

  return lines;
}

/**
 * The matched page of a PDF on a connected drive; `null` until it is rendered,
 * and for other files.
 */
function usePdfPage(result: ResultRow, query: string, load: LoadPdfPage): PdfPagePreview | null {
  const [page, setPage] = useState<PdfPagePreview | null>(null);
  const { path, page: number, connected } = result;
  const renderable = number !== null && connected && path.toLowerCase().endsWith(".pdf");

  useEffect(() => {
    setPage(null);
    if (!renderable || number === null) return;
    // Only the selected Result's page is shown, whichever answers last.
    let current = true;
    load(path, number, query).then((page) => current && setPage(page));
    return () => {
      current = false;
    };
  }, [load, path, number, query, renderable]);

  return page;
}

/**
 * A large picture of an image or video on a connected drive; `null` until the
 * system gives one, and for other files.
 */
function usePicture(result: ResultRow, load: LoadThumbnail): string | null {
  const [picture, setPicture] = useState<string | null>(null);
  const { path, connected } = result;
  const visual = (result.kind === "image" || result.kind === "video") && connected;

  useEffect(() => {
    setPicture(null);
    if (!visual) return;
    // Only the selected Result's picture is shown, whichever answers last.
    let current = true;
    load(path, PICTURE_SIZE).then((picture) => current && setPicture(picture));
    return () => {
      current = false;
    };
  }, [load, path, visual]);

  return picture;
}

/** Text on a sheet of paper, where the highlighter marks show best. */
function Page(props: { location: string | null; children: ReactNode }) {
  return (
    <figure className="page relative rounded-md bg-ink px-5 pt-4 pb-9 text-[13px] leading-relaxed text-canvas shadow-lg shadow-black/30">
      {props.children}
      {props.location && (
        <figcaption className="absolute bottom-2.5 left-3 rounded bg-canvas/80 px-1.5 text-xs text-ink">
          {props.location}
        </figcaption>
      )}
    </figure>
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
