import { render } from "@testing-library/react";
import { DiskStrip } from "./DiskStrip";

describe("DiskStrip", () => {
  it("sizes each segment by share and skips empty categories", () => {
    const { container } = render(
      <DiskStrip
        total={100}
        categories={[
          { category: "video", bytes: 50, files: 1 },
          { category: "photo", bytes: 25, files: 1 },
          { category: "other", bytes: 0, files: 0 },
        ]}
      />,
    );
    const segs = container.querySelectorAll("[data-category]");
    expect(segs.length).toBe(2);
    expect((segs[0] as HTMLElement).style.width).toBe("50%");
    expect((segs[1] as HTMLElement).style.width).toBe("25%");
    expect(segs[0]?.getAttribute("title")).toContain("Video");
  });
});
