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
  return (
    <figure className="relative overflow-hidden rounded-md bg-white shadow-lg shadow-black/30">
      <div className="relative max-h-64 overflow-y-auto">
        <div className="relative">
          <img src={preview.image} alt={`Page ${page} of ${name}`} className="block w-full" />
          {preview.marks.map((mark, index) => (
            <mark
              key={index}
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
