import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Settings, type Dashboard } from "./Settings";

// 15 March 2025, midday UTC: the same month in every time zone.
const midMarch2025 = Date.UTC(2025, 2, 15, 12);

describe("Settings", () => {
  it("shows the files found and how far content search reaches", async () => {
    const dashboard: Dashboard = {
      files: 12_345,
      indexed: 340,
      pending: 660,
      readySince: midMarch2025,
    };

    render(<Settings dashboard={async () => dashboard} />);

    expect(await screen.findByText("12,345 files found")).toBeInTheDocument();
    expect(
      screen.getByText("Content index 34% — files since March 2025 are ready"),
    ).toBeInTheDocument();
  });
});
