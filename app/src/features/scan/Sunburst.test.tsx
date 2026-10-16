import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Sunburst } from "./Sunburst";
import { scanMock } from "@/lib/scanApi";

describe("Sunburst", () => {
  it("draws arcs, selects, drills in, and collects a basket", async () => {
    const user = userEvent.setup();
    scanMock.summary = { root: "/Users/you" } as never;
    const onTrash = vi.fn();
    render(<Sunburst root="/Users/you" onTrash={onTrash} />);
    const arcs = await screen.findAllByRole("button", { name: /GB$/ });
    expect(arcs.length).toBeGreaterThan(2);

    await user.click(screen.getByRole("button", { name: /^dev, / }));
    expect(screen.getByText("/Users/you/dev")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Tambah ke keranjang" }));
    expect(screen.getByText(/1 item, 61,7 GB/)).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /^Movies, / }));
    await user.click(screen.getByRole("button", { name: "Tambah ke keranjang" }));
    expect(screen.getByText(/2 item, /)).toBeInTheDocument();

    const basket = screen.getByRole("heading", { name: "Keranjang" }).closest("div")!.parentElement!;
    await user.click(within(basket).getByRole("button", { name: "Pindahkan keranjang ke Trash" }));
    expect(onTrash).toHaveBeenCalledWith(["/Users/you/dev", "/Users/you/Movies"]);

    await user.click(screen.getByRole("button", { name: "Masuk" }));
    expect((await screen.findAllByTitle("/Users/you/Movies")).length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: "Naik" }));
    expect((await screen.findAllByTitle("/Users/you")).length).toBeGreaterThan(0);
  });
});
