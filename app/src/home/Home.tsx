import { Activity, FolderOpen, House, X } from "lucide-react";
import { useState } from "react";
import { ActivityPage, FoldersPage, type FoldersProps, type StatusProps } from "../Settings";
import { Overview } from "./Overview";

const PAGES = [
  { name: "Home", Icon: House },
  { name: "Activity", Icon: Activity },
  { name: "Folders", Icon: FolderOpen },
] as const;

type WindowProps = {
  /** Closes the Home window, which has no frame of its own. */
  close: () => void;
};

export type HomeProps = StatusProps & FoldersProps & WindowProps;

/** The Home window: indexing status, Indexed Folders, and Exclusions. */
export function Home(props: HomeProps) {
  const [page, setPage] = useState<(typeof PAGES)[number]["name"]>("Home");

  return (
    <div className="flex h-full flex-col">
      {/* The window has no frame: this strip moves it, and closes it. */}
      <header
        data-tauri-drag-region
        className="flex h-10 shrink-0 items-center justify-between border-b border-line pl-5"
      >
        <span data-tauri-drag-region className="text-xs text-muted">
          Search by Memory
        </span>
        <button
          type="button"
          aria-label="Close"
          onClick={props.close}
          className="grid h-10 w-12 place-items-center text-muted transition-colors hover:bg-danger/80 hover:text-white"
        >
          <X aria-hidden className="size-4" strokeWidth={1.75} />
        </button>
      </header>
      <div className="grid min-h-0 flex-1 grid-cols-[200px_minmax(0,1fr)]">
        <nav className="border-r border-line bg-chrome/50 p-3">
          <div role="tablist" aria-orientation="vertical" className="flex flex-col gap-0.5">
            {PAGES.map(({ name, Icon }) => (
              <button
                key={name}
                type="button"
                role="tab"
                aria-selected={page === name}
                onClick={() => setPage(name)}
                className="flex h-9 items-center gap-2.5 rounded-[10px] px-2.5 text-sm text-muted transition-colors hover:bg-hover/60 hover:text-ink aria-selected:bg-hover aria-selected:font-medium aria-selected:text-ink"
              >
                <Icon aria-hidden className="size-4" strokeWidth={1.75} />
                {name}
              </button>
            ))}
          </div>
        </nav>
        <div role="tabpanel" aria-label={page} className="min-h-0 overflow-y-auto">
          <div className="mx-auto flex max-w-2xl flex-col gap-6 px-8 pt-6 pb-12">
            {page === "Home" ? (
              <Overview {...props} />
            ) : page === "Activity" ? (
              <ActivityPage {...props} />
            ) : (
              <FoldersPage {...props} />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
