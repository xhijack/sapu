import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScanView } from "./ScanView";
import { scanMock } from "@/lib/scanApi";

beforeEach(() => scanMock.reset());

describe("ScanView", () => {
  it("starts from the home folder, shows progress, then results", async () => {
    const user = userEvent.setup();
    render(<ScanView />);
    expect(await screen.findByRole("heading", { name: "Pindai" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Mulai pindai" }));

    expect(await screen.findByRole("progressbar")).toBeInTheDocument();
    await screen.findByText(/Sekitar .* lagi/);

    await screen.findByText(/212,4 GB dipakai/, {}, { timeout: 3000 });
    expect(screen.getByRole("img", { name: "Pembagian ruang per kategori" })).toBeInTheDocument();
    expect(screen.getByText(/3 folder tidak bisa dibaca/)).toBeInTheDocument();
  });

  it("can pause, resume and cancel", async () => {
    const user = userEvent.setup();
    scanMock.ticks = 50;
    render(<ScanView />);
    await user.click(await screen.findByRole("button", { name: "Mulai pindai" }));
    await user.click(await screen.findByRole("button", { name: "Jeda" }));
    expect(scanMock.state).toBe("paused");
    await user.click(screen.getByRole("button", { name: "Lanjutkan" }));
    expect(scanMock.state).toBe("running");
    await user.click(screen.getByRole("button", { name: "Batalkan" }));
    await screen.findByText("Pemindaian dihentikan. Hasil di bawah ini belum lengkap.", {}, { timeout: 3000 });
  });

  it("opens the largest list from a category and previews before trashing", async () => {
    const user = userEvent.setup();
    scanMock.ticks = 2;
    render(<ScanView />);
    await user.click(await screen.findByRole("button", { name: "Mulai pindai" }));
    await screen.findByText(/212,4 GB dipakai/, {}, { timeout: 3000 });

    await user.click(screen.getByRole("button", { name: /^Video/ }));
    expect(await screen.findByRole("heading", { name: "File terbesar" })).toBeInTheDocument();
    const rows = await screen.findAllByRole("button", { name: "Pindahkan ke Trash" });
    expect(rows.length).toBeGreaterThan(0);

    await user.click(rows[0]!);
    const dialog = await screen.findByRole("dialog");
    expect(dialog).toHaveTextContent("Pindahkan ke Trash?");
    await within(dialog).findByText(/akan dipindahkan ke Trash/);
    await user.click(within(dialog).getByRole("button", { name: "Pindahkan ke Trash" }));
    await screen.findByText(/1 item dipindahkan ke Trash/);
  });
});

describe("ScanView resume", () => {
  it("shows the running scan when mounted while a scan is in progress", async () => {
    scanMock.state = "running";
    render(<ScanView />);
    expect(await screen.findByRole("progressbar")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Jeda" })).toBeInTheDocument();
  });
});
