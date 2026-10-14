import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { SpeedStep, type SpeedChoice } from "./SpeedStep";

const nvidiaMachine: SpeedChoice[] = [
  { id: "cuda", graphicsCard: "NVIDIA GeForce RTX 4060", recommended: true, extraDownload: 1_900_000_000 },
  { id: "directml", graphicsCard: "NVIDIA GeForce RTX 4060", recommended: false, extraDownload: 0 },
  { id: "cpu", graphicsCard: null, recommended: false, extraDownload: 0 },
];

describe("SpeedStep", () => {
  it("offers the choices in plain language with the recommended one picked", () => {
    render(<SpeedStep choices={nvidiaMachine} onContinue={() => {}} />);

    expect(screen.getByRole("radio", { name: /fastest/i })).toBeChecked();
    expect(screen.getByRole("radio", { name: /any computer/i })).not.toBeChecked();
    expect(screen.getByText(/1\.9 GB/)).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/cuda|directml/i);
  });
});
