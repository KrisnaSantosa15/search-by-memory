import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Launcher, type ResultRow } from "./Launcher";

const report: ResultRow = {
  name: "Q3 Report.pdf",
  path: "D:\\Work\\Q3 Report.pdf",
  copies: [],
  connected: true,
  snippet: "net profit increased by 18%",
  location: "Page 14",
  matchReason: "both",
};

const actions = () => ({
  open: vi.fn(),
  reveal: vi.fn(),
  copyPath: vi.fn(),
  hide: vi.fn(),
});

describe("Launcher", () => {
  it("shows File-view Results for the Query as it is typed", async () => {
    const search = vi.fn(async () => [report]);
    render(<Launcher search={search} {...actions()} />);

    await userEvent.type(screen.getByRole("searchbox"), "profit");

    const result = await screen.findByRole("option", { name: /Q3 Report\.pdf/ });
    expect(search).toHaveBeenLastCalledWith("profit", "file");
    expect(result).toHaveTextContent("Page 14");
    expect(result).toHaveTextContent("name + content");
    expect(result).toHaveTextContent("net profit increased by 18%");
    expect(result).toHaveTextContent("D:\\Work\\Q3 Report.pdf");
  });
});
