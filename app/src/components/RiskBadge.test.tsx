import { render, screen } from "@testing-library/react";
import { RiskBadge } from "./RiskBadge";

describe("RiskBadge", () => {
  it.each([
    ["safe", "Aman dihapus"],
    ["review", "Periksa dulu"],
    ["danger", "Jangan disentuh"],
  ] as const)("%s shows text and an icon, not colour alone", (level, text) => {
    const { container } = render(<RiskBadge level={level} />);
    expect(screen.getByText(text)).toBeInTheDocument();
    expect(container.querySelector("svg")).not.toBeNull();
    expect(container.firstElementChild).toHaveAttribute("data-risk", level);
  });
});
