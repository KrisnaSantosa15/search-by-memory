import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DoneStep } from "./DoneStep";

describe("DoneStep", () => {
  it("shows the search keys and says Home opens next to follow the reading", async () => {
    const onFinish = vi.fn();
    render(<DoneStep hotkey="Ctrl+Shift+Space" onFinish={onFinish} />);

    expect(screen.getAllByText(/^(Ctrl|Shift|Space)$/)).toHaveLength(3);
    expect(screen.getByText(/Home opens next/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole("button", { name: "Start searching" }));

    expect(onFinish).toHaveBeenCalled();
  });
});
