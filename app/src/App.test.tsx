import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import App from "./App";
import { mock } from "@/lib/api";

describe("App", () => {
  it("shows the wizard on first run and the shell afterwards", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByText("Selamat datang di Sapu");
    await user.click(screen.getByRole("button", { name: "Lewati, saya sudah paham" }));
    await waitFor(() => expect(mock.settings.onboarding_done).toBe(true));
    expect(screen.getByRole("navigation")).toBeInTheDocument();
  });

  it("skips the wizard when onboarding is done and navigates", async () => {
    const user = userEvent.setup();
    mock.settings = { ...mock.settings, onboarding_done: true };
    render(<App />);
    await screen.findByRole("navigation");
    expect(await screen.findByText("Macintosh HD")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Riwayat" }));
    expect(await screen.findByText("Belum ada pembersihan")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Pengaturan" }));
    expect(await screen.findByText("Lokasi yang dilindungi")).toBeInTheDocument();
  });

  it("switches language from settings", async () => {
    const user = userEvent.setup();
    mock.settings = { ...mock.settings, onboarding_done: true };
    render(<App />);
    await screen.findByRole("navigation");
    await user.click(screen.getByRole("button", { name: "Pengaturan" }));
    await user.selectOptions(await screen.findByLabelText("Bahasa"), "en");
    expect(await screen.findByRole("button", { name: "Settings" })).toBeInTheDocument();
    expect(document.documentElement.lang).toBe("en");
  });
});
