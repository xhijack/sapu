import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AppsView } from "./AppsView";
import { appsMock } from "@/lib/appsApi";

describe("AppsView", () => {
  it("lists apps largest first, shows leftovers, refuses system apps, uninstalls with leftovers", async () => {
    const user = userEvent.setup();
    render(<AppsView onSeeHistory={vi.fn()} />);
    const items = await screen.findAllByRole("button", { name: /Xcode|Docker|Slack|Zoom/ });
    expect(items[0]).toHaveTextContent("Xcode");

    await user.click(screen.getByRole("button", { name: /Xcode/ }));
    expect((await screen.findByText("Aplikasi bawaan macOS tidak bisa dihapus oleh Sapu.")).closest('[role="note"]')).not.toBeNull();

    await user.click(screen.getByRole("button", { name: /Zoom/ }));
    expect((await screen.findAllByText("zoom.us")).length).toBeGreaterThan(0);
    const btn = await screen.findByRole("button", { name: /Hapus aplikasi dan 4 sisa/ });
    await user.click(btn);
    const dialog = await screen.findByRole("dialog");
    await within(dialog).findByText(/5 item/);
    await user.click(within(dialog).getByRole("button", { name: "Pindahkan ke Trash" }));
    await screen.findByText(/5 item dipindahkan ke Trash/);
    expect(appsMock.lastUninstall?.app?.name).toBe("Zoom");
    expect(appsMock.lastUninstall?.leftovers.length).toBe(4);
    expect(screen.queryByRole("button", { name: /Zoom/ })).toBeNull();
  });

  it("blocks removing a running app", async () => {
    const user = userEvent.setup();
    render(<AppsView onSeeHistory={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: /Slack/ }));
    expect(await screen.findByText("Sedang berjalan")).toBeInTheDocument();
    expect(await screen.findByRole("button", { name: /Hapus aplikasi, 420,0 MB/ })).toBeDisabled();
  });

  it("orphans tab selects all and moves them", async () => {
    const user = userEvent.setup();
    render(<AppsView onSeeHistory={vi.fn()} />);
    await screen.findAllByRole("button", { name: /Zoom/ });
    await user.click(screen.getByRole("tab", { name: "Sisa aplikasi" }));
    await screen.findByText(/com.oldvendor.editor/);
    await user.click(screen.getByRole("checkbox", { name: "Pilih semua" }));
    await user.click(screen.getByRole("button", { name: /Pindahkan 2 sisa ke Trash/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(await within(dialog).findByRole("button", { name: "Pindahkan ke Trash" }));
    await screen.findByText(/2 item dipindahkan/);
    expect(appsMock.lastUninstall?.app).toBeNull();
    expect(appsMock.lastUninstall?.leftovers.length).toBe(2);
  });
});
