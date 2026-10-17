import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { memoryMock } from "@/lib/memoryApi";
import { MemoryDialog } from "./MemoryDialog";
import { MemoryHost } from "./MemoryHost";
import { requestMemoryDialog } from "./memoryBus";

describe("MemoryDialog", () => {
  it("shows pressure honestly and lets only user apps be ticked", async () => {
    render(<MemoryDialog os="macos" onClose={() => undefined} />);
    expect(await screen.findByText("85%")).toBeInTheDocument();
    expect(screen.getByText("Memori mulai penuh")).toBeInTheDocument();
    expect(screen.getByText(/13,6 GB terpakai dari 16,0 GB \(85%\)/)).toBeInTheDocument();
    expect(screen.getByText(/Swap terpakai 1,1 GB/)).toBeInTheDocument();
    // Largest first, helpers folded into their app.
    const boxes = screen.getAllByRole("checkbox");
    expect(boxes.map((b) => b.getAttribute("aria-label"))).toEqual(["Google Chrome", "kernel_task", "Slack", "WindowServer", "node", "Sapu"]);
    expect(screen.getByText("23 proses")).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "kernel_task" })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: "Sapu" })).toBeDisabled();
    expect(screen.getByRole("checkbox", { name: "Google Chrome" })).toBeEnabled();
    // Nothing ticked: the main action waits.
    expect(screen.getByRole("button", { name: "Tutup aplikasi terpilih (0)" })).toBeDisabled();
    // The two honest notes are on screen.
    expect(screen.getByText(/sengaja memakai memori yang menganggur sebagai cache/)).toBeInTheDocument();
    expect(screen.getByText(/menggelembungkan/)).toBeInTheDocument();
  });

  it("asks before quitting, warns about unsaved work, then reports per app", async () => {
    const user = userEvent.setup();
    render(<MemoryDialog os="linux" onClose={() => undefined} />);
    await user.click(await screen.findByRole("checkbox", { name: "Google Chrome" }));
    await user.click(screen.getByRole("checkbox", { name: "Slack" }));
    await user.click(screen.getByRole("button", { name: "Tutup aplikasi terpilih (2)" }));
    expect(screen.getByText("Tutup 2 aplikasi?")).toBeInTheDocument();
    expect(screen.getByText(/Pekerjaan yang belum disimpan bisa hilang/)).toBeInTheDocument();
    expect(memoryMock.quitCalls).toEqual([]);
    await user.click(screen.getByRole("button", { name: "Ya, tutup" }));
    expect(memoryMock.quitCalls).toEqual([[300, 400]]);
    expect(await screen.findByText("Google Chrome: diminta menutup")).toBeInTheDocument();
    expect(screen.getByText("Slack: diminta menutup")).toBeInTheDocument();
    // The refreshed list no longer has them and the figure dropped.
    await waitFor(() => expect(screen.queryByRole("checkbox", { name: "Google Chrome" })).toBeNull());
    expect(screen.getByText("51%")).toBeInTheDocument();
    // Linux: no purge section.
    expect(screen.queryByText(/purge/)).toBeNull();
  });

  it("offers purge on macOS with its real, modest effect", async () => {
    const user = userEvent.setup();
    render(<MemoryDialog os="macos" onClose={() => undefined} />);
    expect(await screen.findByText("Kosongkan cache disk (opsional)")).toBeInTheDocument();
    expect(screen.getByText(/macOS akan mengisinya lagi/)).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Kosongkan cache disk (purge)…" }));
    expect(memoryMock.purgeCalls).toBe(1);
    expect(await screen.findByText("Cache dikosongkan: memori terpakai 13,6 GB → 12,4 GB.")).toBeInTheDocument();
  });

  it("says so when the administrator prompt is dismissed", async () => {
    const user = userEvent.setup();
    memoryMock.purgeCancelled = true;
    render(<MemoryDialog os="macos" onClose={() => undefined} />);
    await user.click(await screen.findByRole("button", { name: "Kosongkan cache disk (purge)…" }));
    expect(await screen.findByText("Dibatalkan; tidak ada yang diubah.")).toBeInTheDocument();
  });

  it("closes through the dialog and tells the parent", async () => {
    const user = userEvent.setup();
    const onClose = vi.fn();
    render(<MemoryDialog os="macos" onClose={onClose} />);
    await user.click(await screen.findByRole("button", { name: "Tutup" }));
    expect(onClose).toHaveBeenCalled();
  });
});

describe("MemoryHost", () => {
  it("opens the dialog when any surface asks for it", async () => {
    render(<MemoryHost />);
    expect(screen.queryByRole("dialog")).toBeNull();
    act(() => requestMemoryDialog());
    expect(await screen.findByRole("dialog")).toBeInTheDocument();
    expect(await screen.findByText("85%")).toBeInTheDocument();
  });
});
