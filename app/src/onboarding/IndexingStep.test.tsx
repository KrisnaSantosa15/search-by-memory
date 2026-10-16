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
            state: "Indexing waits until the computer is plugged in",
          }),
        )}
        every={10}
        onContinue={() => {}}
      />,
    );

    expect(await screen.findByText("340 of 1,000 files read")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "Files read" })).toHaveAttribute(
      "aria-valuenow",
      "34",
    );
    expect(screen.getByText("Q3 Report.pdf")).toBeInTheDocument();
    expect(screen.getByText("5 could not be read")).toBeInTheDocument();
    expect(screen.getByText("Indexing waits until the computer is plugged in")).toBeInTheDocument();
  });
});
