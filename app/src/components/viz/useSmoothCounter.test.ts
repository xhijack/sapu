import { counterStep } from "./useSmoothCounter";

describe("counterStep", () => {
  it("grows with the magnitude so digits stay readable", () => {
    expect(counterStep(120)).toBe(1);
    expect(counterStep(12_000)).toBe(10);
    expect(counterStep(120_000)).toBe(50);
    expect(counterStep(1_200_000)).toBe(100);
  });
});
