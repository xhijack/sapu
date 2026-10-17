import { render, screen } from "@testing-library/react";
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
    expect(screen.getByText(/41,2 GB dibebaskan sejauh ini/)).toBeInTheDocument();
    expect(screen.getByText("Downloads")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Pindai disk sekarang" }));
    expect(nav).toHaveBeenCalledWith("scan");
    await user.click(screen.getByRole("button", { name: "Rawat Mac saya" }));
    expect(nav).toHaveBeenCalledWith("care");
    await user.click(screen.getByRole("button", { name: /^Duplikat/ }));
    expect(nav).toHaveBeenCalledWith("dedupe");
  });
});
