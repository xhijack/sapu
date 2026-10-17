import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { scanMock, scanApi } from "@/lib/scanApi";
import { StatusBar } from "./StatusBar";

describe("StatusBar", () => {
  it("shows disk, memory and per-core cpu compactly", async () => {
    render(<StatusBar onNavigate={() => undefined} />);
    expect(await screen.findByText("61,0 GB kosong dari 494,0 GB (12%)")).toBeInTheDocument();
    // 61 GB free is above the 49.4 GB guidance, so no warning badge.
    expect(screen.queryByText(/disarankan/)).toBeNull();
    expect(screen.getByText("Memori 62%")).toBeInTheDocument();
    expect(screen.getByText("CPU 24%")).toBeInTheDocument();
    expect(screen.getByRole("img", { name: "10 inti, rata-rata 24%" })).toBeInTheDocument();
    expect(screen.getAllByTitle(/^#\d+: \d+%$/).length).toBe(10);
    expect(screen.queryByRole("progressbar")).toBeNull();
  });

  it("shows the running scan and opens its screen on click", async () => {
    const user = userEvent.setup();
    const nav = vi.fn();
    scanMock.ticks = 30;
    render(<StatusBar onNavigate={nav} />);
    await scanApi.start("/Users/you");
    const task = await screen.findByRole("button", { name: "Buka tugas yang berjalan" }, { timeout: 3000 });
    expect(task).toHaveTextContent(/Memindai/);
    await user.click(task);
    expect(nav).toHaveBeenCalledWith("scan");
    await scanApi.cancel();
  });
});
