import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Settings, type Dashboard } from "./Settings";

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
    pause: vi.fn(async () => {}),
    resume: vi.fn(async () => {}),
    ...overrides,
  };
  render(<Settings {...props} />);
  return props;
}

describe("Settings", () => {
  it("shows the files found and how far content search reaches", async () => {
    renderSettings();

    expect(await screen.findByText("12,345 files found")).toBeInTheDocument();
    expect(
      screen.getByText("Content index 34% — files since March 2025 are ready"),
    ).toBeInTheDocument();
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
