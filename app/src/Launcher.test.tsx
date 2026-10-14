import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Launcher, type ResultRow } from "./Launcher";

const report: ResultRow = {
  name: "Q3 Report.pdf",
  path: "D:\\Work\\Q3 Report.pdf",
  folder: "D:\\Work",
  copies: [],
  connected: true,
  size: 2_100_000,
  modified: Date.UTC(2026, 9, 3),
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
    expect(search).toHaveBeenLastCalledWith("profit");
    expect(result).toHaveTextContent("Page 14");
    expect(result).toHaveTextContent("name + content");
    expect(result).toHaveTextContent("net profit increased by 18%");
    expect(result).toHaveTextContent("D:\\Work\\Q3 Report.pdf");
  });

  it("selects the first Result and moves the selection with the arrow keys", async () => {
    const notes = { ...report, name: "notes.md", path: "D:\\notes.md" };
    render(<Launcher search={async () => [report, notes]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    const first = await screen.findByRole("option", { name: /Q3 Report/ });
    const second = screen.getByRole("option", { name: /notes\.md/ });

    expect(first).toHaveAttribute("aria-selected", "true");
    await userEvent.keyboard("{ArrowDown}");
    expect(second).toHaveAttribute("aria-selected", "true");
    expect(first).toHaveAttribute("aria-selected", "false");
    await userEvent.keyboard("{ArrowDown}{ArrowUp}{ArrowUp}");
    expect(first).toHaveAttribute("aria-selected", "true");
  });

  it("opens the selected file with Enter", async () => {
    const act = actions();
    render(<Launcher search={async () => [report]} {...act} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Enter}");

    expect(act.open).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
    expect(act.reveal).not.toHaveBeenCalled();
  });

  it("reveals the selected file in its folder with Ctrl + Enter or Shift + Enter", async () => {
    const act = actions();
    render(<Launcher search={async () => [report]} {...act} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Control>}{Enter}{/Control}{Shift>}{Enter}{/Shift}");

    expect(act.reveal).toHaveBeenCalledTimes(2);
    expect(act.reveal).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
    expect(act.open).not.toHaveBeenCalled();
  });

  it("copies the selected file's path with Ctrl + C", async () => {
    const act = actions();
    render(<Launcher search={async () => [report]} {...act} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Control>}c{/Control}");

    expect(act.copyPath).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
  });

  it("leaves Ctrl + C to copy Query text that is selected", async () => {
    const act = actions();
    render(<Launcher search={async () => [report]} {...act} />);
    const box = screen.getByRole("searchbox");
    await userEvent.type(box, "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Control>}a{/Control}{Control>}c{/Control}");

    expect(act.copyPath).not.toHaveBeenCalled();
  });

  it("asks which Copy to open when a Result has several, and opens the one picked", async () => {
    const act = actions();
    const copied = { ...report, copies: ["E:\\Backup\\Q3 Report.pdf"] };
    render(<Launcher search={async () => [copied]} {...act} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Enter}");
    expect(act.open).not.toHaveBeenCalled();
    const copies = screen.getByRole("listbox", { name: /copies/i });
    expect(within(copies).getAllByRole("option").map((copy) => copy.textContent)).toEqual([
      "D:\\Work\\Q3 Report.pdf",
      "E:\\Backup\\Q3 Report.pdf",
    ]);
    await userEvent.keyboard("{ArrowDown}{Enter}");

    expect(act.open).toHaveBeenCalledWith("E:\\Backup\\Q3 Report.pdf");
  });

  it("opens the Detail Panel with Arrow Right and closes it with Arrow Left", async () => {
    const copied = { ...report, copies: ["E:\\Backup\\Q3 Report.pdf"] };
    const notes = { ...report, name: "notes.md", path: "D:\\notes.md" };
    render(<Launcher search={async () => [copied, notes]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findAllByRole("option");

    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("listbox", { name: /copies/i })).toBeInTheDocument();
    await userEvent.keyboard("{ArrowLeft}{ArrowDown}");

    expect(screen.queryByRole("listbox", { name: /copies/i })).not.toBeInTheDocument();
    expect(screen.getByRole("option", { name: /notes\.md/ })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  it("says when a Result's drive is not connected and does not try to open it", async () => {
    const act = actions();
    const away = { ...report, connected: false };
    render(<Launcher search={async () => [away]} {...act} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");

    expect(await screen.findByRole("option")).toHaveTextContent(/drive not connected/i);
    await userEvent.keyboard("{Enter}{Control>}{Enter}{/Control}");

    expect(act.open).not.toHaveBeenCalled();
    expect(act.reveal).not.toHaveBeenCalled();
  });

  it("focuses the Query box whenever the window comes back", async () => {
    render(<Launcher search={async () => []} {...actions()} />);
    const box = screen.getByRole("searchbox");
    box.blur();

    window.dispatchEvent(new FocusEvent("focus"));

    expect(box).toHaveFocus();
  });

  it("previews the selected Result: where it is, how big, when it changed, and why it matched", async () => {
    const notes = { ...report, name: "notes.md", path: "D:\\notes.md", folder: "D:\\" };
    render(<Launcher search={async () => [report, notes]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findAllByRole("option");

    const preview = screen.getByRole("region", { name: /preview/i });
    expect(within(preview).getByRole("heading")).toHaveTextContent("Q3 Report.pdf");
    expect(preview).toHaveTextContent("D:\\Work");
    expect(preview).toHaveTextContent("2.1 MB");
    expect(preview).toHaveTextContent("2026");
    expect(preview).toHaveTextContent("Name and contents match");
    expect(preview).toHaveTextContent("net profit increased by 18%");
    await userEvent.keyboard("{ArrowDown}");
    expect(within(preview).getByRole("heading")).toHaveTextContent("notes.md");
  });

  it("hides with Escape", async () => {
    const act = actions();
    render(<Launcher search={async () => []} {...act} />);

    await userEvent.type(screen.getByRole("searchbox"), "{Escape}");

    expect(act.hide).toHaveBeenCalled();
  });
});
