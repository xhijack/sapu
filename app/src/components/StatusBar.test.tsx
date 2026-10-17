import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { scanMock, scanApi } from "@/lib/scanApi";
import { mock } from "@/lib/api";
import { MEMORY_OPEN_EVENT } from "@/features/memory/memoryBus";
import { StatusBar } from "./StatusBar";

describe("StatusBar", () => {
  it("colours the disk dot by core's SpaceLevel and offers to free memory", async () => {
    const user = userEvent.setup();
    const snapshot = { ...mock.system };
    const low = { ...snapshot.disks[0]!, available_bytes: 18 * 1024 ** 3, level: "critical" as const };
    mock.system = { ...snapshot, disks: [low, ...snapshot.disks.slice(1)] };
    const asked = vi.fn();
    window.addEventListener(MEMORY_OPEN_EVENT, asked);
    try {
      render(<StatusBar onNavigate={() => undefined} />);
      await screen.findByText(/18,0 GB kosong/);
      const dot = document.querySelector("[data-level]");
      expect(dot?.getAttribute("data-level")).toBe("critical");
      expect(dot?.className).toContain("bg-gauge-danger");
      await user.click(screen.getByRole("button", { name: "Bebaskan memori" }));
      expect(asked).toHaveBeenCalledTimes(1);
    } finally {
      window.removeEventListener(MEMORY_OPEN_EVENT, asked);
      mock.system = snapshot;
    }
  });

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
