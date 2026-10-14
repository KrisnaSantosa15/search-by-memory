import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  Settings,
  type Dashboard,
  type ExclusionList,
  type ExclusionRow,
  type FailedFile,
  type IndexedFolder,
} from "./Settings";

// 15 March 2025, midday UTC: the same month in every time zone.
const midMarch2025 = Date.UTC(2025, 2, 15, 12);

const running: Dashboard = {
  files: 12_345,
  indexed: 340,
  pending: 660,
  readySince: midMarch2025,
  state: "Indexing: 660 files to go",
  paused: false,
  sizeOnDisk: 1_234_567_890,
};

function renderSettings(overrides: Partial<Parameters<typeof Settings>[0]> = {}) {
  const props = {
    dashboard: async () => running,
    failedFiles: async () => [] as FailedFile[],
    folders: async () => [] as IndexedFolder[],
    addFolder: vi.fn(async () => false),
    removeFolder: vi.fn(async (_path: string) => {}),
    exclusions: async (): Promise<ExclusionList> => ({ rows: [], defaultsTurnedOff: false }),
    removeExclusion: vi.fn(async (_row: ExclusionRow) => {}),
    restoreDefaults: vi.fn(async () => {}),
    excludeNames: vi.fn(async (_pattern: string) => {}),
    excludeFolder: vi.fn(async () => false),
    pause: vi.fn(async () => {}),
    resume: vi.fn(async () => {}),
    ...overrides,
  };
  render(<Settings {...props} />);
  return props;
}

describe("Settings", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("shows the files found and how far content search reaches", async () => {
    renderSettings();

    expect(await screen.findByText("12,345 files found")).toBeInTheDocument();
    expect(
      screen.getByText("Content index 34% — files since March 2025 are ready"),
    ).toBeInTheDocument();
  });

  it("shows the content index as complete before any file is found", async () => {
    renderSettings({
      dashboard: async () => ({ ...running, files: 0, indexed: 0, pending: 0, readySince: null }),
    });

    expect(await screen.findByText("Content index 100%")).toBeInTheDocument();
  });

  it("shows how much disk the index takes", async () => {
    renderSettings();

    expect(await screen.findByText("The index takes 1.2 GB on disk")).toBeInTheDocument();
  });

  it("keeps the status up to date while it is open", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let files = 10;
    renderSettings({ dashboard: async () => ({ ...running, files }) });
    expect(await screen.findByText("10 files found")).toBeInTheDocument();

    files = 25;
    await vi.advanceTimersByTimeAsync(2_000);

    expect(await screen.findByText("25 files found")).toBeInTheDocument();
  });

  it("says why indexing waits", async () => {
    renderSettings({
      dashboard: async () => ({
        ...running,
        state: "Indexing waits while a full-screen app is open",
      }),
    });

    expect(
      await screen.findByText("Indexing waits while a full-screen app is open"),
    ).toBeInTheDocument();
  });

  it("lists the files whose content could not be indexed, and why", async () => {
    renderSettings({
      failedFiles: async () => [
        {
          path: "C:\\Users\\ana\\Documents\\locked.pdf",
          reason: "the PDF is password-protected",
        },
      ],
    });

    expect(await screen.findByText("1 file could not be read")).toBeInTheDocument();
    expect(screen.getByText("locked.pdf")).toBeInTheDocument();
    expect(screen.getByText("the PDF is password-protected")).toBeInTheDocument();
  });

  it("pauses indexing, then offers to resume it", async () => {
    let paused = false;
    const props = renderSettings({
      dashboard: async () => ({
        ...running,
        paused,
        state: paused ? "Indexing is paused" : running.state,
      }),
      pause: vi.fn(async () => {
        paused = true;
      }),
    });

    await userEvent.click(await screen.findByRole("button", { name: "Pause indexing" }));

    expect(props.pause).toHaveBeenCalled();
    expect(await screen.findByText("Indexing is paused")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Resume indexing" }));
    expect(props.resume).toHaveBeenCalled();
  });

  it("lists the Indexed Folders and says which drives are disconnected", async () => {
    renderSettings({
      folders: async () => [
        { path: "C:\\Users\\ana", connected: true },
        { path: "E:\\Photos", connected: false },
      ],
    });

    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    const list = screen.getByRole("list", { name: "Indexed Folders" });
    const items = within(list).getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("C:\\Users\\ana");
    expect(items[0]).not.toHaveTextContent("not connected");
    expect(items[1]).toHaveTextContent("E:\\Photos");
    expect(items[1]).toHaveTextContent("Drive not connected");
  });

  it("adds a folder the user picks and lists it", async () => {
    let list: IndexedFolder[] = [];
    const props = renderSettings({
      folders: async () => list,
      addFolder: vi.fn(async () => {
        list = [{ path: "D:\\Projects", connected: true }];
        return true;
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(screen.getByRole("button", { name: "Add folder…" }));

    expect(props.addFolder).toHaveBeenCalled();
    expect(await screen.findByText("D:\\Projects")).toBeInTheDocument();
  });

  it("removes a folder once the user confirms it", async () => {
    let list: IndexedFolder[] = [
      { path: "C:\\Users\\ana", connected: true },
      { path: "D:\\Projects", connected: true },
    ];
    const props = renderSettings({
      folders: async () => list,
      removeFolder: vi.fn(async (path: string) => {
        list = list.filter((folder) => folder.path !== path);
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(await screen.findByRole("button", { name: "Remove D:\\Projects" }));
    expect(props.removeFolder).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Stop searching it" }));

    expect(props.removeFolder).toHaveBeenCalledWith("D:\\Projects");
    await waitFor(() => expect(screen.queryByText("D:\\Projects")).not.toBeInTheDocument());
    expect(screen.getByText("C:\\Users\\ana")).toBeInTheDocument();
  });

  it("keeps a folder when the user changes their mind", async () => {
    const props = renderSettings({
      folders: async () => [{ path: "D:\\Projects", connected: true }],
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(await screen.findByRole("button", { name: "Remove D:\\Projects" }));
    await userEvent.click(screen.getByRole("button", { name: "Keep" }));

    expect(props.removeFolder).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Remove D:\\Projects" })).toBeInTheDocument();
  });

  it("lists the Exclusions with the defaults marked", async () => {
    renderSettings({
      exclusions: async () => ({
        rows: [
          { kind: "name", pattern: "node_modules", default: true },
          { kind: "name", pattern: "*.bak", default: false },
          { kind: "folder", path: "C:\\Users\\ana\\private" },
        ],
        defaultsTurnedOff: false,
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    const list = await screen.findByRole("list", { name: "Exclusions" });
    const items = within(list).getAllByRole("listitem");
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent("node_modules");
    expect(items[0]).toHaveTextContent("Default");
    expect(items[1]).toHaveTextContent("*.bak");
    expect(items[1]).not.toHaveTextContent("Default");
    expect(items[2]).toHaveTextContent("C:\\Users\\ana\\private");
    expect(screen.queryByRole("button", { name: "Restore defaults" })).not.toBeInTheDocument();
  });

  it("removes the user's Exclusions and turns off default ones", async () => {
    const nodeModules: ExclusionRow = { kind: "name", pattern: "node_modules", default: true };
    const backups: ExclusionRow = { kind: "name", pattern: "*.bak", default: false };
    let rows = [nodeModules, backups];
    const props = renderSettings({
      exclusions: async () => ({ rows, defaultsTurnedOff: false }),
      removeExclusion: vi.fn(async (row: ExclusionRow) => {
        rows = rows.filter((other) => other !== row);
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(await screen.findByRole("button", { name: "Remove *.bak" }));
    await userEvent.click(screen.getByRole("button", { name: "Turn off node_modules" }));

    expect(props.removeExclusion).toHaveBeenCalledWith(backups);
    expect(props.removeExclusion).toHaveBeenCalledWith(nodeModules);
    await waitFor(() =>
      expect(within(screen.getByRole("list", { name: "Exclusions" })).queryAllByRole("listitem"))
        .toHaveLength(0),
    );
  });

  it("restores the defaults once one is turned off", async () => {
    let turnedOff = true;
    const props = renderSettings({
      exclusions: async () => ({
        rows: turnedOff ? [] : [{ kind: "name", pattern: "node_modules", default: true }],
        defaultsTurnedOff: turnedOff,
      }),
      restoreDefaults: vi.fn(async () => {
        turnedOff = false;
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(await screen.findByRole("button", { name: "Restore defaults" }));

    expect(props.restoreDefaults).toHaveBeenCalled();
    expect(await screen.findByText("node_modules")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Restore defaults" })).not.toBeInTheDocument();
  });

  it("excludes the names the user types", async () => {
    let rows: ExclusionRow[] = [];
    const props = renderSettings({
      exclusions: async () => ({ rows, defaultsTurnedOff: false }),
      excludeNames: vi.fn(async (pattern: string) => {
        rows = [{ kind: "name", pattern, default: false }];
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    const box = screen.getByRole("textbox", { name: "Exclude names like" });
    expect(screen.getByRole("button", { name: "Exclude" })).toBeDisabled();
    await userEvent.type(box, "  *.bak {Enter}");

    expect(props.excludeNames).toHaveBeenCalledWith("*.bak");
    expect(await screen.findByText("*.bak")).toBeInTheDocument();
    expect(box).toHaveValue("");
  });

  it("excludes a folder the user picks", async () => {
    let rows: ExclusionRow[] = [];
    const props = renderSettings({
      exclusions: async () => ({ rows, defaultsTurnedOff: false }),
      excludeFolder: vi.fn(async () => {
        rows = [{ kind: "folder", path: "C:\\Users\\ana\\private" }];
        return true;
      }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(screen.getByRole("button", { name: "Exclude a folder…" }));

    expect(props.excludeFolder).toHaveBeenCalled();
    expect(await screen.findByText("C:\\Users\\ana\\private")).toBeInTheDocument();
  });
});
