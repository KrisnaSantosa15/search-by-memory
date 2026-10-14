import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FoldersStep, type FolderChoice } from "./FoldersStep";

const choices: FolderChoice[] = [
  { path: "C:\Users\ana", kind: "personal", picked: true },
  { path: "D:\\", kind: "drive", picked: false },
];

describe("FoldersStep", () => {
  it("starts with the personal folders picked and continues with what the user picked", async () => {
    const onContinue = vi.fn();
    render(<FoldersStep choices={choices} onContinue={onContinue} />);

    expect(screen.getByRole("checkbox", { name: /your personal folders/i })).toBeChecked();
    await userEvent.click(screen.getByRole("checkbox", { name: /drive D:/i }));
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    expect(onContinue).toHaveBeenCalledWith(["C:\Users\ana", "D:\\"]);
  });
});
