import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Wizard } from "./Wizard";

const STEPS = ["Welcome", "Speed", "Location"];

describe("Wizard", () => {
  it("marks the current step, and the ones before it as done", () => {
    render(
      <Wizard steps={STEPS} current={1}>
        <p>Pick a speed</p>
      </Wizard>,
    );

    const rail = screen.getByRole("list", { name: "Steps" });
    const [welcome, speed, location] = within(rail).getAllByRole("listitem");
    expect(welcome).toHaveTextContent("Done");
    expect(speed).toHaveAttribute("aria-current", "step");
    expect(location).not.toHaveAttribute("aria-current");
    expect(location).not.toHaveTextContent("Done");
    expect(screen.getByText("Pick a speed")).toBeInTheDocument();
  });

  it("goes back only where going back is offered", async () => {
    const onBack = vi.fn();
    const { rerender } = render(
      <Wizard steps={STEPS} current={1} onBack={onBack}>
        <p>Pick a speed</p>
      </Wizard>,
    );

    await userEvent.click(screen.getByRole("button", { name: "Back" }));
    expect(onBack).toHaveBeenCalled();

    rerender(
      <Wizard steps={STEPS} current={2}>
        <p>Pick a location</p>
      </Wizard>,
    );
    expect(screen.queryByRole("button", { name: "Back" })).not.toBeInTheDocument();
  });
});
