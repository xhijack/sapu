import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Callout } from "./Callout";

describe("Callout", () => {
  it("is a note with full-strength text, with or without a bold first line", () => {
    const { rerender } = render(<Callout title="3 dari 6 kelompok dipilih">Periksa yang lain.</Callout>);
    expect(screen.getByRole("note")).toHaveTextContent("3 dari 6 kelompok dipilih");
    rerender(<Callout tone="review">Sapu menghapus file.</Callout>);
    const note = screen.getByRole("note");
    expect(note).toHaveTextContent("Sapu menghapus file.");
    expect(note.querySelector("p.font-semibold")).toBeNull();
    expect(note.innerHTML).not.toMatch(/text-fg-muted|text-fg-faint/);
  });
});
