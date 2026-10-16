import { act, cleanup, render, screen, waitFor } from "@testing-library/react";
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

const idle = {
  files: 0,
  indexed: 0,
  pending: 0,
  readySince: null,
  state: "",
  paused: false,
  sizeOnDisk: 0,
  scanning: false,
  current: null,
  failed: 0,
};

describe("App", () => {
  it("starts onboarding with the speed choices this machine offers", async () => {
    mockIPC((command) => {
      if (command === "speed_choices") return cpuOnly;
    });

    render(<App />);

    expect(await screen.findByRole("radio", { name: /any computer/i })).toBeChecked();
  });

  it("quits from the close button during onboarding", async () => {
    const called: string[] = [];
    mockIPC((command) => {
      called.push(command);
      if (command === "speed_choices") return cpuOnly;
    });
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: /quit/i }));

    expect(called).toContain("quit");
  });

  it("skips onboarding once it has finished", async () => {
    mockIPC((command) => {
      if (command === "settings") return { accelerator: "cpu", hotkey: "Alt+Space" };
      if (command === "speed_choices") return cpuOnly;
    });

    render(<App />);

    expect(await screen.findByRole("searchbox")).toBeInTheDocument();
    expect(screen.queryByRole("radio")).not.toBeInTheDocument();
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

  it("lets the user go on to the location while the download runs", async () => {
    let cancelled = false;
    mockIPC(
      (command) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") return new Promise(() => {});
        if (command === "default_location") return "C:\\Users\\ana";
        if (command === "cancel_prepare") cancelled = true;
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));

    await screen.findByRole("progressbar");
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    expect(await screen.findByText("C:\\Users\\ana")).toBeInTheDocument();
    expect(cancelled).toBe(false);
  });

  it("shows the download on the indexing step when the user went on before it ended", async () => {
    mockIPC(
      (command) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") return new Promise(() => {});
        if (command === "default_location") return "C:\\Users\\ana";
        if (command === "dashboard") return { ...idle, files: 3, pending: 3 };
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));
    await screen.findByText("C:\\Users\\ana");
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    await act(() =>
      emit("setup-progress", { done: 500_000_000, total: 1_000_000_000, secondsLeft: null }),
    );

    expect(await screen.findByRole("progressbar", { name: "Download" })).toHaveAttribute(
      "aria-valuenow",
      "50",
    );
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

  it("goes back to the speed choices when the download is cancelled", async () => {
    let stop: (complete: boolean) => void = () => {};
    mockIPC(
      (command) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") return new Promise((finished) => (stop = finished));
        // The download notices, keeps what it has, and reports it did not finish.
        if (command === "cancel_prepare") stop(false);
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));

    await userEvent.click(await screen.findByRole("button", { name: /cancel/i }));

    expect(await screen.findByRole("radio", { name: /any computer/i })).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("asks where to search, the personal folder by default, then starts indexing it", async () => {
    const started: unknown[] = [];
    mockIPC(
      (command, payload) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") return true;
        if (command === "default_location") return "C:\\Users\\ana";
        if (command === "start_indexing") started.push(payload);
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));

    await screen.findByText("C:\\Users\\ana");
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    expect(started).toEqual([{ choice: "cpu", location: "C:\\Users\\ana" }]);
  });

  it("shows indexing after the location, then the search keys, then finishing saves what onboarding decided", async () => {
    const finished: unknown[] = [];
    mockIPC(
      (command, payload) => {
        if (command === "speed_choices") return cpuOnly;
        if (command === "prepare") return true;
        if (command === "default_location") return "C:\\Users\\ana";
        if (command === "dashboard") return { ...idle, files: 3, pending: 3 };
        if (command === "try_hotkey") return true;
        if (command === "finish_onboarding") finished.push(payload);
      },
      { shouldMockEvents: true },
    );
    render(<App />);
    await userEvent.click(await screen.findByRole("button", { name: /continue/i }));
    await screen.findByText("C:\\Users\\ana");
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    await screen.findByText("3 files found");
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    await screen.findByText("Alt + Space");
    await waitFor(() => expect(screen.getByRole("button", { name: /continue/i })).toBeEnabled());
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));
    await userEvent.click(await screen.findByRole("button", { name: /start searching/i }));

    expect(finished).toEqual([{ choice: "cpu", hotkey: "Alt+Space" }]);
  });
});
