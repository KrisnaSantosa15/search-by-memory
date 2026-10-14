import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { DownloadStep } from "./DownloadStep";

describe("DownloadStep", () => {
  it("shows one progress bar with the amount done and the time left", () => {
    render(<DownloadStep done={1_200_000_000} total={3_000_000_000} secondsLeft={170} />);

    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "40");
    expect(screen.getByText("1.2 of 3.0 GB")).toBeInTheDocument();
    expect(screen.getByText("About 3 minutes left")).toBeInTheDocument();
  });
});
