import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { api, mock } from "@/lib/api";
import { cleanMock } from "@/lib/cleanApi";
import { HomeView } from "./HomeView";

describe("HomeView", () => {
  it("shows every disk with used/free, activity and quick actions", async () => {
    const user = userEvent.setup();
    const nav = vi.fn();
    render(<HomeView onNavigate={nav} />);
    // Main disk is the hero ring; other disks are small rings.
    expect(await screen.findByRole("img", { name: /Macintosh HD: 88%/ })).toBeInTheDocument();
    expect(screen.getByText("Backup")).toBeInTheDocument();
    expect(screen.getByText(/dari 494,0 GB, 88% terpakai/)).toBeInTheDocument();
    expect(screen.getByText(/Trash dulu/)).toBeInTheDocument();
    expect(screen.getByText("41,2 GB")).toBeInTheDocument();
    expect(screen.getByText("Perapi 10 GB")).toBeInTheDocument();
    expect(screen.getByText("Downloads")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Pindai disk sekarang" }));
    expect(nav).toHaveBeenCalledWith("scan");
    await user.click(screen.getByRole("button", { name: "Rawat Mac saya" }));
    expect(nav).toHaveBeenCalledWith("care");
    await user.click(screen.getByRole("button", { name: /^Duplikat/ }));
    expect(nav).toHaveBeenCalledWith("dedupe");
  });
});

describe("HomeView empty trash", () => {
  it("empties the Trash after a second confirmation and shows what was freed", async () => {
    const user = userEvent.setup();
    render(<HomeView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: /Kosongkan Trash \(12,4 GB\)/ }));
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent(/1.204 item/);
    const go = within(dialog).getByRole("button", { name: "Kosongkan sekarang" });
    expect(go).toBeDisabled();
    await user.click(within(dialog).getByRole("checkbox"));
    await user.click(go);
    // Done only after checking the Trash again: it really is empty.
    const note = await within(dialog).findByRole("note", {}, { timeout: 3000 });
    expect(note).toHaveTextContent(/Trash sudah kosong/);
    expect(note).toHaveTextContent(/12,4 GB dibebaskan/);
    await user.click(within(dialog).getAllByRole("button", { name: "Tutup" }).at(-1)!);
    expect(screen.queryByRole("button", { name: /Kosongkan Trash/ })).toBeNull();
  });

  it("says plainly when the Trash is not empty after emptying, with why", async () => {
    const user = userEvent.setup();
    cleanMock.afterEmpty = { bytes: 2 * 1024 ** 3, items: 40, unreadable: [] };
    render(<HomeView onNavigate={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: /Kosongkan Trash/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("checkbox"));
    await user.click(within(dialog).getByRole("button", { name: "Kosongkan sekarang" }));
    const note = await within(dialog).findByRole("note", {}, { timeout: 3000 });
    expect(note).toHaveTextContent(/Trash belum kosong/);
    expect(note).toHaveTextContent(/40 item \(2,0 GB\) masih ada/);
    expect(within(dialog).getByText(/locked\.app/)).toBeInTheDocument();
    expect(within(dialog).getByText(/Operation not permitted/)).toBeInTheDocument();
    await user.click(within(dialog).getAllByRole("button", { name: "Tutup" }).at(-1)!);
    // Home shows what is really left, not zero.
    expect(await screen.findByRole("button", { name: /Kosongkan Trash \(2,0 GB\)/ })).toBeInTheDocument();
  });

  it("explains when Sapu may not open the Trash, instead of pretending it is empty", async () => {
    const user = userEvent.setup();
    const open = vi.spyOn(api, "openDiskAccessSettings");
    cleanMock.trash = { bytes: 0, items: 0, unreadable: ["/Users/you/.Trash"] };
    render(<HomeView onNavigate={vi.fn()} />);
    const title = await screen.findByText(/Sapu belum bisa membuka Trash/);
    const note = title.closest('[role="note"]') as HTMLElement;
    expect(await within(note).findByText(/Aktifkan Sapu di Akses Disk Penuh/)).toBeInTheDocument();
    await user.click(within(note).getByRole("button", { name: "Buka pengaturan Akses Disk Penuh" }));
    expect(open).toHaveBeenCalled();
    open.mockRestore();
  });

  it("names the app that really needs the permission when Sapu runs from another app", async () => {
    const user = userEvent.setup();
    const reveal = vi.spyOn(api, "revealAppBinary");
    mock.info = { ...mock.info, permission_owner: { name: "Visual Studio Code", path: "/Applications/Visual Studio Code.app", inherited: true } };
    cleanMock.trash = { bytes: 0, items: 0, unreadable: ["/Users/you/.Trash"] };
    render(<HomeView onNavigate={vi.fn()} />);
    const note = (await screen.findByText(/Sapu belum bisa membuka Trash/)).closest('[role="note"]') as HTMLElement;
    expect(await within(note).findByText(/dijalankan dari Visual Studio Code/)).toBeInTheDocument();
    await user.click(within(note).getByRole("button", { name: "Tunjukkan Visual Studio Code di Finder" }));
    expect(reveal).toHaveBeenCalled();
    reveal.mockRestore();
  });
});
