import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { organizeMock } from "@/lib/organizeApi";
import { scanApi } from "@/lib/scanApi";
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

  it("offers file renames and likely junk unticked, trashes only ticked junk, and sends only accepted renames", async () => {
    const user = userEvent.setup();
    const trash = vi.spyOn(scanApi, "trash");
    render(<OrganizeView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Analisis folder" }));
    await screen.findByText(/5 file ganda/, {}, { timeout: 3000 });

    // The state of the plan is said up front, in a callout, not in grey small print.
    const note = screen.getByRole("note");
    expect(note).toHaveTextContent("3 dari 6 kelompok dipilih");
    expect(note).toHaveTextContent(/3 kelompok menunggu keputusanmu/);
    expect(within(screen.getByRole("group", { name: "Arsip Lama" })).getByText(/Memindahkan folder utuh/)).toBeVisible();
    expect(within(screen.getByRole("group", { name: "Invoice" })).getByText(/Belum pasti/)).toBeVisible();
    expect(screen.getByText("3 kelompok belum dipilih")).toBeVisible();

    // Office lock files and extra exact copies sit in the junk list too, with a plain reason.
    const junkList = screen.getByRole("region", { name: "Kemungkinan sampah" });
    expect(within(junkList).getByRole("checkbox", { name: /~\$ RAMDANI BARU\.docx/ })).not.toBeChecked();
    expect(within(junkList).getByText(/Sisa file kunci Office/)).toBeVisible();
    expect(within(junkList).getByText(/Salinan persis dari KTP2\.jpeg/)).toBeVisible();
    // The copy that stays is never offered, even when the AI flags it.
    expect(within(junkList).queryByRole("checkbox", { name: /Pilih KTP2\.jpeg/ })).toBeNull();

    // A group that moves a whole folder is never ticked by default.
    expect(within(screen.getByRole("group", { name: "Arsip Lama" })).getByRole("checkbox", { name: /Pindahkan/ })).not.toBeChecked();
    expect(within(screen.getByRole("group", { name: "Proyek" })).getByRole("checkbox", { name: /Pindahkan/ })).not.toBeChecked();

    // A file left out of its group is not offered for renaming either.
    expect(within(screen.getByRole("region", { name: "Saran nama file" })).getByRole("checkbox", { name: /screenshot\.png/ })).toBeInTheDocument();
    await user.click(within(screen.getByRole("group", { name: "Gambar" })).getByRole("button", { name: /Lihat isi/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("checkbox", { name: "screenshot.png" }));
    await user.click(within(dialog).getByRole("button", { name: "Tutup" }));
    expect(within(screen.getByRole("region", { name: "Saran nama file" })).queryByRole("checkbox", { name: /screenshot\.png/ })).toBeNull();

    // Rename suggestions: unticked, editable, extension shown.
    const renames = screen.getByRole("region", { name: "Saran nama file" });
    const tick = within(renames).getByRole("checkbox", { name: /scan0001\.pdf/ });
    expect(tick).not.toBeChecked();
    const newName = within(renames).getByRole("textbox", { name: /scan0001\.pdf/ });
    expect(newName).toHaveValue("KTP Ramdani.pdf");
    await user.click(tick);
    await user.clear(newName);
    await user.type(newName, "KTP.pdf");

    // Likely junk: yellow list with the reason, nothing ticked, trash only what is ticked.
    const junk = screen.getByRole("region", { name: "Kemungkinan sampah" });
    expect(within(junk).getByText(/Salinan tanpa judul/)).toBeVisible();
    const junkTick = within(junk).getByRole("checkbox", { name: /Untitled \(copy 3\)\.docx/ });
    expect(junkTick).not.toBeChecked();
    expect(within(junk).getByRole("button", { name: /Trash/ })).toBeDisabled();
    await user.click(junkTick);
    await user.click(within(junk).getByRole("button", { name: /Trash/ }));
    expect(trash).toHaveBeenCalledWith(["/Users/you/Documents/Untitled (copy 3).docx"], true);
    expect(await within(junk).findByRole("status")).toHaveTextContent(/1 file dipindahkan ke Trash/);
    expect(within(junk).queryByRole("checkbox", { name: /Untitled/ })).toBeNull();

    // Items with no destination are listed as staying put.
    expect(screen.getByRole("region", { name: "Tetap di tempat" })).toHaveTextContent("catatan.txt");

    await user.click(screen.getByRole("button", { name: /Rapikan \d+ kelompok/ }));
    await screen.findByText(/item dipindahkan ke/, {}, { timeout: 3000 });
    const renamedHere = within(screen.getByRole("region", { name: "Sesudah" })).getByRole("group", { name: "Ganti nama di Documents" });
    expect(renamedHere).toBeInTheDocument();
    const sel = organizeMock.lastSelection!;
    expect(sel.file_renames).toEqual([["/Users/you/Documents/scan0001.pdf", "KTP.pdf"]]);
    expect(sel.group_ids).not.toContain("g6");
    trash.mockRestore();
  });

  it("ticks every waiting group with one button in the callout", async () => {
    const user = userEvent.setup();
    render(<OrganizeView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Analisis folder" }));
    const note = await screen.findByRole("note", {}, { timeout: 3000 });
    await user.click(within(note).getByRole("button", { name: "Pilih semua" }));
    expect(note).toHaveTextContent("6 dari 6 kelompok dipilih");
    expect(within(screen.getByRole("group", { name: "Arsip Lama" })).getByRole("checkbox", { name: /Pindahkan/ })).toBeChecked();
    expect(screen.queryByText(/kelompok belum dipilih/)).toBeNull();
  });
});
