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

  it("always sums to one", () => {
    const next = resizeFractions([0.3, 0.3, 0.4], 1, -50, 800);
    expect(next.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 10);
  });

  it("clamps so neither neighbour falls below the minimum cell size", () => {
    const next = resizeFractions([0.5, 0.5], 0, 10_000, 1000, MIN_CELL_PX);
    expect(next[1] * 1000).toBeGreaterThanOrEqual(MIN_CELL_PX - 0.001);
    expect(next[0] * 1000).toBeGreaterThanOrEqual(MIN_CELL_PX - 0.001);
  });

  it("leaves other cells untouched", () => {
    const next = resizeFractions([0.25, 0.25, 0.5], 0, 50, 1000);
    expect(next[2]).toBeCloseTo(0.5, 10);
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
