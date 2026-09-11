import { describe, it, expect } from "vitest"
import { focusAfterClose } from "./focus"

describe("focusAfterClose", () => {
  it("follows the focused pane down when an earlier one is closed", () => {
    expect(focusAfterClose(2, 0, 3)).toBe(1)
  })

  it("leaves the focused pane alone when a later one is closed", () => {
    expect(focusAfterClose(1, 2, 3)).toBe(1)
  })

  it("hands focus to the pane that slid into the closed cell", () => {
    expect(focusAfterClose(1, 1, 3)).toBe(1)
  })

  it("steps back when the closed pane was the last one", () => {
    expect(focusAfterClose(3, 3, 3)).toBe(2)
  })

  it("lands on the only survivor", () => {
    expect(focusAfterClose(1, 1, 1)).toBe(0)
  })
})
