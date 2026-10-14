import type { ReactNode } from "react";

/**
 * `text` with the Query's words marked. Words match whole, ignoring case; the
 * last Query word is still being typed, so it also marks the start of longer words.
 */
export function Marked({ text, query }: { text: string; query: string }) {
  const words: string[] = query.toLowerCase().match(WORD) ?? [];
  const typing = words.at(-1);
  const parts: ReactNode[] = [];
  let start = 0;
  for (const match of text.matchAll(WORD)) {
    const word = match[0].toLowerCase();
    const length = words.includes(word)
      ? word.length
      : typing && word.startsWith(typing)
        ? typing.length
        : 0;
    if (length === 0) continue;
    parts.push(text.slice(start, match.index));
    parts.push(<mark key={match.index}>{text.slice(match.index, match.index + length)}</mark>);
    start = match.index + length;
  }
  parts.push(text.slice(start));
  return <>{parts}</>;
}

/** Letters and digits in any script. */
const WORD = /[\p{L}\p{N}]+/gu;
