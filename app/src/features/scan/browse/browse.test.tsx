import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { BrowseView } from "./BrowseView";
import type { FileRow } from "./FileTable";
import { sortRows } from "./sortRows";

const row = (name: string, bytes: number, kind: FileRow["kind"] = "other", atime: number | null = null): FileRow => ({ path: `/x/${name}`, name, kind, is_dir: false, bytes, atime: atime === null ? null : { secs_since_epoch: atime, nanos_since_epoch: 0 }, mtime: null });

describe("sortRows", () => {
  it("sorts by size, name, kind and last opened", () => {
    const rows = [row("b.mov", 10, "video", 50), row("a.dmg", 30, "disk_image", 10), row("c.pdf", 20, "pdf")];
    expect(sortRows(rows, "size", true).map((r) => r.name)).toEqual(["a.dmg", "c.pdf", "b.mov"]);
    expect(sortRows(rows, "name", false).map((r) => r.name)).toEqual(["a.dmg", "b.mov", "c.pdf"]);
    expect(sortRows(rows, "kind", false).map((r) => r.kind)).toEqual(["disk_image", "pdf", "video"]);
    // Unknown access time sorts as oldest.
    expect(sortRows(rows, "accessed", true).map((r) => r.name)).toEqual(["b.mov", "a.dmg", "c.pdf"]);
  });
});

describe("BrowseView", () => {
  const noop = () => undefined;

  it("lists the Downloads folder by group with a one-click stale installer action", async () => {
    const user = userEvent.setup();
    render(<BrowseView root="/Users/you" initialTab="downloads" onTrash={noop} removed={new Set()} />);
    expect(await screen.findByRole("tab", { name: "Downloads", selected: true })).toBeInTheDocument();
    const groups = await screen.findByRole("radiogroup", { name: "Kelompok" });
    expect(within(groups).getByRole("radio", { name: /Installer/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Buang 4 installer dan arsip lama/ })).toBeInTheDocument();

    await user.click(within(groups).getByRole("radio", { name: /Video/ }));
    const table = screen.getByRole("table");
    expect(within(table).getAllByRole("row")).toHaveLength(2); // header + one video
    expect(within(table).getByText("rekaman-rapat.mov")).toBeInTheDocument();
  });

  it("browses folders through a crumb trail", async () => {
    const user = userEvent.setup();
    render(<BrowseView root="/Users/you" initialTab="folders" onTrash={noop} removed={new Set()} />);
    await screen.findByRole("tab", { name: "Folder", selected: true });
    await user.click(await screen.findByRole("button", { name: "Movies" }));
    expect(await screen.findByText("zoom-rekaman-2025-03.mp4")).toBeInTheDocument();
    const crumbs = screen.getByRole("navigation", { name: "Lokasi" });
    expect(within(crumbs).getByText("Movies")).toHaveAttribute("aria-current", "location");
    await user.click(within(crumbs).getByRole("button", { name: "you" }));
    expect(await screen.findByRole("button", { name: "Movies" })).toBeInTheDocument();
  });

  it("flags apps built for another processor", async () => {
    render(<BrowseView root="/Users/you" initialTab="unsupported" onTrash={noop} removed={new Set()} />);
    expect(await screen.findByText("Zoom")).toBeInTheDocument();
    expect(screen.getByText("Lewat Rosetta")).toBeInTheDocument();
    expect(screen.queryByText("Slack")).toBeNull();
  });
});
