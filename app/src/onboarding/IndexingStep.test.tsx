import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Dashboard } from "../Settings";
import { IndexingStep } from "./IndexingStep";

const status = (overrides: Partial<Dashboard>): Dashboard => ({
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
  ...overrides,
});

/** Answers each poll with the next status, then keeps answering the last. */
function polls(...statuses: Dashboard[]) {
  let next = 0;
  return vi.fn(async () => statuses[Math.min(next++, statuses.length - 1)]);
}

describe("IndexingStep", () => {
  it("lets the user go on once the names are found", async () => {
    const onContinue = vi.fn();
    render(
      <IndexingStep
        progress={polls(status({ scanning: true }), status({ files: 12_345, pending: 12_345 }))}
        every={10}
        onContinue={onContinue}
      />,
    );

    expect(await screen.findByText("Finding files…")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Continue" })).toBeDisabled();

    expect(await screen.findByText("12,345 files found")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));
    expect(onContinue).toHaveBeenCalled();
  });

  it("shows how many files have been read, which one is being read, and how many could not be", async () => {
    render(
      <IndexingStep
        progress={polls(
          status({
            files: 1_000,
            indexed: 340,
            pending: 655,
            failed: 5,
            current: "C:\\Users\\ana\\Documents\\Q3 Report.pdf",
            state: "Indexing: 655 files to go",
          }),
        )}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText("340 of 1,000 files read")).toBeInTheDocument();
    // 340 read and 5 given up on, of 1,000.
    expect(screen.getByRole("progressbar", { name: "Files read" })).toHaveAttribute(
      "aria-valuenow",
      "35",
    );
    expect(screen.getByText("Q3 Report.pdf")).toBeInTheDocument();
    expect(screen.getByText("5 could not be read")).toBeInTheDocument();
    expect(screen.queryByText("Indexing: 655 files to go")).not.toBeInTheDocument();
  });

  it("fills the bar once every file was read or given up on", async () => {
    render(
      <IndexingStep
        progress={polls(status({ files: 17, indexed: 5, failed: 1 }))}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText("5 of 6 files read")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Files read" })).toHaveAttribute(
      "aria-valuenow",
      "100",
    );
  });

  it("says why reading waits while no file is being read", async () => {
    render(
      <IndexingStep
        progress={polls(
          status({
            files: 1_000,
            pending: 1_000,
            state: "Indexing waits until the computer is plugged in",
          }),
        )}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(
      await screen.findByText("Indexing waits until the computer is plugged in"),
    ).toBeInTheDocument();
  });

  it("shows the download that reading what is inside the files waits for", async () => {
    render(
      <IndexingStep
        progress={polls(status({ files: 1_000, pending: 1_000 }))}
        download={{ done: 1_200_000_000, total: 3_000_000_000, secondsLeft: 170 }}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText("Downloading what search needs to read your files")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Download" })).toHaveAttribute(
      "aria-valuenow",
      "40",
    );
    expect(screen.getByText("1.2 of 3.0 GB")).toBeInTheDocument();
  });

  it("offers to try the download again when it stopped", async () => {
    const onRetryDownload = vi.fn();
    render(
      <IndexingStep
        progress={polls(status({ files: 1_000, pending: 1_000 }))}
        onRetryDownload={onRetryDownload}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText(/download stopped/i)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Try again" }));
    expect(onRetryDownload).toHaveBeenCalled();
  });

  it("measures the speed from when reading began, not while the model loaded", async () => {
    const looks = [
      { at: 0, status: status({ files: 1_000, pending: 1_000 }) },
      { at: 60_000, status: status({ files: 1_000, pending: 1_000, current: "C:\\a.txt" }) },
      { at: 120_000, status: status({ files: 1_000, indexed: 60, pending: 940, current: "C:\\b.txt" }) },
    ];
    let look = 0;
    render(
      <IndexingStep
        progress={async () => looks[Math.min(look, 2)].status}
        now={() => looks[Math.min(look++, 2)].at}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText("60 files a minute")).toBeInTheDocument();
  });

  it("says how fast files are read and how long the rest takes", async () => {
    // A minute passes between the first look and every later one.
    const looks = [
      { at: 0, status: status({ files: 1_000, indexed: 100, pending: 900 }) },
      { at: 60_000, status: status({ files: 1_000, indexed: 160, pending: 840 }) },
    ];
    let look = 0;
    render(
      <IndexingStep
        progress={async () => looks[Math.min(look, 1)].status}
        now={() => looks[Math.min(look++, 1)].at}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText("60 files a minute")).toBeInTheDocument();
    expect(screen.getByText("About 14 minutes left")).toBeInTheDocument();
  });

});
