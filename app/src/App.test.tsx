import { render, screen } from "@testing-library/react";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { afterEach, describe, expect, it } from "vitest";
import App from "./App";

afterEach(clearMocks);

describe("App", () => {
  it("starts onboarding with the speed choices this machine offers", async () => {
    mockIPC((command) => {
      if (command === "speed_choices") {
        return [{ id: "cpu", graphicsCard: null, recommended: true, extraDownload: 0 }];
      }
    });

    render(<App />);

    expect(await screen.findByRole("radio", { name: /any computer/i })).toBeChecked();
  });
});
