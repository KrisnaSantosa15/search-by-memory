import { useCallback, useEffect, useRef } from "react";

/** A rendered PDF page, as the Rust side sends it. */
export type PdfPagePreview = {
  /** The page as a PNG `data:` URL. */
  image: string;
  /** Where the Query's words sit, in fractions of the page from its top left. */
  marks: { left: number; top: number; width: number; height: number }[];
};

/** Renders page `page` (1-based) of a PDF, marking the words of `query`. */
export type LoadPdfPage = (
  path: string,
  page: number,
  query: string,
) => Promise<PdfPagePreview | null>;

type Props = {
  preview: PdfPagePreview;
  /** The file's name, for the picture's description. */
  name: string;
  page: number;
};

/** The page itself, with highlighter marks over the Query's words. */
export function PdfPage({ preview, name, page }: Props) {
  const first = useRef<HTMLElement>(null);
  const firstIndex = topmost(preview.marks);
  // Marks sit in fractions of the picture, so scroll again once it has its height.
  const scrollToFirst = useCallback(
    () => first.current?.scrollIntoView?.({ block: "center", inline: "nearest" }),
    [],
  );
  useEffect(scrollToFirst, [preview, scrollToFirst]);

  return (
    <figure className="relative overflow-hidden rounded-md bg-white shadow-lg shadow-black/30">
      <div className="relative max-h-64 overflow-y-auto">
        <div className="relative">
          <img
            src={preview.image}
            alt={`Page ${page} of ${name}`}
            className="block w-full"
            onLoad={scrollToFirst}
          />
          {preview.marks.map((mark, index) => (
            <mark
              key={index}
              ref={index === firstIndex ? first : undefined}
              aria-hidden
              className="absolute rounded-sm mix-blend-multiply"
              style={{
                left: `${mark.left * 100}%`,
                top: `${mark.top * 100}%`,
                width: `${mark.width * 100}%`,
                height: `${mark.height * 100}%`,
                background: "rgb(227 166 70 / 0.55)",
              }}
            />
          ))}
        </div>
      </div>
      <figcaption className="absolute bottom-2.5 left-3 rounded bg-canvas/80 px-1.5 text-xs text-ink">
        Page {page}
      </figcaption>
    </figure>
  );
}

/** Where reading the page meets a mark first: top to bottom, then left to right. */
function topmost(marks: PdfPagePreview["marks"]): number {
  let best = -1;
  marks.forEach((mark, index) => {
    const other = marks[best];
    if (!other || mark.top < other.top || (mark.top === other.top && mark.left < other.left)) {
      best = index;
    }
  });
  return best;
}
