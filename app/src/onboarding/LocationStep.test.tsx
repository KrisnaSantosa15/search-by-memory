import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { LocationStep } from "./LocationStep";

const never = () => new Promise<never>(() => {});
const home = "C:\\Users\\ana";

describe("LocationStep", () => {
  it("says how many files the location holds and how large they are", async () => {
    const estimate = vi.fn(async () => ({ files: 12_400, bytes: 38_200_000_000 }));
    render(
      <LocationStep
        defaultLocation={home}
        estimate={estimate}
        pickFolder={async () => null}
        onContinue={() => {}}
      />,
    );

    expect(screen.getByText("Counting files…")).toBeInTheDocument();
    expect(await screen.findByText("About 12,400 files, 38 GB")).toBeInTheDocument();
    expect(estimate).toHaveBeenCalledWith(home);
  });

  it("searches the folder the user picks with Change…", async () => {
    const onContinue = vi.fn();
    render(
      <LocationStep
        defaultLocation={home}
        estimate={never}
        pickFolder={async () => "D:\\Archive"}
        onContinue={onContinue}
      />,
    );

    await userEvent.click(screen.getByRole("button", { name: "Change…" }));
    await screen.findByText("D:\\Archive");
    await userEvent.click(screen.getByRole("button", { name: "Continue" }));

    expect(onContinue).toHaveBeenCalledWith("D:\\Archive");
  });

  it("offers Reset to default only after a change, and it brings back the personal folder", async () => {
    render(
      <LocationStep
        defaultLocation={home}
        estimate={never}
        pickFolder={async () => "D:\\Archive"}
        onContinue={() => {}}
      />,
    );
    expect(screen.queryByRole("button", { name: "Reset to default" })).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Change…" }));
    await userEvent.click(await screen.findByRole("button", { name: "Reset to default" }));

    expect(screen.getByText(home)).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Reset to default" })).not.toBeInTheDocument();
  });
});
