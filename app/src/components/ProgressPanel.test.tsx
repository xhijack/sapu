import { render, screen } from "@testing-library/react";
import { ProgressPanel } from "./ProgressPanel";

describe("ProgressPanel", () => {
  it("shows percentage, counts, eta and elapsed", () => {
    render(
      <ProgressPanel
        title="Memindai"
        state={{
          fraction: 0.42,
          items_done: 420,
          items_total: 1000,
          bytes_done: 0,
          bytes_total: null,
          elapsed_secs: 65,
          eta: { kind: "about", "0": { secs: 120, nanos: 0 } },
          stage: { current: 2, total: 3, label: "membandingkan isi file" },
        }}
      />,
    );
    expect(screen.getByText("42%")).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveAttribute("aria-valuenow", "42");
    expect(screen.getByTestId("eta")).toHaveTextContent("Sekitar 2 menit lagi");
    expect(screen.getByText(/Langkah 2 dari 3/)).toBeInTheDocument();
    expect(screen.getByText(/Berjalan 1 menit/)).toBeInTheDocument();
  });

  it("indeterminate state has no percentage and no aria-valuenow", () => {
    render(
      <ProgressPanel
        title="Docker"
        state={{ fraction: null, items_done: 0, items_total: null, bytes_done: 0, bytes_total: null, elapsed_secs: 3, eta: { kind: "indeterminate" } }}
      />,
    );
    expect(screen.queryByText(/%/)).toBeNull();
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");
    expect(screen.getByTestId("eta")).toHaveTextContent("Sedang bekerja");
  });
});
