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

  it("says when another app already uses Alt + Space and asks for other keys", async () => {
    render(<HotkeyStep tryHotkey={async () => false} onContinue={() => {}} />);

    expect(await screen.findByText(/another app already uses these keys/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /continue/i })).toBeDisabled();
  });

  it("tries the keys the user presses and continues with them when they are free", async () => {
    const tryHotkey = vi.fn(async (hotkey: string) => hotkey !== "Alt+Space");
    const onContinue = vi.fn();
    render(<HotkeyStep tryHotkey={tryHotkey} onContinue={onContinue} />);
    await screen.findByText(/another app already uses these keys/i);

    await userEvent.click(screen.getByRole("button", { name: /keys for search/i }));
    await userEvent.keyboard("{Control>}{Shift>}k{/Shift}{/Control}");

    expect(await screen.findByText("Ctrl + Shift + K")).toBeInTheDocument();
    expect(screen.queryByText(/another app already uses these keys/i)).not.toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: /continue/i }));
    expect(onContinue).toHaveBeenCalledWith("Ctrl+Shift+KeyK");
  });
});
