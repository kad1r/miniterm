import { describe, it, expect } from "vitest"
import { cells, dividerCount, templateWithDividers } from "./grid"

describe("cells", () => {
  it("numbers cells row-major", () => {
    expect(cells(2, 3)).toEqual([
      { index: 0, row: 0, col: 0 },
      { index: 1, row: 0, col: 1 },
      { index: 2, row: 0, col: 2 },
      { index: 3, row: 1, col: 0 },
      { index: 4, row: 1, col: 1 },
      { index: 5, row: 1, col: 2 },
    ])
  })

  it("handles a single cell", () => {
    expect(cells(1, 1)).toEqual([{ index: 0, row: 0, col: 0 }])
  })
})

describe("dividerCount", () => {
  it("is one less than the track count", () => {
    expect(dividerCount(3)).toBe(2)
    expect(dividerCount(1)).toBe(0)
  })
})

describe("templateWithDividers", () => {
  it("interleaves fixed divider tracks between fractions", () => {
    expect(templateWithDividers([0.25, 0.75])).toBe("0.25fr 4px 0.75fr")
  })

  it("leaves a single track untouched", () => {
    expect(templateWithDividers([1])).toBe("1fr")
  })

  it("honours a custom divider width", () => {
    expect(templateWithDividers([0.5, 0.5], 6)).toBe("0.5fr 6px 0.5fr")
  })
})
