import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import type {
  Dashboard,
  ExclusionList,
  ExclusionRow,
  FailedFile,
  IndexedFolder,
} from "../Settings";
import type { DeviceUsage } from "./DevicePanel";
import { Home, type HomeProps } from "./Home";

// 15 March 2025, midday UTC: the same month in every time zone.
const midMarch2025 = Date.UTC(2025, 2, 15, 12);

const GIB = 2 ** 30;

const busyLaptop: DeviceUsage = {
  cpu: { percent: 12.4, ownPercent: 4.2 },
  processor: { name: "12th Gen Intel(R) Core(TM) i5-12450H", cores: 8, threads: 12, ghz: 2.5 },
  memory: { used: 16.2 * GIB, total: 31.7 * GIB, own: 0.8 * GIB },
  gpus: [
    {
      name: "NVIDIA GeForce RTX 4060",
      percent: 76,
      ownPercent: 40,
      memoryUsed: 4.7 * GIB,
      memoryTotal: 8 * GIB,
      ownMemory: 1.2 * GIB,
    },
  ],
};

const running: Dashboard = {
  files: 12_345,
  indexed: 340,
  pending: 660,
  readySince: midMarch2025,
  state: "Indexing: 660 files to go",
  paused: false,
  sizeOnDisk: 1_234_567_890,
  scanning: false,
  current: null,
  failed: 0,
  model: { state: "ready", device: "cuda" },
  download: null,
};

function renderHome(overrides: Partial<HomeProps> = {}) {
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
    purge: vi.fn(async () => {}),
    close: vi.fn(),
    retryFailed: vi.fn(async () => {}),
    settings: async () => ({ accelerator: "cuda", hotkey: "Ctrl+Shift+Space" }),
    openSearch: vi.fn((_typed: string) => {}),
    changeHotkey: vi.fn(async (_hotkey: string) => true),
    version: async () => "0.1.0",
    downloadModel: vi.fn(async () => {}),
    deviceUsage: async () => busyLaptop,
    ...overrides,
  };
  render(<Home {...props} />);
  return props;
}

/** Renders Home and goes to one of its pages. */
async function renderPage(page: string, overrides: Partial<HomeProps> = {}) {
  const props = renderHome(overrides);
  await userEvent.click(screen.getByRole("tab", { name: page }));
  return props;
}

describe("Home", () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it("closes from the close button, as the window has no frame of its own", async () => {
    const props = renderHome();

    await userEvent.click(screen.getByRole("button", { name: "Close" }));

    expect(props.close).toHaveBeenCalled();
  });

  it("opens on Home, which counts the files found, the share read, and the index's size", async () => {
    renderHome();

    const glance = await screen.findByRole("list", { name: "At a glance" });
    const tiles = within(glance).getAllByRole("listitem").map((tile) => tile.textContent);
    expect(screen.getByRole("tab", { name: "Home" })).toHaveAttribute("aria-selected", "true");
    expect(tiles).toEqual([
      expect.stringMatching(/12,345.*Files found/),
      expect.stringMatching(/34%.*Contents read/),
      expect.stringMatching(/1\.2 GB.*Index size/),
    ]);
  });

  it("shows on Home that reading what is inside files waits for the download", async () => {
    renderHome({
      dashboard: async () => ({
        ...running,
        indexed: 0,
        pending: 1_000,
        model: { state: "loading" },
        download: { done: 1_200_000_000, total: 3_000_000_000 },
      }),
    });

    const steps = await screen.findByRole("list", { name: "Getting ready" });
    const model = within(steps).getByRole("listitem", { name: "Reading model" });
    const contents = within(steps).getByRole("listitem", { name: "Contents" });
    expect(model).toHaveTextContent("Downloading: 1.2 of 3.0 GB");
    expect(within(model).getByRole("progressbar", { name: "Download" })).toHaveAttribute(
      "aria-valuenow",
      "40",
    );
    expect(contents).toHaveTextContent("Starts once the download finishes");
  });

  it("says on Home where the reading model runs once it is ready", async () => {
    let device = "cuda";
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderHome({ dashboard: async () => ({ ...running, model: { state: "ready", device } }) });

    const model = await screen.findByRole("listitem", { name: "Reading model" });
    expect(model).toHaveTextContent("Ready on your graphics card");

    device = "cpu";
    await vi.advanceTimersByTimeAsync(2_000);
    await waitFor(() => expect(model).toHaveTextContent("Ready on your processor"));
  });

  it("says on Home when the reading model could not be loaded, so only names are searched", async () => {
    renderHome({
      dashboard: async () => ({
        ...running,
        model: { state: "failed", reason: "model.onnx not found" },
      }),
    });

    const model = await screen.findByRole("listitem", { name: "Reading model" });
    expect(model).toHaveTextContent("Could not be loaded, so files are found by their names only");
    expect(model).toHaveTextContent("model.onnx not found");
    expect(screen.getByRole("listitem", { name: "Contents" })).toHaveTextContent(
      "Waits for the reading model",
    );
  });

  it("downloads the reading model again from Home when it could not be loaded", async () => {
    const props = renderHome({
      dashboard: async () => ({ ...running, model: { state: "failed", reason: "model.onnx not found" } }),
    });

    await userEvent.click(await screen.findByRole("button", { name: "Download again" }));

    expect(props.downloadModel).toHaveBeenCalled();
  });

  it("says on Home that contents are read once the reading model has loaded", async () => {
    renderHome({ dashboard: async () => ({ ...running, model: { state: "loading" } }) });

    const model = await screen.findByRole("listitem", { name: "Reading model" });
    expect(model).toHaveTextContent("Loading");
    expect(screen.getByRole("listitem", { name: "Contents" })).toHaveTextContent(
      "Starts once the reading model has loaded",
    );
  });

  it("says on Home while the files' names are still being found", async () => {
    renderHome({ dashboard: async () => ({ ...running, scanning: true }) });

    expect(await screen.findByRole("listitem", { name: "File names" })).toHaveTextContent(
      "Finding files…",
    );
  });

  it("shows on Home how far reading contents has got, and when every file is read", async () => {
    let indexed = 340;
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderHome({
      dashboard: async () => ({ ...running, indexed, pending: 1_000 - indexed }),
    });

    const contents = await screen.findByRole("listitem", { name: "Contents" });
    expect(contents).toHaveTextContent("340 of 1,000 files read");
    expect(within(contents).getByRole("progressbar", { name: "Files read" })).toHaveAttribute(
      "aria-valuenow",
      "34",
    );

    indexed = 1_000;
    await vi.advanceTimersByTimeAsync(2_000);
    await waitFor(() => expect(contents).toHaveTextContent("All 1,000 files read"));
    expect(within(contents).queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("does not say on Home that every file was read when some could not be", async () => {
    renderHome({ dashboard: async () => ({ ...running, indexed: 995, pending: 0, failed: 5 }) });

    expect(await screen.findByRole("listitem", { name: "Contents" })).toHaveTextContent(
      "995 files read, 5 could not be",
    );
  });

  it("opens search from Home, showing the keys that bring it up anywhere", async () => {
    const props = renderHome();

    const search = await screen.findByRole("button", { name: /Search your files/ });
    expect(search).toHaveAccessibleName("Search your files, or press Ctrl + Shift + Space");
    expect(within(search).getAllByText(/^(Ctrl|Shift|Space)$/)).toHaveLength(3);
    await userEvent.click(search);

    expect(props.openSearch).toHaveBeenCalledWith("");
  });

  it("starts a search from Home with a filter prefix picked from the ways to search", async () => {
    const props = renderHome();

    const ways = await screen.findByRole("list", { name: "Ways to search" });
    await userEvent.click(within(ways).getByRole("button", { name: /Changed this week/ }));

    expect(props.openSearch).toHaveBeenCalledWith("date:week ");
  });

  it("says in the sidebar what search is doing, on every page", async () => {
    let status: Dashboard = { ...running, model: { state: "loading" }, download: { done: 4, total: 10 } };
    vi.useFakeTimers({ shouldAdvanceTime: true });
    renderHome({ dashboard: async () => status });
    await userEvent.click(screen.getByRole("tab", { name: "Folders" }));

    const sidebar = await screen.findByRole("status", { name: "Search status" });
    await waitFor(() => expect(sidebar).toHaveTextContent("Downloading40%"));
    expect(sidebar).toHaveTextContent("Reading waits for it");
    expect(sidebar).not.toHaveTextContent("files to go");

    status = running;
    await vi.advanceTimersByTimeAsync(2_000);
    await waitFor(() => expect(sidebar).toHaveTextContent("Reading files34%"));

    status = { ...running, indexed: 1_000, pending: 0 };
    await vi.advanceTimersByTimeAsync(2_000);
    await waitFor(() => expect(sidebar).toHaveTextContent("Up to date"));
  });

  it("lists the Engine's parts in the sidebar, each with what it is doing", async () => {
    renderHome();

    const engine = await screen.findByRole("list", { name: "Engine" });
    await waitFor(() => expect(within(engine).getAllByRole("listitem")).toHaveLength(3));
    const [model, names, contents] = within(engine).getAllByRole("listitem");
    expect(model).toHaveTextContent("Reading model");
    expect(model).toHaveTextContent("EmbeddingGemma on the graphics card");
    expect(names).toHaveTextContent("File names");
    expect(names).toHaveTextContent("12,345 files");
    expect(contents).toHaveTextContent("Contents");
    expect(contents).toHaveTextContent("34%");
    expect(contents).toHaveTextContent("660 to go");
  });

  it("opens This computer from the sidebar: how busy the processor, memory and graphics card are, and this app's share", async () => {
    renderHome();

    await userEvent.click(await screen.findByRole("button", { name: "This computer" }));

    const panel = await screen.findByRole("dialog", { name: "This computer" });
    const cpu = await within(panel).findByRole("group", { name: "Processor" });
    expect(cpu).toHaveTextContent("12%");
    expect(cpu).toHaveTextContent("12th Gen Intel(R) Core(TM) i5-12450H");
    expect(cpu).toHaveTextContent("8 cores, 12 threads, 2.5 GHz");
    expect(cpu).toHaveTextContent("Search by Memory 4%");
    const memory = within(panel).getByRole("group", { name: "Memory" });
    expect(memory).toHaveTextContent("16.2 of 31.7 GB");
    expect(memory).toHaveTextContent("Search by Memory 0.8 GB");
    const gpu = within(panel).getByRole("group", { name: "NVIDIA GeForce RTX 4060" });
    expect(gpu).toHaveTextContent("76%");
    expect(gpu).toHaveTextContent("4.7 of 8 GB video memory");
    expect(gpu).toHaveTextContent("Search by Memory 40%, 1.2 GB");
  });

  it("shows on Home which model reads the files, where it runs, and that nothing leaves the computer", async () => {
    renderHome({
      dashboard: async () => ({ ...running, model: { state: "ready", device: "directml" } }),
    });

    const card = await screen.findByRole("region", { name: "On this computer" });
    expect(card).toHaveTextContent("EmbeddingGemma 2");
    expect(card).toHaveTextContent("Graphics card");
    expect(card).toHaveTextContent("Nothing leaves this computer");
  });

  it("lists in the sidebar the folders searched in, and goes to them on a click", async () => {
    renderHome({
      folders: async () => [
        { path: "C:\\Users\\ana", connected: true },
        { path: "E:\\Photos", connected: false },
      ],
    });

    const folders = await screen.findByRole("list", { name: "Searching in" });
    const rows = await within(folders).findAllByRole("button");
    expect(rows.map((row) => row.textContent)).toEqual(["ana", "PhotosNot connected"]);
    await userEvent.click(rows[1]);

    expect(screen.getByRole("tab", { name: "Folders" })).toHaveAttribute("aria-selected", "true");
  });

  it("shows reading as onboarding does: files read of the total and the file being read", async () => {
    await renderPage("Activity", {
      dashboard: async () => ({ ...running, current: "C:\\Users\\ana\\Documents\\Q3 Report.pdf" }),
    });

    expect(await screen.findByText("340 of 1,000 files read")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Files read" })).toBeInTheDocument();
    expect(screen.getByText("Q3 Report.pdf")).toBeInTheDocument();
  });

  it("shows the files found and how far content search reaches", async () => {
    await renderPage("Activity");

    expect(await screen.findByText("12,345 files found")).toBeInTheDocument();
    expect(
      screen.getByText("Content index 34% — files since March 2025 are ready"),
    ).toBeInTheDocument();
  });

  it("shows the content index as complete before any file is found", async () => {
    await renderPage("Activity", {
      dashboard: async () => ({ ...running, files: 0, indexed: 0, pending: 0, readySince: null }),
    });

    expect(await screen.findByText("Content index 100%")).toBeInTheDocument();
  });

  it("shows how much disk the index takes", async () => {
    await renderPage("Activity");

    expect(await screen.findByText("The index takes 1.2 GB on disk")).toBeInTheDocument();
  });

  it("keeps the status up to date while it is open", async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    let files = 10;
    await renderPage("Activity", { dashboard: async () => ({ ...running, files }) });
    expect(await screen.findByText("10 files found")).toBeInTheDocument();

    files = 25;
    await vi.advanceTimersByTimeAsync(2_000);

    expect(await screen.findByText("25 files found")).toBeInTheDocument();
  });

  it("says why indexing waits", async () => {
    await renderPage("Activity", {
      dashboard: async () => ({
        ...running,
        state: "Indexing waits while a full-screen app is open",
      }),
    });

    expect(
      await within(screen.getByRole("tabpanel")).findByText(
        "Indexing waits while a full-screen app is open",
      ),
    ).toBeInTheDocument();
  });

  it("lists the files whose content could not be indexed, and why in plain words", async () => {
    await renderPage("Activity", {
      failedFiles: async () => [
        {
          path: "C:\\Users\\ana\\Documents\\locked.pdf",
          reason: "passwordProtected",
          detail: "PDF is protected by a password",
        },
      ],
    });

    expect(await screen.findByText("1 file could not be read")).toBeInTheDocument();
    expect(screen.getByText("locked.pdf")).toBeInTheDocument();
    expect(screen.getByText("Protected by a password")).toBeInTheDocument();
  });

  it("tries the files that could not be read once more, and lists them again", async () => {
    let failed = [
      { path: "C:\\Users\\ana\\Documents\\open.docx", reason: "inUse" as const, detail: "in use" },
    ];
    const props = await renderPage("Activity", {
      failedFiles: async () => failed,
      retryFailed: vi.fn(async () => {
        failed = [];
      }),
    });

    await userEvent.click(await screen.findByRole("button", { name: "Try again" }));

    expect(props.retryFailed).toHaveBeenCalled();
    await waitFor(() => expect(screen.queryByText("open.docx")).not.toBeInTheDocument());
  });

  it("shows the underlying error of a failed file only when asked", async () => {
    await renderPage("Activity", {
      failedFiles: async () => [
        {
          path: "C:\\Users\\ana\\Documents\\report.docx",
          reason: "damaged",
          detail: "invalid Zip archive: Could not find EOCD",
        },
      ],
    });

    const details = await screen.findByRole("button", { name: "Details" });
    expect(screen.queryByText("invalid Zip archive: Could not find EOCD")).toBeNull();

    await userEvent.click(details);

    expect(screen.getByText("invalid Zip archive: Could not find EOCD")).toBeInTheDocument();
  });

  it("pauses indexing, then offers to resume it", async () => {
    let paused = false;
    const props = await renderPage("Activity", {
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

  it("deletes the index once the user confirms it", async () => {
    const props = await renderPage("Settings");

    await userEvent.click(await screen.findByRole("button", { name: "Delete index…" }));
    expect(props.purge).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Delete index and start over" }));

    expect(props.purge).toHaveBeenCalled();
  });

  it("keeps the index when the user changes their mind", async () => {
    const props = await renderPage("Settings");

    await userEvent.click(await screen.findByRole("button", { name: "Delete index…" }));
    await userEvent.click(screen.getByRole("button", { name: "Keep" }));

    expect(props.purge).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Delete index…" })).toBeInTheDocument();
  });

  it("changes the search keys in Settings, keeping the old ones when another app uses the new", async () => {
    const changeHotkey = vi.fn(async (hotkey: string) => hotkey !== "Ctrl+Alt+KeyF");
    await renderPage("Settings", { changeHotkey });

    await userEvent.click(await screen.findByRole("button", { name: "Keys for search: Ctrl + Shift + Space" }));
    await userEvent.keyboard("{Control>}{Alt>}f{/Alt}{/Control}");

    expect(changeHotkey).toHaveBeenLastCalledWith("Ctrl+Alt+KeyF");
    expect(await screen.findByText("Another app already uses Ctrl + Alt + F, so search keeps Ctrl + Shift + Space.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Keys for search: Ctrl + Shift + Space" })).toBeInTheDocument();

    await userEvent.keyboard("{Control>}{Alt>}k{/Alt}{/Control}");

    expect(await screen.findByText("Saved. Press Ctrl + Alt + K in any app to search.")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Keys for search: Ctrl + Alt + K" })).toBeInTheDocument();
  });

  it("says in Settings what reads the files and which version this is", async () => {
    await renderPage("Settings", {
      settings: async () => ({ accelerator: "cpu", hotkey: "Ctrl+Shift+Space" }),
    });

    expect(await screen.findByText("Processor")).toBeInTheDocument();
    expect(screen.getByText("Version 0.1.0")).toBeInTheDocument();
  });

  it("lists the Indexed Folders and says which drives are disconnected", async () => {
    renderHome({
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
    const props = renderHome({
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
    const props = renderHome({
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
    const props = renderHome({
      folders: async () => [{ path: "D:\\Projects", connected: true }],
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    await userEvent.click(await screen.findByRole("button", { name: "Remove D:\\Projects" }));
    await userEvent.click(screen.getByRole("button", { name: "Keep" }));

    expect(props.removeFolder).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Remove D:\\Projects" })).toBeInTheDocument();
  });

  it("lists the Exclusions with the defaults marked", async () => {
    renderHome({
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

  it("marks the system drive's default folders and offers to turn them off", async () => {
    const windows: ExclusionRow = { kind: "folder", path: "C:\\Windows", default: true };
    const props = renderHome({
      exclusions: async () => ({ rows: [windows], defaultsTurnedOff: false }),
    });
    await userEvent.click(await screen.findByRole("tab", { name: "Folders" }));

    const list = await screen.findByRole("list", { name: "Exclusions" });
    expect(within(list).getByRole("listitem")).toHaveTextContent("Default");
    await userEvent.click(screen.getByRole("button", { name: "Turn off C:\\Windows" }));

    expect(props.removeExclusion).toHaveBeenCalledWith(windows);
  });

  it("removes the user's Exclusions and turns off default ones", async () => {
    const nodeModules: ExclusionRow = { kind: "name", pattern: "node_modules", default: true };
    const backups: ExclusionRow = { kind: "name", pattern: "*.bak", default: false };
    let rows = [nodeModules, backups];
    const props = renderHome({
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
    const props = renderHome({
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
    const props = renderHome({
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
    const props = renderHome({
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
