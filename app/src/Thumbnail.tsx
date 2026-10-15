import { useEffect, useState } from "react";
import type { Kind } from "./Launcher";

/**
 * The system's thumbnail of a file, `size` pixels at most on its longer side,
 * as a `data:` URL; `null` when it has none.
 */
export type LoadThumbnail = (path: string, size: number) => Promise<string | null>;

/** How large a row's thumbnail is asked for: twice its drawn size, for sharp edges. */
const ROW_SIZE = 80;

/** `load`, asked once per path and size. */
export function cachedThumbnails(load: LoadThumbnail): LoadThumbnail {
  const known = new Map<string, Promise<string | null>>();
  return (path, size) => {
    const key = `${size} ${path}`;
    let thumbnail = known.get(key);
    if (!thumbnail) {
      thumbnail = load(path, size).catch(() => null);
      known.set(key, thumbnail);
    }
    return thumbnail;
  };
}

type Props = {
  path: string;
  kind: Kind;
  /** False on a Disconnected Drive: nothing to ask the system for. */
  connected: boolean;
  load: LoadThumbnail;
};

/** A small picture of the file: its thumbnail, or an icon for its kind. */
export function Thumbnail({ path, kind, connected, load }: Props) {
  const [picture, setPicture] = useState<string | null>(null);

  useEffect(() => {
    setPicture(null);
    if (!connected) return;
    let current = true;
    load(path, ROW_SIZE).then((picture) => current && setPicture(picture));
    return () => {
      current = false;
    };
  }, [path, connected, load]);

  if (picture) {
    return (
      <img
        src={picture}
        alt=""
        className="size-10 shrink-0 rounded border border-line bg-raised object-cover"
      />
    );
  }
  return <KindIcon kind={kind} />;
}

const KIND_NAMES: Record<Kind, string> = {
  document: "Document",
  code: "Code",
  image: "Image",
  audio: "Audio",
  video: "Video",
  other: "File",
};

/** A quiet drawing of what kind of file it is: a page, brackets, a note, a frame. */
function KindIcon({ kind }: { kind: Kind }) {
  return (
    <svg
      role="img"
      aria-label={KIND_NAMES[kind]}
      viewBox="0 0 40 40"
      className="size-10 shrink-0 text-muted"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.5"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <path d="M11 5h13l7 7v23H11z" className="fill-raised" />
      <path d="M24 5v7h7" />
      {kind === "document" && <path d="M15 19h12M15 23h12M15 27h8" />}
      {kind === "code" && <path d="m17 20-3 3 3 3m6-6 3 3-3 3" />}
      {kind === "image" && <path d="m14 29 5-6 4 4 3-3 3 5M24 19h.01" />}
      {kind === "audio" && <path d="M19 28v-9l7-2v9M19 28a2 2 0 1 1-4 0 2 2 0 0 1 4 0m7-2a2 2 0 1 1-4 0 2 2 0 0 1 4 0" />}
      {kind === "video" && <path d="m18 19 7 4-7 4z" />}
    </svg>
  );
}
