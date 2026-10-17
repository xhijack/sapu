import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { organizeMock } from "@/lib/organizeApi";
import { OrganizeView } from "./OrganizeView";

describe("OrganizeView", () => {
  it("analyses a folder, applies the ticked groups with a rename and an exclusion, and undoes", async () => {
    const user = userEvent.setup();
    render(<OrganizeView onNavigate={vi.fn()} />);

    // Idle: Documents is the default target.
    expect(await screen.findByRole("radio", { name: /Dokumen/ })).toBeChecked();
    await user.click(screen.getByRole("button", { name: "Analisis folder" }));

    // Plan: duplicate summary, groups with sure ones ticked and check ones not.
    expect(await screen.findByText(/5 file ganda/, {}, { timeout: 3000 })).toBeInTheDocument();
    const invoice = screen.getByRole("group", { name: "Invoice" });
    const reports = screen.getByRole("group", { name: "Laporan Keuangan" });
    expect(within(invoice).getByRole("checkbox", { name: /Pindahkan/ })).not.toBeChecked();
    expect(within(reports).getByRole("checkbox", { name: /Pindahkan/ })).toBeChecked();
    // Existing folders and projects keep their name.
    expect(within(screen.getByRole("group", { name: "Proyek" })).queryByRole("textbox")).toBeNull();

    // Tick Invoice and rename it; exclude one file of the report group.
    await user.click(within(invoice).getByRole("checkbox", { name: /Pindahkan/ }));
    const name = within(invoice).getByRole("textbox", { name: "Nama folder" });
    await user.clear(name);
    await user.type(name, "Tagihan");
    await user.click(within(reports).getByRole("button", { name: /Lihat isi/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("checkbox", { name: "Laporan Keuangan Q3.xlsx" }));
    await user.click(within(dialog).getByRole("button", { name: "Tutup" }));

    await user.click(screen.getByRole("button", { name: /Rapikan \d+ kelompok/ }));
    expect(await screen.findByText(/item dipindahkan ke/, {}, { timeout: 3000 })).toBeInTheDocument();

    // Before and after, side by side.
    const before = screen.getByRole("region", { name: "Sebelum" });
    const after = screen.getByRole("region", { name: "Sesudah" });
    expect(within(before).getByText("Invoice-2024-01.pdf")).toBeVisible();
    const tagihan = within(after).getByRole("group", { name: /Tagihan/ });
    expect(tagihan).toHaveTextContent("3 item");
    expect(within(tagihan).getByText("Invoice-2024-01.pdf")).toBeVisible();
    // Picking a folder on the right highlights where its files were on the left.
    const reportsDone = within(after).getByRole("group", { name: /Laporan Keuangan/ });
    await user.click(within(reportsDone).getByText(/Laporan Keuangan/));
    expect(within(before).getByText("Laporan Keuangan Q1.xlsx").closest("li")).toHaveAttribute("data-highlight", "true");
    expect(within(before).getByText("Invoice-2024-01.pdf").closest("li")).toHaveAttribute("data-highlight", "false");

    const sel = organizeMock.lastSelection!;
    expect(sel.group_ids).toContain("g3");
    expect(sel.renames).toContainEqual(["g3", "Tagihan"]);
    expect(sel.excluded).toContain("/Users/you/Documents/Laporan Keuangan Q3.xlsx");

    await user.click(screen.getByRole("button", { name: "Kembalikan semua" }));
    expect(await screen.findByText(/dikembalikan/)).toBeInTheDocument();
  });
});
