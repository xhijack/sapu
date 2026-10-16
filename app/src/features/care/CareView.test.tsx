import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CareView } from "./CareView";
import { cleanMock } from "@/lib/cleanApi";

describe("CareView", () => {
  it("runs both stages, shows the score and recommendations, then cleans the safe set", async () => {
    const user = userEvent.setup();
    const nav = vi.fn();
    render(<CareView onNavigate={nav} />);
    await user.click(await screen.findByRole("button", { name: "Rawat sekarang" }));
    expect(await screen.findByRole("progressbar")).toBeInTheDocument();
    await screen.findByText("Memeriksa apa yang aman dibersihkan", {}, { timeout: 3000 });

    await screen.findByText("Mulai sesak. Beberapa langkah di bawah akan membantu.", {}, { timeout: 3000 });
    expect(screen.getByText(/Bersihkan 22,6 GB yang aman. Semua ke Trash dulu./)).toBeInTheDocument();
    expect(screen.getByText(/Folder terbesar: dev/)).toBeInTheDocument();
    expect(screen.getByText("20/40")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /Buka hasil pindai/ }));
    expect(nav).toHaveBeenCalledWith("scan");

    await user.click(screen.getByRole("button", { name: "Bersihkan 22,6 GB yang aman" }));
    await screen.findByText("Selesai dirawat");
    expect(cleanMock.lastExecute).toMatchObject({ ruleIds: ["macos-browser-caches", "macos-old-logs"], artefacts: ["/Users/you/dev/landing-page-2024/node_modules"], apply: true, permanentOk: false });
  });
});
