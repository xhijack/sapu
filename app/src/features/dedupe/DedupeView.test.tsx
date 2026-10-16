import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DedupeView } from "./DedupeView";
import { defaultKeep } from "./keep";
import { dedupeMock } from "@/lib/dedupeApi";

const MB = 1024 ** 2;

describe("defaultKeep mirrors core strategies", () => {
  const g = {
    id: 1,
    size: MB,
    hash: "h",
    wasted_bytes: 2 * MB,
    files: [
      { path: "/a/IMG.jpg", mtime: { secs_since_epoch: 100, nanos_since_epoch: 0 }, name_score: 0 },
      { path: "/b/IMG (1).jpg", mtime: { secs_since_epoch: 300, nanos_since_epoch: 0 }, name_score: -15 },
      { path: "/c/IMG copy.jpg", mtime: { secs_since_epoch: 200, nanos_since_epoch: 0 }, name_score: -10 },
    ],
  };
  it("picks by name, oldest, newest", () => {
    expect(defaultKeep(g, "original_name")).toBe("/a/IMG.jpg");
    expect(defaultKeep(g, "oldest")).toBe("/a/IMG.jpg");
    expect(defaultKeep(g, "newest")).toBe("/b/IMG (1).jpg");
  });
});

describe("DedupeView", () => {
  it("runs the three stages and shows results with folder pairs", async () => {
    const user = userEvent.setup();
    render(<DedupeView onSeeHistory={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Cari duplikat" }));
    expect(await screen.findByRole("progressbar")).toBeInTheDocument();
    await screen.findByText(/9,5 MB bisa dibebaskan/, {}, { timeout: 3000 });
    expect(screen.getByText("Per folder")).toBeInTheDocument();
    expect(screen.getAllByRole("radio").length).toBe(5);
  });

  it("strategy changes the kept copy; explicit pick wins; preview then execute", async () => {
    const user = userEvent.setup();
    render(<DedupeView onSeeHistory={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Cari duplikat" }));
    await screen.findByText(/bisa dibebaskan/, {}, { timeout: 3000 });

    const original = screen.getByRole("radio", { name: /Pictures\/liburan\/IMG_2041.jpg/ });
    expect(original).toBeChecked();
    await user.selectOptions(screen.getByRole("combobox"), "newest");
    expect(screen.getByRole("radio", { name: /IMG_2041 copy.jpg/ })).toBeChecked();

    await user.click(screen.getByRole("radio", { name: /Downloads\/IMG_2041 \(1\).jpg/ }));
    expect(screen.getByRole("radio", { name: /Downloads\/IMG_2041 \(1\).jpg/ })).toBeChecked();

    await user.click(screen.getByRole("button", { name: /Hapus duplikat/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Pindahkan ke Trash" }));
    await screen.findByRole("heading", { name: "Selesai" });
    expect(dedupeMock.lastExecute?.apply).toBe(true);
    expect(dedupeMock.lastExecute?.strategy).toEqual({ kind: "newest" });
    expect(dedupeMock.lastExecute?.keeps[1]).toMatch(/Downloads\/IMG_2041 \(1\).jpg/);
  });

  it("cancelling the preview runs nothing", async () => {
    const user = userEvent.setup();
    render(<DedupeView onSeeHistory={vi.fn()} />);
    await user.click(await screen.findByRole("button", { name: "Cari duplikat" }));
    await screen.findByText(/bisa dibebaskan/, {}, { timeout: 3000 });
    await user.click(screen.getByRole("button", { name: /Hapus duplikat/ }));
    const dialog = await screen.findByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Batal" }));
    expect(dedupeMock.lastExecute).toBeNull();
  });
});
