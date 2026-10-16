import { Folder, HardDrive } from "lucide-react";
import { useEffect, useState } from "react";
import type { IndexedFolder } from "../Settings";

/** The Indexed Folders under the sidebar's pages, as VoiceStudio lists saved voices there. */
export function SidebarFolders(props: {
  folders: () => Promise<IndexedFolder[]>;
  /** Asks again whenever this changes, such as the page on screen. */
  refreshKey: string;
  onOpen: () => void;
}) {
  const { folders, refreshKey } = props;
  const [list, setList] = useState<IndexedFolder[]>([]);

  useEffect(() => {
    folders().then(setList);
  }, [folders, refreshKey]);

  if (list.length === 0) return null;
  return (
    <div className="flex min-h-0 flex-col gap-1 px-3 pt-5">
      <p id="searching-in" className="px-2.5 pb-1 text-[11px] font-semibold tracking-wide text-muted uppercase">
        Searching in
      </p>
      <ul aria-labelledby="searching-in" className="flex min-h-0 flex-col gap-0.5 overflow-y-auto">
        {list.map(({ path, connected }) => {
          const Icon = isDrive(path) ? HardDrive : Folder;
          return (
            <li key={path}>
              <button
                type="button"
                title={path}
                onClick={props.onOpen}
                className="flex h-8 w-full items-center gap-2.5 rounded-[10px] px-2.5 text-left text-[13px] text-muted transition-colors hover:bg-hover/60 hover:text-ink"
              >
                <Icon aria-hidden className="size-4 shrink-0" strokeWidth={1.75} />
                <span className="min-w-0 flex-1 truncate">{lastPart(path)}</span>
                {!connected && (
                  <span className="shrink-0 rounded-full bg-raised px-1.5 text-[10px] text-warning">
                    Not connected
                  </span>
                )}
              </button>
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** A whole drive, such as `D:\`, rather than a folder on it. */
function isDrive(path: string): boolean {
  return /^[A-Za-z]:[\\/]?$/.test(path) || path === "/";
}

/** The folder's own name; a drive's letter. */
function lastPart(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  return trimmed.split(/[\\/]/).pop() || path;
}
