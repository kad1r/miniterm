import { describe, expect, it } from "vitest";
import { equalSizes, layoutsFor, MIN_CELL_PX, resizeFractions, toGridTemplate } from "./layout";

describe("layoutsFor", () => {
  it("offers every factor pair, rows ascending", () => {
    expect(layoutsFor(1)).toEqual([{ rows: 1, cols: 1 }]);
    expect(layoutsFor(2)).toEqual([{ rows: 1, cols: 2 }, { rows: 2, cols: 1 }]);
    expect(layoutsFor(4)).toEqual([
      { rows: 1, cols: 4 },
      { rows: 2, cols: 2 },
      { rows: 4, cols: 1 },
    ]);
    expect(layoutsFor(5)).toEqual([{ rows: 1, cols: 5 }, { rows: 5, cols: 1 }]);
    expect(layoutsFor(6)).toEqual([
      { rows: 1, cols: 6 },
      { rows: 2, cols: 3 },
      { rows: 3, cols: 2 },
      { rows: 6, cols: 1 },
    ]);
  });

  it("returns nothing outside 1..6", () => {
    expect(layoutsFor(0)).toEqual([]);
    expect(layoutsFor(7)).toEqual([]);
  });
});

describe("equalSizes", () => {
  it("splits evenly and sums to one", () => {
    expect(equalSizes(4)).toEqual([0.25, 0.25, 0.25, 0.25]);
    const sum = equalSizes(3).reduce((a, b) => a + b, 0);
    expect(sum).toBeCloseTo(1, 10);
  });
});

describe("resizeFractions", () => {
  it("moves space from one cell to its neighbour", () => {
    const next = resizeFractions([0.5, 0.5], 0, 100, 1000);
    expect(next[0]).toBeCloseTo(0.6, 10);
    expect(next[1]).toBeCloseTo(0.4, 10);
  });

  it("transfers space between the two adjacent tracks and conserves the total", () => {
    // Divider 1 sits between cells[1] and cells[2]; cells[0] must not move.
    const before = [0.3, 0.3, 0.4];
    const next = resizeFractions(before, 1, -50, 800);
    const deltaFrac = -50 / 800; // −0.0625
    // cells[1] shrinks and cells[2] grows by the same delta.
    expect(next[1]).toBeCloseTo(before[1] + deltaFrac, 10);
    expect(next[2]).toBeCloseTo(before[2] - deltaFrac, 10);
    // Non-adjacent track is byte-identical.
    expect(next[0]).toBe(before[0]);
    // Total is conserved.
    expect(next.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 10);
  });

  it("clamps so neither neighbour falls below the minimum cell size", () => {
    const next = resizeFractions([0.5, 0.5], 0, 10_000, 1000, MIN_CELL_PX);
    expect(next[1] * 1000).toBeGreaterThanOrEqual(MIN_CELL_PX - 0.001);
    expect(next[0] * 1000).toBeGreaterThanOrEqual(MIN_CELL_PX - 0.001);
  });

  it("only changes the two tracks adjacent to the dragged divider", () => {
    // Divider 0 sits between cells[0] and cells[1]; cells[2] must be untouched.
    const before = [0.25, 0.25, 0.5];
    const next = resizeFractions(before, 0, 50, 1000);
    const deltaFrac = 50 / 1000; // 0.05
    // Adjacent tracks change by the expected delta.
    expect(next[0]).toBeCloseTo(before[0] + deltaFrac, 10);
    expect(next[1]).toBeCloseTo(before[1] - deltaFrac, 10);
    // Non-adjacent track is byte-identical (reference equality on the number).
    expect(next[2]).toBe(before[2]);
    // Total is conserved.
    expect(next.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 10);
  });

  it("ignores an out-of-range divider index", () => {
    expect(resizeFractions([0.5, 0.5], 1, 100, 1000)).toEqual([0.5, 0.5]);
    expect(resizeFractions([0.5, 0.5], -1, 100, 1000)).toEqual([0.5, 0.5]);
  });
});

describe("toGridTemplate", () => {
  it("emits fr units", () => {
    expect(toGridTemplate([0.25, 0.75])).toBe("0.25fr 0.75fr");
  });
});
