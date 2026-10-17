import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { mock } from "@/lib/api";

describe("App", () => {
  it("shows the wizard on first run and the shell afterwards", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Selamat datang di Sapu");
    // The notice on step 2 must be accepted before anything can be skipped.
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await user.click(screen.getByRole("checkbox", { name: /Saya sudah membaca/ }));
    await user.click(screen.getByRole("button", { name: "Lewati, saya sudah paham" }));
    await waitFor(() => expect(mock.settings.onboarding_done).toBe(true));
    expect(mock.settings.disclaimer_accepted).toBe(true);
    expect(screen.getByRole("navigation")).toBeInTheDocument();
  });

  it("gates people who set up Sapu before the notice existed", async () => {
    const user = userEvent.setup();
    mock.settings = { ...mock.settings, onboarding_done: true, disclaimer_accepted: false };
    render(<App />);
    await screen.findByText("Sebelum mulai");
    expect(screen.getByRole("button", { name: "Lanjut" })).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: /Saya sudah membaca/ }));
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await screen.findByRole("navigation");
    expect(mock.settings.disclaimer_accepted).toBe(true);
  });

  it("skips the wizard when onboarding is done and navigates", async () => {
    const user = userEvent.setup();
    mock.settings = { ...mock.settings, onboarding_done: true, disclaimer_accepted: true };
    render(<App />);
    await screen.findByRole("navigation");
    expect(await screen.findByRole("img", { name: /Macintosh HD/ })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Riwayat" }));
    expect(await screen.findByText("Belum ada pembersihan")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Pengaturan" }));
    await user.click(await screen.findByRole("tab", { name: "Lokasi terlindungi" }));
    expect(await screen.findByText("Lokasi yang dilindungi")).toBeInTheDocument();
    await user.click(screen.getAllByRole("button", { name: "Tentang" })[1]!);
    expect((await screen.findAllByText(/PT Sopwer Teknologi Indonesia/)).length).toBeGreaterThan(0);
  });

  it("switches language from settings", async () => {
    const user = userEvent.setup();
    mock.settings = { ...mock.settings, onboarding_done: true, disclaimer_accepted: true };
    render(<App />);
    await screen.findByRole("navigation");
    await user.click(screen.getByRole("button", { name: "Pengaturan" }));
    await user.selectOptions(await screen.findByLabelText("Bahasa"), "en");
    expect(await screen.findByRole("button", { name: "Settings" })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("en");
  });
});
