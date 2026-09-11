import { describe, it, expect } from "vitest"
import { addableCounts, defaultLayoutFor, relayout, terminalCount } from "./relayout"

const grid = (rows: number, cols: number) => ({
  rows,
  cols,
  rowSizes: Array.from({ length: rows }, () => 1 / rows),
  colSizes: Array.from({ length: cols }, () => 1 / cols),
})

describe("defaultLayoutFor", () => {
  it("keeps a row of terminals a row", () => {
    expect(defaultLayoutFor({ rows: 1, cols: 2 }, 3)).toEqual({ rows: 1, cols: 3 })
  })

  it("keeps a column of terminals a column", () => {
    expect(defaultLayoutFor({ rows: 2, cols: 1 }, 3)).toEqual({ rows: 3, cols: 1 })
  })

  it("splits a lone pane side by side, since it has no orientation yet", () => {
    expect(defaultLayoutFor({ rows: 1, cols: 1 }, 2)).toEqual({ rows: 1, cols: 2 })
  })

  it("falls back to the first valid shape when the current one cannot survive", () => {
    // 5 is prime: a 2×2 has no 2-row successor.
    expect(defaultLayoutFor({ rows: 2, cols: 2 }, 5)).toEqual({ rows: 1, cols: 5 })
  })
})

describe("relayout", () => {
  it("redistributes only the axis that changed", () => {
    const ws = { rows: 1, cols: 2, rowSizes: [1], colSizes: [0.7, 0.3] }
    expect(relayout(ws, { rows: 2, cols: 2 })).toEqual({
      rows: 2,
      cols: 2,
      rowSizes: [0.5, 0.5],
      colSizes: [0.7, 0.3],
    })
  })

  it("does not hand back the caller's own size array", () => {
    const ws = { rows: 1, cols: 2, rowSizes: [1], colSizes: [0.7, 0.3] }
    const next = relayout(ws, { rows: 2, cols: 2 })
    expect(next.colSizes).not.toBe(ws.colSizes)
  })

  it("resets both axes when the grid is transposed", () => {
    const ws = { rows: 1, cols: 3, rowSizes: [1], colSizes: [0.5, 0.3, 0.2] }
    expect(relayout(ws, { rows: 3, cols: 1 })).toEqual({
      rows: 3,
      cols: 1,
      rowSizes: [1 / 3, 1 / 3, 1 / 3],
      colSizes: [1],
    })
  })
})

describe("addableCounts", () => {
  it("starts at the count the workspace already has", () => {
    expect(addableCounts({ rows: 1, cols: 3 })).toEqual([3, 4, 5, 6])
  })

  it("offers nothing but the status quo at the grid limit", () => {
    expect(addableCounts({ rows: 2, cols: 3 })).toEqual([6])
  })
})

describe("terminalCount", () => {
  it("is the grid area", () => {
    expect(terminalCount(grid(2, 3))).toBe(6)
  })
})
