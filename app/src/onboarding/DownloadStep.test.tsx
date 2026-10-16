import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DownloadStep } from "./DownloadStep";

describe("DownloadStep", () => {
  it("shows one progress bar with the amount done and the time left", () => {
    render(
      <DownloadStep
        done={1_200_000_000}
        total={3_000_000_000}
        secondsLeft={170}
        onCancel={() => {}}
        onContinue={() => {}}
      />,
    );

    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    expect(screen.getByText("1.2 of 3.0 GB")).toBeInTheDocument();
    expect(screen.getByText("About 3 minutes left")).toBeInTheDocument();
  });

  it.each([
    [null, "Working out how long this takes"],
    [40, "Less than a minute left"],
    [65, "About 1 minute left"],
    [4800, "About 1 hour 20 minutes left"],
  ])("says the time left in plain words (%s s)", (secondsLeft, words) => {
    render(
      <DownloadStep
        done={0}
        total={1}
        secondsLeft={secondsLeft}
        onCancel={() => {}}
        onContinue={() => {}}
      />,
    );

    expect(screen.getByText(words)).toBeInTheDocument();
  });
});
