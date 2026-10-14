import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { Settings, type Dashboard, type FailedFile } from "./Settings";

// 15 March 2025, midday UTC: the same month in every time zone.
const midMarch2025 = Date.UTC(2025, 2, 15, 12);

const running: Dashboard = {
  files: 12_345,
  indexed: 340,
  pending: 660,
  readySince: midMarch2025,
  state: "Indexing: 660 files to go",
  paused: false,
};

function renderSettings(overrides: Partial<Parameters<typeof Settings>[0]> = {}) {
  const props = {
    dashboard: async () => running,
    failedFiles: async () => [] as FailedFile[],
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
});
