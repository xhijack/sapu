import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Wizard } from "./Wizard";
import { defaultSettings, mock } from "@/lib/api";

describe("Wizard", () => {
  it("walks through all six steps and finishes with onboarding_done", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<Wizard settings={defaultSettings} onFinish={onFinish} />);

    expect(screen.getByText("Langkah 1 dari 6")).toBeInTheDocument();
    for (let i = 1; i < 6; i++) await user.click(screen.getByRole("button", { name: "Lanjut" }));
    expect(screen.getByText("Langkah 6 dari 6")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Selesai" }));

    expect(onFinish).toHaveBeenCalledWith(expect.objectContaining({ onboarding_done: true }));
  });

  it("skip finishes immediately from any step", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<Wizard settings={defaultSettings} onFinish={onFinish} />);
    await user.click(screen.getByRole("button", { name: "Lewati, saya sudah paham" }));
    expect(onFinish).toHaveBeenCalledWith(expect.objectContaining({ onboarding_done: true }));
  });

  it("suggests the developer profile when dev tools are detected", async () => {
    const user = userEvent.setup();
    mock.info = { ...mock.info, dev_tools: ["docker", "node"] };
    render(<Wizard settings={defaultSettings} onFinish={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await waitFor(() => expect(screen.getByText(/docker, node/)).toBeInTheDocument());
    expect(screen.getByRole("radio", { name: /Developer/ })).toBeChecked();
  });

  it("AI defaults to off (P5)", async () => {
    const user = userEvent.setup();
    render(<Wizard settings={defaultSettings} onFinish={vi.fn()} />);
    for (let i = 0; i < 4; i++) await user.click(screen.getByRole("button", { name: "Lanjut" }));
    expect(screen.getByRole("radio", { name: /^Mati/ })).toBeChecked();
  });

  it("macOS access step shows the live permission status", async () => {
    const user = userEvent.setup();
    mock.info = { ...mock.info, os: "macos", full_disk_access: "denied" };
    render(<Wizard settings={defaultSettings} onFinish={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveAttribute("data-status", "denied"));
  });
});

describe("Wizard language first", () => {
  it("switches the UI language immediately from step 1", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<Wizard settings={defaultSettings} onFinish={onFinish} />);
    expect(screen.getByText("Langkah 1 dari 6")).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "English" }));
    expect(await screen.findByText("Step 1 of 6")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Skip, I know this" }));
    expect(onFinish).toHaveBeenCalledWith(expect.objectContaining({ locale: "en" }));
  });
});
