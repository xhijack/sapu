import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { HomeView } from "./HomeView";

describe("HomeView", () => {
  it("shows every disk with used/free, cpu and memory, and quick actions", async () => {
    const user = userEvent.setup();
    const nav = vi.fn();
    render(<HomeView onNavigate={nav} />);
    expect(await screen.findByText("Macintosh HD")).toBeInTheDocument();
    expect(screen.getByText("Backup")).toBeInTheDocument();
    expect(screen.getByText(/433,0 GB terpakai dari 494,0 GB/)).toBeInTheDocument();
    expect(screen.getByText("24%")).toBeInTheDocument(); // cpu rounded
    expect(screen.getByText("62%")).toBeInTheDocument(); // memory 9.92/16
    await user.click(screen.getByRole("button", { name: /Pindai disk/ }));
    expect(nav).toHaveBeenCalledWith("scan");
  });
});
