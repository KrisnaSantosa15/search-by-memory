import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LocationStep } from "./LocationStep";

const home = "C:\\Users\\ana";

describe("LocationStep", () => {
  it("searches the folder the user picks with Change…", async () => {
    const onContinue = vi.fn();
    render(
      <LocationStep
        defaultLocation={home}
        pickFolder={async () => "D:\\Archive"}
        onContinue={onContinue}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "Change…" }));
    await screen.findByText("D:\\Archive");
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));

    expect(onContinue).toHaveBeenCalledWith("D:\\Archive");
  });
});
