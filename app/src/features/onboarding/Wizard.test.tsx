import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Wizard } from "./Wizard";
import { defaultSettings, mock } from "@/lib/api";

describe("Wizard", () => {
  /** Step 1 is the language, step 2 the notice that must be accepted. */
  const acceptNotice = async (user: ReturnType<typeof userEvent.setup>) => {
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    expect(screen.getByRole("button", { name: "Lanjut" })).toBeDisabled();
    await user.click(screen.getByRole("checkbox", { name: /Saya sudah membaca/ }));
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
  };

  it("walks through all seven steps and finishes with onboarding_done and the notice accepted", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<Wizard settings={defaultSettings} onFinish={onFinish} />);

    expect(screen.getByText("Langkah 1 dari 7")).toBeInTheDocument();
    await acceptNotice(user);
    for (let i = 3; i < 7; i++) await user.click(screen.getByRole("button", { name: "Lanjut" }));
    expect(screen.getByText("Langkah 7 dari 7")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Selesai" }));

    expect(onFinish).toHaveBeenCalledWith(expect.objectContaining({ onboarding_done: true, disclaimer_accepted: true }));
  });

  it("skip is only possible once the notice is accepted", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<Wizard settings={defaultSettings} onFinish={onFinish} />);
    expect(screen.getByRole("button", { name: "Lewati, saya sudah paham" })).toBeDisabled();
    await acceptNotice(user);
    await user.click(screen.getByRole("button", { name: "Lewati, saya sudah paham" }));
    expect(onFinish).toHaveBeenCalledWith(expect.objectContaining({ onboarding_done: true, disclaimer_accepted: true }));
  });

  it("suggests the developer profile when dev tools are detected", async () => {
    const user = userEvent.setup();
    mock.info = { ...mock.info, dev_tools: ["docker", "node"] };
    render(<Wizard settings={{ ...defaultSettings, disclaimer_accepted: true }} onFinish={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await waitFor(() => expect(screen.getByText(/docker, node/)).toBeInTheDocument());
    expect(screen.getByRole("radio", { name: /Developer/ })).toBeChecked();
  });

  it("AI defaults to off (P5)", async () => {
    const user = userEvent.setup();
    render(<Wizard settings={{ ...defaultSettings, disclaimer_accepted: true }} onFinish={vi.fn()} />);
    for (let i = 0; i < 5; i++) await user.click(screen.getByRole("button", { name: "Lanjut" }));
    expect(screen.getByRole("radio", { name: /^Mati/ })).toBeChecked();
  });

  it("macOS access step shows the live permission status", async () => {
    const user = userEvent.setup();
    mock.info = { ...mock.info, os: "macos", full_disk_access: "denied" };
    render(<Wizard settings={{ ...defaultSettings, disclaimer_accepted: true }} onFinish={vi.fn()} />);
    for (let i = 0; i < 3; i++) await user.click(screen.getByRole("button", { name: "Lanjut" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveAttribute("data-status", "denied"));
  });
});

describe("Wizard language first", () => {
  it("switches the UI language immediately from step 1", async () => {
    const user = userEvent.setup();
    const onFinish = vi.fn();
    render(<Wizard settings={{ ...defaultSettings, disclaimer_accepted: true }} onFinish={onFinish} />);
    expect(screen.getByText("Langkah 1 dari 7")).toBeInTheDocument();
    await user.click(screen.getByRole("radio", { name: "English" }));
    expect(await screen.findByText("Step 1 of 7")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Skip, I know this" }));
    expect(onFinish).toHaveBeenCalledWith(expect.objectContaining({ locale: "en" }));
  });
});
