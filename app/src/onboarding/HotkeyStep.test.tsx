import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { HotkeyStep } from "./HotkeyStep";

describe("HotkeyStep", () => {
  it("offers Alt + Space when it is free", async () => {
    const tryHotkey = vi.fn(async () => true);
    const onContinue = vi.fn();
    render(<HotkeyStep tryHotkey={tryHotkey} onContinue={onContinue} />);

    expect(await screen.findByText("Alt + Space")).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));

    expect(tryHotkey).toHaveBeenCalledWith("Alt+Space");
    expect(onContinue).toHaveBeenCalledWith("Alt+Space");
  });
});
