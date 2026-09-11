import { describe, it, expect } from "vitest"
import { isClosePaneChord } from "./pane"

const key = (over: Partial<Parameters<typeof isClosePaneChord>[0]>) => ({
  key: "w",
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  metaKey: false,
  ...over,
})

describe("isClosePaneChord", () => {
  it("matches Ctrl+Shift+W", () => {
    expect(isClosePaneChord(key({ ctrlKey: true, shiftKey: true }))).toBe(true)
  })

  it("leaves Ctrl+W to the shell, where it deletes a word", () => {
    expect(isClosePaneChord(key({ ctrlKey: true }))).toBe(false)
  })

  it("ignores the chord when Alt is held", () => {
    expect(isClosePaneChord(key({ ctrlKey: true, shiftKey: true, altKey: true }))).toBe(false)
  })

  it("ignores other letters", () => {
    expect(isClosePaneChord(key({ key: "q", ctrlKey: true, shiftKey: true }))).toBe(false)
  })
})
