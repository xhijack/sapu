import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { aiMock } from "@/lib/aiApi";
import { scanMock } from "@/lib/scanApi";
import { ScanView } from "@/features/scan/ScanView";
import { CleanView } from "@/features/clean/CleanView";
import { AiSettings } from "./AiSettings";
import { WhatIsDialog } from "./WhatIsDialog";

describe("AI off", () => {
  it("shows no AI controls on results when disabled", async () => {
    const user = userEvent.setup();
    render(<ScanView />);
    await user.click(await screen.findByRole("button", { name: "Mulai pindai" }));
    await screen.findByTestId("scan-total", {}, { timeout: 3000 });
    expect(screen.queryByRole("button", { name: "Buat ringkasan dengan AI" })).toBeNull();
    await user.click(screen.getByRole("tab", { name: "Jelajah" }));
    expect(screen.queryByRole("searchbox")).toBeNull();
    expect(screen.queryByRole("button", { name: "Apa ini?" })).toBeNull();
  });
});

describe("AI on", () => {
  beforeEach(() => {
    aiMock.config = { ...aiMock.config, mode: "local" };
    scanMock.reset();
  });

  it("offers a narrative report, natural-language search and what-is on results", async () => {
    const user = userEvent.setup();
    render(<ScanView />);
    await user.click(await screen.findByRole("button", { name: "Mulai pindai" }));
    await screen.findByTestId("scan-total", {}, { timeout: 3000 });

    await user.click(await screen.findByRole("button", { name: "Buat ringkasan dengan AI" }));
    const panel = await screen.findByTestId("ai-panel");
    expect(panel).toHaveTextContent("Saran AI");
    expect(panel).toHaveTextContent(/video rekaman/);
    await user.click(within(panel).getByRole("button", { name: "Lihat data yang dikirim" }));
    expect(panel).toHaveTextContent(/Hanya ini yang dikirim ke ollama/);

    await user.click(screen.getByRole("tab", { name: "Jelajah" }));
    await user.type(await screen.findByRole("searchbox"), "video besar lama");
    await user.click(screen.getByRole("button", { name: "Cari" }));
    expect(await screen.findByText(/Dipahami sebagai: video besar yang lama/)).toBeInTheDocument();
    const searchPanel = screen.getAllByTestId("ai-panel").at(-1)!;
    expect(within(searchPanel).getByText(/zoom-rekaman-2025-03.mp4/)).toBeInTheDocument();

    const whatIs = (await screen.findAllByRole("button", { name: "Apa ini?" }))[0]!;
    await user.click(whatIs);
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("Folder cache aplikasi Spotify.")).toBeInTheDocument();
    expect(within(dialog).getByText("Mungkin bisa dihapus, periksa dulu")).toBeInTheDocument();
    expect(within(dialog).getByText(/Keyakinan 82%/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Simpan sebagai aturan" }));
    expect(await within(dialog).findByText(/Aturan disimpan/)).toBeInTheDocument();
    expect(aiMock.savedRules.length).toBe(1);
  });

  it("never renders a green label for an AI verdict", async () => {
    render(<WhatIsDialog path="/Users/you/Library/Caches/com.spotify.client" onClose={vi.fn()} />);
    const dialog = await screen.findByRole("dialog");
    await within(dialog).findByText("Folder cache aplikasi Spotify.");
    expect(within(dialog).queryByText("Aman dihapus")).toBeNull();
  });

  it("explains a project's artefacts in Bersih", async () => {
    const user = userEvent.setup();
    render(<CleanView profile="developer" onSeeHistory={vi.fn()} />);
    await screen.findByText("landing-page-2024");
    await user.click((await screen.findAllByRole("button", { name: "Jelaskan dengan AI" }))[0]!);
    expect(await screen.findByText(/Proyek Next.js yang tidak disentuh 7 bulan/)).toBeInTheDocument();
    expect(screen.getByText("pnpm run build")).toBeInTheDocument();
  });

  it("settings: switch mode, load models, save key for cloud, test connection", async () => {
    const user = userEvent.setup();
    aiMock.config = { ...aiMock.config, mode: "off" };
    render(<AiSettings />);
    await user.selectOptions(await screen.findByLabelText("Mode"), "local");
    expect(aiMock.config.mode).toBe("local");
    await user.click(screen.getByRole("button", { name: "Muat daftar model" }));
    expect(await screen.findByRole("option", { name: "qwen2.5:3b" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Uji koneksi" }));
    expect(await screen.findByText(/Terhubung, model/)).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("Mode"), "cloud");
    await user.type(screen.getByLabelText(/API key/), "sk-abc");
    await user.click(screen.getByRole("button", { name: "Simpan key" }));
    expect(aiMock.hasKey).toBe(true);
    expect(await screen.findByLabelText(/API key \(Tersimpan\)/)).toBeInTheDocument();
  });
});
