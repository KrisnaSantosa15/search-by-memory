import { AudioLines, Clapperboard, File, FileCode2, FileText, Image, type LucideIcon } from "lucide-react";
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
        className="size-10 shrink-0 rounded-lg border border-line bg-raised object-cover"
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

/** What kind of file it is, as a tinted icon on a tile, the way VoiceStudio tints its places. */
const KIND_ICONS: Record<Kind, { Icon: LucideIcon; tint: string }> = {
  document: { Icon: FileText, tint: "text-sky-600 dark:text-sky-400" },
  code: { Icon: FileCode2, tint: "text-emerald-600 dark:text-emerald-400" },
  image: { Icon: Image, tint: "text-fuchsia-600 dark:text-fuchsia-400" },
  audio: { Icon: AudioLines, tint: "text-amber-600 dark:text-amber-400" },
  video: { Icon: Clapperboard, tint: "text-violet-600 dark:text-violet-400" },
  other: { Icon: File, tint: "text-muted" },
};

function KindIcon({ kind }: { kind: Kind }) {
  const { Icon, tint } = KIND_ICONS[kind];
  return (
    <span
      role="img"
      aria-label={KIND_NAMES[kind]}
      className={`grid size-10 shrink-0 place-items-center rounded-lg border border-line bg-raised ${tint}`}
    >
      <Icon aria-hidden className="size-5" strokeWidth={1.75} />
    </span>
  );
}
