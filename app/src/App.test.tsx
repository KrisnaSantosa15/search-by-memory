import { act, cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

// Unmount first and let it stop listening, which needs the mocks.
afterEach(async () => {
  cleanup();
  await new Promise((settled) => setTimeout(settled));
  clearMocks();
});

const cpuOnly = [{ id: "cpu", graphicsCard: null, recommended: true, extraDownload: 0 }];

describe("App", () => {
  it("starts onboarding with the speed choices this machine offers", async () => {
    mockIPC((command) => {
      if (command === "speed_choices") return cpuOnly;
    });

    render(<App />);

    expect(await screen.findByRole("radio", { name: /any computer/i })).toBeChecked();
  });

  it("downloads what the picked speed needs and shows its progress", async () => {
    const prepared: unknown[] = [];
    mockIPC(
      (command, payload) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") {
          prepared.push(payload);
          return new Promise(() => {});
        }
      },
      { shouldMockEvents: true },
    );
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));
    await act(() =>
      emit("setup-progress", { done: 500_000_000, total: 1_000_000_000, secondsLeft: null }),
    );

    expect(prepared).toEqual([{ choice: "cpu" }]);
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "50");
  });

  it("offers to try again when the download stops", async () => {
    let attempts = 0;
    mockIPC(
      (command) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") {
          attempts += 1;
          return attempts === 1 ? Promise.reject("connection reset") : new Promise(() => {});
        }
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));

    expect(await screen.findByText(/download stopped/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /try again/i }));

    expect(attempts).toBe(2);
    expect(screen.queryByText(/download stopped/i)).not.toBeInTheDocument();
  });

  it("asks which folders to search once the download is done, then starts indexing them", async () => {
    const started: unknown[] = [];
    mockIPC(
      (command, payload) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") return true;
        if (command === "folder_choices") {
          return [{ path: "C:\\Users\\ana", kind: "personal", picked: true }];
        }
        if (command === "start_indexing") started.push(payload);
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));

    await screen.findByRole("checkbox", { name: /your personal folders/i });
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    expect(started).toEqual([{ choice: "cpu", folders: ["C:\\Users\\ana"] }]);
  });
});
