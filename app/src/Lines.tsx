import { useEffect, useRef } from "react";
import { Marked } from "./Marked";

/** Some lines of a text file, as the Rust side reads them. */
export type TextPreview = {
  /** The 1-based number of `lines[0]`. */
  firstLine: number;
  lines: string[];
};

/** Reads the lines of a text file around the matched ones, or its first lines. */
export type LoadLines = (
  path: string,
  lineStart: number | null,
  lineEnd: number | null,
) => Promise<TextPreview | null>;

type Props = {
  preview: TextPreview;
  query: string;
  /** The matched lines, 1-based and inclusive; `null` without a Content Match. */
  matched: [number, number] | null;
};

/** Code as an editor shows it: numbered lines, the matched ones lit. */
export function CodeLines(props: Props) {
  return (
    <LineList
      {...props}
      numbered
      className="max-h-72 overflow-auto rounded-md border border-line bg-canvas py-2 font-mono text-[12.5px] leading-relaxed"
      lineClassName="flex whitespace-pre aria-[current]:bg-accent-soft"
    />
  );
}

/** Lines of a text document, as on paper: the matched ones in full ink, the rest faded. */
export function PageLines(props: Props) {
  return (
    <LineList
      {...props}
      className="max-h-60 overflow-auto"
      lineClassName="min-h-[1lh] whitespace-pre-wrap opacity-55 aria-[current]:opacity-100"
    />
  );
}

/** The lines, the matched ones marked as current and scrolled into view. */
function LineList(
  props: Props & { numbered?: boolean; className: string; lineClassName: string },
) {
  const { preview, query, matched } = props;
  const firstMatched = useRef<HTMLLIElement>(null);

  useEffect(() => {
    firstMatched.current?.scrollIntoView?.({ block: "nearest" });
  }, [preview]);

  return (
    <ol aria-label="Lines" className={props.className}>
      {preview.lines.map((line, offset) => {
        const number = preview.firstLine + offset;
        const lit = matched !== null && number >= matched[0] && number <= matched[1];
        return (
          <li
            key={number}
            value={number}
            aria-current={lit || undefined}
            ref={lit && number === matched[0] ? firstMatched : undefined}
            className={props.lineClassName}
          >
            {props.numbered ? (
              <>
                <span aria-hidden className="w-12 shrink-0 select-none pr-4 text-right text-muted/60">
                  {number}
                </span>
                <span className="pr-4">
                  <Marked text={line} query={query} />
                </span>
              </>
            ) : (
              <Marked text={line} query={query} />
            )}
          </li>
        );
      })}
    </ol>
  );
}
