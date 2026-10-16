import { Activity, FolderOpen, House, Settings as SettingsIcon, X } from "lucide-react";
import { useState } from "react";
import { ActivityPage, FoldersPage, type FoldersProps, type StatusProps } from "../Settings";
import { BrandMark } from "../ui/BrandMark";
import { EngineMonitor, type EngineMonitorProps } from "./EngineMonitor";
import { Overview, type OverviewProps } from "./Overview";
import { SettingsPage, type SettingsPageProps } from "./SettingsPage";
import { SidebarFolders } from "./SidebarFolders";

const PAGES = [
  { name: "Home", Icon: House },
  { name: "Activity", Icon: Activity },
  { name: "Folders", Icon: FolderOpen },
  { name: "Settings", Icon: SettingsIcon },
] as const;

type Page = (typeof PAGES)[number]["name"];

/** Settings sits apart, at the foot of the sidebar below the Engine, as in VoiceStudio. */
const FOOT_PAGE: Page = "Settings";

type WindowProps = {
  /** Closes the Home window, which has no frame of its own. */
  close: () => void;
};

export type HomeProps = StatusProps &
  FoldersProps &
  OverviewProps &
  SettingsPageProps &
  EngineMonitorProps &
  WindowProps;

/** The Home window: what search can do now, how far indexing has got, and the folders it covers. */
export function Home(props: HomeProps) {
  const [page, setPage] = useState<Page>("Home");
  const tab = ({ name, Icon }: (typeof PAGES)[number]) => (
    <button
      key={name}
      type="button"
      role="tab"
      aria-selected={page === name}
      onClick={() => setPage(name)}
      className="group flex h-9 items-center gap-2.5 rounded-[10px] px-2.5 text-sm text-muted transition-colors hover:bg-hover/60 hover:text-ink aria-selected:bg-hover aria-selected:font-medium aria-selected:text-ink"
    >
      <Icon
        aria-hidden
        className="size-4 transition-colors group-aria-selected:text-accent"
        strokeWidth={1.75}
      />
      {name}
    </button>
  );

  return (
    <div className="grid h-full min-h-0 grid-cols-[232px_minmax(0,1fr)]">
      <aside className="flex min-h-0 flex-col border-r border-line bg-chrome/50">
        {/* The window has no frame: the brand row moves it, like the title strip. */}
        <div data-tauri-drag-region className="flex h-12 shrink-0 items-center gap-2.5 px-4">
          <BrandMark className="size-6 shrink-0" />
          <span data-tauri-drag-region className="text-sm font-semibold tracking-[-0.01em]">
            Search by Memory
          </span>
        </div>
        <nav className="px-3 pt-2">
          <div role="tablist" aria-orientation="vertical" className="flex flex-col gap-0.5">
            {PAGES.filter(({ name }) => name !== FOOT_PAGE).map(tab)}
          </div>
        </nav>
        <SidebarFolders
          folders={props.folders}
          refreshKey={page}
          onOpen={() => setPage("Folders")}
        />
        <div className="mt-auto flex flex-col gap-2 p-3">
          <EngineMonitor
            dashboard={props.dashboard}
            deviceUsage={props.deviceUsage}
            onOpen={setPage}
          />
          <div role="tablist" aria-orientation="vertical" className="flex flex-col">
            {PAGES.filter(({ name }) => name === FOOT_PAGE).map(tab)}
          </div>
        </div>
      </aside>
      <div className="flex min-h-0 flex-col">
        <header data-tauri-drag-region className="flex h-12 shrink-0 items-center justify-between pl-6">
          <span data-tauri-drag-region className="text-sm font-medium">
            {page}
          </span>
          <button
            type="button"
            aria-label="Close"
            onClick={props.close}
            className="grid h-12 w-12 place-items-center text-muted transition-colors hover:bg-danger/80 hover:text-white"
          >
            <X aria-hidden className="size-4" strokeWidth={1.75} />
          </button>
        </header>
        <div role="tabpanel" aria-label={page} className="min-h-0 flex-1 overflow-y-auto">
          <div className="mx-auto flex max-w-4xl flex-col gap-6 px-8 pt-4 pb-12">
            {page === "Home" ? (
              <Overview {...props} />
            ) : page === "Activity" ? (
              <ActivityPage {...props} />
            ) : page === "Folders" ? (
              <FoldersPage {...props} />
            ) : (
              <SettingsPage {...props} />
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
