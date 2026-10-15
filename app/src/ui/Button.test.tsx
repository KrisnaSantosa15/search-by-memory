import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "./Button";

describe("Button", () => {
  it("does its action without submitting the form it sits in", async () => {
    const submit = vi.fn((event: SubmitEvent) => event.preventDefault());
    const action = vi.fn();
    render(
      <form onSubmit={(event) => submit(event.nativeEvent as SubmitEvent)}>
        <Button onClick={action}>Change…</Button>
      </form>,
    );

    await userEvent.click(screen.getByRole("button", { name: "Change…" }));

    expect(action).toHaveBeenCalledOnce();
    expect(submit).not.toHaveBeenCalled();
  });
});
