import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { organizeMock } from "@/lib/organizeApi";
import { cleanMock } from "@/lib/cleanApi";
import { OrganizeView } from "./organize/OrganizeView";
import { CareView } from "./care/CareView";
import { DedupeView } from "./dedupe/DedupeView";
import { CleanView } from "./clean/CleanView";

/** Every step after the first has a way back, and nothing in flight is lost by using it. */
describe("navigation: no dead ends", () => {
  it("Organize: back from the plan keeps it reachable", async () => {
    const user = userEvent.setup();
    render(<OrganizeView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Analisis folder" }));
    await screen.findByRole("heading", { name: "Usulan kelompok" }, { timeout: 3000 });
    await user.click(screen.getByRole("button", { name: "Kembali" }));
    expect(screen.getByRole("button", { name: "Analisis folder" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /Lihat usulan/ }));
    expect(screen.getByRole("heading", { name: "Usulan kelompok" })).toBeInTheDocument();
  });

  it("Organize: back while analysing does not yank you to the plan later", async () => {
    const user = userEvent.setup();
    organizeMock.tickMs = 60;
    render(<OrganizeView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Analisis folder" }));
    await user.click(await screen.findByRole("button", { name: "Kembali" }));
    expect(await screen.findByRole("button", { name: /Lihat usulan/ }, { timeout: 3000 })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Usulan kelompok" })).toBeNull();
  });

  it("Care: back from the report to the Care start screen", async () => {
    const user = userEvent.setup();
    render(<CareView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Rawat sekarang" }));
    await screen.findByRole("button", { name: /Bersihkan/ }, { timeout: 3000 });
    await user.click(screen.getByRole("button", { name: "Kembali" }));
    expect(await screen.findByRole("button", { name: "Rawat sekarang" })).toBeInTheDocument();
  });

  it("Duplicates: back from the groups to a new search, at the top", async () => {
    const user = userEvent.setup();
    render(<DedupeView profile="general" onSeeHistory={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Cari duplikat" }));
    const back = await screen.findByRole("button", { name: "Kembali" }, { timeout: 4000 });
    await user.click(back);
    expect(await screen.findByRole("button", { name: "Cari duplikat" })).toBeInTheDocument();
  });

  it("Clean: back while re-checking shows the previous result", async () => {
    const user = userEvent.setup();
    cleanMock.tickMs = 60;
    render(<CleanView profile="general" onSeeHistory={vi.fn()} />);
    await screen.findByRole("checkbox", { name: "Cache browser" });
    await user.click(screen.getByRole("button", { name: "Periksa ulang" }));
    await user.click(await screen.findByRole("button", { name: "Kembali" }));
    expect(screen.getByRole("checkbox", { name: "Cache browser" })).toBeInTheDocument();
  });
});
