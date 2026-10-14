import { act, render, screen, waitFor, within } from "@testing-library/react";
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
  kind: "document",
  weaker: false,
  snippet: "net profit increased by 18%",
  location: "Page 14",
  page: 14,
  lineStart: null,
  lineEnd: null,
  matchReason: "both",
};

const actions = () => ({
  open: vi.fn(),
  reveal: vi.fn(),
  copyFile: vi.fn(),
  hide: vi.fn(),
  previewLines: vi.fn(async () => null),
  thumbnail: vi.fn(async () => null),
  pdfPage: vi.fn(async () => null),
});

describe("Launcher", () => {
  it("shows File-view Results for the Query as it is typed", async () => {
    const search = vi.fn(async () => [report]);
    render(<Launcher search={search} {...actions()} />);

    await userEvent.type(screen.getByRole("searchbox"), "profit");

    const result = await screen.findByRole("option", { name: /Q3 Report\.pdf/ });
    expect(search).toHaveBeenLastCalledWith("profit", false);
    expect(result).toHaveTextContent("Page 14");
    expect(result).toHaveTextContent("net profit increased by 18%");
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

  it("selects a Result that is clicked, keeping the keyboard in the Query box", async () => {
    const notes = { ...report, name: "notes.md", path: "D:\\notes.md" };
    render(<Launcher search={async () => [report, notes]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");

    await userEvent.click(await screen.findByRole("option", { name: /notes\.md/ }));

    expect(screen.getByRole("option", { name: /notes\.md/ })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("searchbox")).toHaveFocus();
  });

  it("groups Results by kind, each group in the order of its best Result", async () => {
    const retry = { ...report, name: "retry.ts", path: "D:\\code\\retry.ts", kind: "code" as const };
    const lease = { ...report, name: "lease.pdf", path: "D:\\lease.pdf" };
    render(<Launcher search={async () => [report, retry, lease]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findAllByRole("option");

    const groups = screen.getAllByRole("group");
    expect(groups.map((group) => group.getAttribute("aria-label"))).toEqual(["Documents", "Code"]);
    expect(within(groups[0]).getAllByRole("option").map((option) => option.textContent)).toEqual([
      expect.stringContaining("Q3 Report.pdf"),
      expect.stringContaining("lease.pdf"),
    ]);
    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByRole("option", { name: /lease\.pdf/ })).toHaveAttribute("aria-selected", "true");
  });

  it("folds Weaker Matches into one row that Enter unfolds", async () => {
    const faint = { ...report, name: "faint.txt", path: "D:\\faint.txt", weaker: true };
    const fainter = { ...report, name: "fainter.txt", path: "D:\\fainter.txt", weaker: true };
    render(<Launcher search={async () => [report, faint, fainter]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");

    const fold = await screen.findByRole("option", { name: "2 weaker matches" });
    expect(screen.queryByRole("option", { name: /faint\.txt/ })).not.toBeInTheDocument();
    await userEvent.keyboard("{ArrowDown}");
    expect(fold).toHaveAttribute("aria-selected", "true");
    await userEvent.keyboard("{Enter}{ArrowDown}");

    expect(screen.getByRole("option", { name: /faint\.txt/ })).toHaveAttribute("aria-selected", "true");
    expect(within(screen.getByRole("region", { name: /preview/i })).getByRole("heading")).toHaveTextContent(
      "faint.txt",
    );
  });

  it("explains the weaker matches row while it is selected", async () => {
    const faint = { ...report, name: "faint.txt", path: "D:\\faint.txt", weaker: true };
    render(<Launcher search={async () => [report, faint]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option", { name: "1 weaker match" });

    await userEvent.keyboard("{ArrowDown}");
    expect(screen.getByText(/match less closely.*Enter to show/i)).toBeInTheDocument();
    await userEvent.keyboard("{Enter}");
    expect(screen.getByText(/match less closely.*Enter to hide/i)).toBeInTheDocument();
  });

  it("counts Weaker Matches apart in the footer", async () => {
    const faint = { ...report, name: "faint.txt", path: "D:\\faint.txt", weaker: true };
    const fainter = { ...report, name: "fainter.txt", path: "D:\\fainter.txt", weaker: true };
    render(<Launcher search={async () => [report, faint, fainter]} {...actions()} />);

    await userEvent.type(screen.getByRole("searchbox"), "profit");

    expect(await screen.findByRole("status")).toHaveTextContent("1 result, 2 weaker");
  });

  it("unfolds Weaker Matches with Arrow Right or a click", async () => {
    const faint = { ...report, name: "faint.txt", path: "D:\\faint.txt", weaker: true };
    render(<Launcher search={async () => [report, faint]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    const fold = await screen.findByRole("option", { name: "1 weaker match" });

    await userEvent.keyboard("{ArrowDown}{ArrowRight}");
    expect(screen.getByRole("option", { name: /faint\.txt/ })).toBeInTheDocument();
    await userEvent.click(fold);
    expect(screen.queryByRole("option", { name: /faint\.txt/ })).not.toBeInTheDocument();
  });

  it("opens the selected file with Enter", async () => {
    const spies = actions();
    render(<Launcher search={async () => [report]} {...spies} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Enter}");

    expect(spies.open).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
    expect(spies.reveal).not.toHaveBeenCalled();
  });

  it("reveals the selected file in its folder with Ctrl + Enter or Shift + Enter", async () => {
    const spies = actions();
    render(<Launcher search={async () => [report]} {...spies} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Control>}{Enter}{/Control}{Shift>}{Enter}{/Shift}");

    expect(spies.reveal).toHaveBeenCalledTimes(2);
    expect(spies.reveal).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
    expect(spies.open).not.toHaveBeenCalled();
  });

  it("copies the selected file with Ctrl + C", async () => {
    const spies = actions();
    render(<Launcher search={async () => [report]} {...spies} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Control>}c{/Control}");

    expect(spies.copyFile).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
  });

  it("leaves Ctrl + C to copy Query text that is selected", async () => {
    const spies = actions();
    render(<Launcher search={async () => [report]} {...spies} />);
    const box = screen.getByRole("searchbox");
    await userEvent.type(box, "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Control>}a{/Control}{Control>}c{/Control}");

    expect(spies.copyFile).not.toHaveBeenCalled();
  });

  it("asks which Copy to open when a Result has several, and opens the one picked", async () => {
    const spies = actions();
    const copied = { ...report, copies: ["E:\\Backup\\Q3 Report.pdf"] };
    render(<Launcher search={async () => [copied]} {...spies} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.keyboard("{Enter}");
    expect(spies.open).not.toHaveBeenCalled();
    const copies = screen.getByRole("listbox", { name: /copies/i });
    expect(within(copies).getAllByRole("option").map((copy) => copy.textContent)).toEqual([
      "D:\\Work\\Q3 Report.pdf",
      "E:\\Backup\\Q3 Report.pdf",
    ]);
    await userEvent.keyboard("{ArrowDown}{Enter}");

    expect(spies.open).toHaveBeenCalledWith("E:\\Backup\\Q3 Report.pdf");
  });

  it("starts asking which Copy on the one picked last time, even after a restart", async () => {
    const saved = new Map<string, string>();
    const memory = {
      getItem: (key: string) => saved.get(key) ?? null,
      setItem: (key: string, value: string) => void saved.set(key, value),
    };
    const copied = { ...report, copies: ["E:\\Backup\\Q3 Report.pdf"] };
    const first = render(<Launcher search={async () => [copied]} {...actions()} memory={memory} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");
    await userEvent.keyboard("{Enter}{ArrowDown}{Enter}");
    first.unmount();

    const spies = actions();
    render(<Launcher search={async () => [copied]} {...spies} memory={memory} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");
    await userEvent.keyboard("{Enter}");

    expect(spies.open).not.toHaveBeenCalled();
    const copies = screen.getByRole("listbox", { name: /copies/i });
    expect(within(copies).getByRole("option", { selected: true })).toHaveTextContent(
      "E:\\Backup\\Q3 Report.pdf",
    );
    await userEvent.keyboard("{Enter}");
    expect(spies.open).toHaveBeenCalledWith("E:\\Backup\\Q3 Report.pdf");
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
    const spies = actions();
    const away = { ...report, connected: false };
    render(<Launcher search={async () => [away]} {...spies} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");

    expect(await screen.findByRole("option")).toHaveTextContent(/drive not connected/i);
    await userEvent.keyboard("{Enter}{Control>}{Enter}{/Control}");

    expect(spies.open).not.toHaveBeenCalled();
    expect(spies.reveal).not.toHaveBeenCalled();
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

  it("marks the Query's words where they appear in the snippet", async () => {
    render(<Launcher search={async () => [report]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "Profit in");
    await screen.findByRole("option");

    const preview = screen.getByRole("region", { name: /preview/i });
    const marks = preview.querySelectorAll("mark");
    expect([...marks].map((mark) => mark.textContent)).toEqual(["profit", "in"]);
  });

  it("opens or shows the previewed file from its buttons", async () => {
    const spies = actions();
    render(<Launcher search={async () => [report]} {...spies} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");

    await userEvent.click(screen.getByRole("button", { name: "Open" }));
    await userEvent.click(screen.getByRole("button", { name: "Show in folder" }));

    expect(spies.open).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
    expect(spies.reveal).toHaveBeenCalledWith("D:\\Work\\Q3 Report.pdf");
  });

  it("lists the Copies in the Preview Pane and picks one there", async () => {
    const copied = { ...report, copies: ["E:\\Backup\\Q3 Report.pdf"] };
    render(<Launcher search={async () => [copied]} {...actions()} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findByRole("option");
    const preview = screen.getByRole("region", { name: /preview/i });

    expect(preview).toHaveTextContent("E:\\Backup\\Q3 Report.pdf");
    await userEvent.keyboard("{Enter}");

    expect(within(preview).getByRole("listbox", { name: /copies/i })).toBeInTheDocument();
    expect(preview).toHaveTextContent(/which copy/i);
  });

  it("says in the footer what the search is doing, with the keys to act", async () => {
    let answer: (results: ResultRow[]) => void = () => {};
    const search = () => new Promise<ResultRow[]>((resolve) => (answer = resolve));
    render(<Launcher search={search} {...actions()} />);
    const status = screen.getByRole("status");

    await userEvent.type(screen.getByRole("searchbox"), "profit");
    expect(status).toHaveTextContent("Searching…");
    await act(async () => answer([report, { ...report, path: "D:\\other.pdf" }]));

    expect(status).toHaveTextContent("2 results");
    expect(screen.getByRole("list", { name: "Keys" })).toHaveTextContent(/Enter.*Open/);
    expect(screen.getByRole("list", { name: "Keys" })).toHaveTextContent(/Ctrl\+Enter.*Show in folder/);
    expect(screen.getByRole("list", { name: "Keys" })).toHaveTextContent(/Ctrl\+C.*Copy/);
  });

  it("says when nothing matches", async () => {
    render(<Launcher search={async () => []} {...actions()} />);

    await userEvent.type(screen.getByRole("searchbox"), "zebra");

    expect(await screen.findByRole("status")).toHaveTextContent("Nothing matches yet");
  });

  it("previews code with line numbers, marking the matched lines and the Query's words", async () => {
    const retry = {
      ...report,
      name: "retry.ts",
      path: "D:\\code\\retry.ts",
      kind: "code" as const,
      snippet: "export async function retry",
      location: "Lines 8–9",
      lineStart: 8,
      lineEnd: 9,
    };
    const previewLines = vi.fn(async () => ({
      firstLine: 7,
      lines: [" */", "export async function retry<T>(", "  fn: () => Promise<T>,", "): Promise<T> {"],
    }));
    render(<Launcher search={async () => [retry]} {...actions()} previewLines={previewLines} />);
    await userEvent.type(screen.getByRole("searchbox"), "retry");

    const code = await screen.findByRole("list", { name: "Lines" });
    expect(previewLines).toHaveBeenCalledWith("D:\\code\\retry.ts", 8, 9);
    const lines = within(code).getAllByRole("listitem");
    expect(lines.map((line) => line.getAttribute("value"))).toEqual(["7", "8", "9", "10"]);
    expect(lines.map((line) => line.getAttribute("aria-current"))).toEqual([null, "true", "true", null]);
    expect(within(lines[1]).getByText("retry", { selector: "mark" })).toBeInTheDocument();
  });

  it("shows a text document's lines around the match on the page", async () => {
    const notes = {
      ...report,
      name: "notes.md",
      path: "D:\\notes.md",
      snippet: "net profit rose",
      location: "Line 2",
      lineStart: 2,
      lineEnd: 2,
    };
    const previewLines = vi.fn(async () => ({
      firstLine: 1,
      lines: ["# Q3", "net profit rose", "costs stayed flat"],
    }));
    render(<Launcher search={async () => [notes]} {...actions()} previewLines={previewLines} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");

    const page = await screen.findByRole("list", { name: "Lines" });
    const lines = within(page).getAllByRole("listitem");
    expect(lines.map((line) => line.textContent)).toEqual(["# Q3", "net profit rose", "costs stayed flat"]);
    expect(lines.map((line) => line.getAttribute("aria-current"))).toEqual([null, "true", null]);
    expect(screen.getByRole("region", { name: /preview/i })).toHaveTextContent("Line 2");
  });

  it("keeps the snippet in view while lines load, or when they cannot be read", async () => {
    const retry = { ...report, name: "retry.ts", kind: "code" as const, lineStart: 8, lineEnd: 9 };
    const notes = { ...retry, name: "gone.ts", path: "D:\\gone.ts" };
    const previewLines = vi.fn((path: string) =>
      path === notes.path ? Promise.resolve(null) : new Promise<null>(() => {}),
    );
    render(<Launcher search={async () => [retry, notes]} {...actions()} previewLines={previewLines} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");
    await screen.findAllByRole("option");
    const preview = screen.getByRole("region", { name: /preview/i });

    expect(preview).toHaveTextContent("net profit increased by 18%");
    await userEvent.keyboard("{ArrowDown}");
    expect(within(preview).getByRole("heading")).toHaveTextContent("gone.ts");
    expect(preview).toHaveTextContent("net profit increased by 18%");
  });

  it("searches only code when the Query starts with code:", async () => {
    const search = vi.fn(async () => [report]);
    render(<Launcher search={search} {...actions()} />);

    await userEvent.type(screen.getByRole("searchbox"), "Code: retry with backoff");

    await screen.findByRole("option");
    expect(search).toHaveBeenLastCalledWith("retry with backoff", true);
    expect(screen.getByRole("img", { name: /searching code/i })).toBeInTheDocument();
  });

  it("shows a file's thumbnail in its row, or an icon for its kind without one", async () => {
    const photo = { ...report, name: "holiday.jpg", path: "D:\\holiday.jpg", kind: "image" as const };
    const thumbnail = vi.fn(async (path: string) =>
      path === photo.path ? "data:image/png;base64,AAAA" : null,
    );
    render(<Launcher search={async () => [report, photo]} {...actions()} thumbnail={thumbnail} />);
    await userEvent.type(screen.getByRole("searchbox"), "profit");

    const photoRow = await screen.findByRole("option", { name: /holiday\.jpg/ });
    await waitFor(() =>
      expect(photoRow.querySelector("img")).toHaveAttribute("src", "data:image/png;base64,AAAA"),
    );
    const reportRow = screen.getByRole("option", { name: /Q3 Report\.pdf/ });
    expect(within(reportRow).getByRole("img", { name: "Document" })).toBeInTheDocument();
  });

  it("shows a picture or video as a large picture in the Preview Pane", async () => {
    const photo = { ...report, name: "holiday.jpg", path: "D:\\holiday.jpg", kind: "image" as const };
    const thumbnail = vi.fn(async (_path: string, size: number) =>
      size > 80 ? "data:image/png;base64,LARGE" : "data:image/png;base64,SMALL",
    );
    render(<Launcher search={async () => [photo]} {...actions()} thumbnail={thumbnail} />);
    await userEvent.type(screen.getByRole("searchbox"), "beach");

    const picture = await screen.findByRole("img", { name: "holiday.jpg" });
    expect(picture).toHaveAttribute("src", "data:image/png;base64,LARGE");
    expect(thumbnail).toHaveBeenCalledWith("D:\\holiday.jpg", 840);
  });

  it("shows the matched PDF page with the Query's words marked on it", async () => {
    const pdfPage = vi.fn(async () => ({
      image: "data:image/png;base64,PAGE",
      marks: [
        { left: 0.1, top: 0.2, width: 0.05, height: 0.02 },
        { left: 0.3, top: 0.6, width: 0.05, height: 0.02 },
      ],
    }));
    render(<Launcher search={async () => [report]} {...actions()} pdfPage={pdfPage} />);
    await userEvent.type(screen.getByRole("searchbox"), "net profit");

    const page = await screen.findByRole("img", { name: "Page 14 of Q3 Report.pdf" });
    expect(page).toHaveAttribute("src", "data:image/png;base64,PAGE");
    expect(pdfPage).toHaveBeenLastCalledWith("D:\\Work\\Q3 Report.pdf", 14, "net profit");
    const preview = screen.getByRole("region", { name: /preview/i });
    expect(preview.querySelectorAll("figure mark")).toHaveLength(2);
  });

  it("scrolls the PDF page to the first of the Query's words on it", async () => {
    const pdfPage = vi.fn(async () => ({
      image: "data:image/png;base64,PAGE",
      marks: [
        { left: 0.3, top: 0.6, width: 0.05, height: 0.02 },
        { left: 0.1, top: 0.2, width: 0.05, height: 0.02 },
      ],
    }));
    const scrolled: Element[] = [];
    // jsdom has no scrolling; record which element asked to be scrolled into view.
    Element.prototype.scrollIntoView = function (this: Element) {
      scrolled.push(this);
    };
    render(<Launcher search={async () => [report]} {...actions()} pdfPage={pdfPage} />);
    await userEvent.type(screen.getByRole("searchbox"), "net profit");

    await screen.findByRole("img", { name: "Page 14 of Q3 Report.pdf" });
    await waitFor(() => expect(scrolled.at(-1)).toHaveStyle({ top: "20%" }));
    expect(scrolled.at(-1)?.tagName).toBe("MARK");
    // @ts-expect-error: back to jsdom's own, which has none.
    delete Element.prototype.scrollIntoView;
  });

  it("hides with Escape", async () => {
    const spies = actions();
    render(<Launcher search={async () => []} {...spies} />);

    await userEvent.type(screen.getByRole("searchbox"), "{Escape}");

    expect(spies.hide).toHaveBeenCalled();
  });
});
