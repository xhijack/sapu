import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
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
    expect(await within(dialog).findByText(/12,4 GB dibebaskan/, {}, { timeout: 3000 })).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Selesai" }));
    expect(screen.queryByRole("button", { name: /Kosongkan Trash/ })).toBeNull();
  });
});
