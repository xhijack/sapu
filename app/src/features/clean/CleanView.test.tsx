import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CleanView } from "./CleanView";
import { cleanMock } from "@/lib/cleanApi";

describe("CleanView", () => {
  it("pre-selects green rules only and the button follows the selection", async () => {
    render(<CleanView profile="general" onSeeHistory={vi.fn()} />);
    const browser = await screen.findByRole("checkbox", { name: "Cache browser" });
    expect(browser).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Log lama" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "File sementara aplikasi" })).not.toBeChecked();
    expect(screen.getByRole("checkbox", { name: "Kosongkan Trash" })).not.toBeChecked();
    // 2.3 GB + 145 MB + 117 MB ≈ 2.6 GB
    expect(screen.getByRole("button", { name: /Bersihkan 2,6 GB yang dipilih/ })).toBeInTheDocument();
    expect(screen.queryByText(/docker/i)).toBeNull();
  });

  it("shows docker commands for developers, marks unavailable ones", async () => {
    render(<CleanView profile="developer" onSeeHistory={vi.fn()} />);
    expect(await screen.findByText("docker image prune -f", { exact: false })).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "Image Docker tak terpakai" })).toBeChecked();
  });

  it("previews before cleaning and requires the permanent acknowledgement for Trash", async () => {
    const user = userEvent.setup();
    render(<CleanView profile="general" onSeeHistory={vi.fn()} />);
    await screen.findByRole("checkbox", { name: "Cache browser" });
    await user.click(screen.getByRole("checkbox", { name: "Kosongkan Trash" }));
    await user.click(screen.getByRole("button", { name: /Bersihkan/ }));

    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("Bersihkan sekarang?")).toBeInTheDocument();
    const confirm = within(dialog).getByRole("button", { name: "Bersihkan" });
    expect(confirm).toBeDisabled();
    await user.click(within(dialog).getByRole("checkbox"));
    expect(confirm).toBeEnabled();
    await user.click(confirm);

    await screen.findByRole("heading", { name: "Selesai" });
    expect(cleanMock.lastExecute).toMatchObject({ ruleIds: expect.arrayContaining(["macos-browser-caches", "macos-old-logs", "macos-trash"]), apply: true, permanentOk: true });
    expect(screen.getByText(/dibebaskan/)).toBeInTheDocument();
  });

  it("nothing is executed when the preview is cancelled", async () => {
    const user = userEvent.setup();
    render(<CleanView profile="general" onSeeHistory={vi.fn()} />);
    await screen.findByRole("checkbox", { name: "Cache browser" });
    await user.click(screen.getByRole("button", { name: /Bersihkan/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Batal" }));
    expect(cleanMock.lastExecute).toBeNull();
  });
});

describe("CleanView projects", () => {
  it("lists development projects, pre-selects green artefacts and sends them on clean", async () => {
    const user = userEvent.setup();
    render(<CleanView profile="developer" onSeeHistory={vi.fn()} />);
    expect(await screen.findByText("landing-page-2024")).toBeInTheDocument();
    expect(screen.getByText(/tidak disentuh 7 bulan/)).toBeInTheDocument();
    expect(screen.getByRole("checkbox", { name: "landing-page-2024/node_modules" })).toBeChecked();
    expect(screen.getByRole("checkbox", { name: "api/target" })).not.toBeChecked();
    expect(screen.getByText("pnpm install && pnpm run build")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Bersihkan/ }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("landing-page-2024/node_modules")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Bersihkan" }));
    await screen.findByRole("heading", { name: "Selesai" });
    expect(cleanMock.lastExecute?.artefacts).toEqual(expect.arrayContaining(["/Users/you/dev/landing-page-2024/node_modules", "/Users/you/dev/landing-page-2024/.next"]));
    expect(cleanMock.lastExecute?.ruleIds).not.toContain(expect.stringContaining("artefact:"));
    expect(screen.getByText("Proyek landing-page-2024")).toBeInTheDocument();
  });
});

describe("CleanView developer tab", () => {
  it("filters to developer sources with a per-tool strip and keeps the selection", async () => {
    const user = userEvent.setup();
    render(<CleanView profile="developer" onSeeHistory={vi.fn()} />);
    await screen.findByRole("checkbox", { name: "Cache browser" });
    await user.click(screen.getByRole("tab", { name: "Developer" }));
    expect(screen.queryByRole("checkbox", { name: "Cache browser" })).toBeNull();
    expect(screen.getByRole("checkbox", { name: "Image Docker tak terpakai" })).toBeChecked();
    expect(screen.getByText(/dari \d+ sumber/)).toBeInTheDocument();
    expect(screen.getByText("Proyek")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Bersihkan .* yang dipilih/ })).toBeInTheDocument();
    await user.click(screen.getByRole("tab", { name: "Semua" }));
    expect(await screen.findByRole("checkbox", { name: "Cache browser" })).toBeInTheDocument();
  });
});
