import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Shell } from "./Shell";

describe("Shell", () => {
  it("gives the window a drag handle that Tauri understands and that does not scroll away", () => {
    render(
      <Shell screen="home" onNavigate={vi.fn()}>
        <p>isi</p>
      </Shell>,
    );
    const handles = document.querySelectorAll("[data-tauri-drag-region]");
    expect(handles.length).toBeGreaterThanOrEqual(2);
    // The strip above the content stays put while the page scrolls.
    const strip = screen.getByTestId("titlebar");
    expect(strip).toHaveAttribute("data-tauri-drag-region");
    expect(strip.className).toMatch(/sticky/);
  });
});
