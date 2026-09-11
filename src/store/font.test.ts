import { describe, it, expect } from "vitest"
import {
  DEFAULT_FONT_SCALE, FONT_SCALE_KEY, MAX_FONT_SCALE, MIN_FONT_SCALE, fontAction, readFontScale,
  stepFontScale, termFontSize, writeFontScale,
} from "./font"

describe("stepFontScale", () => {
  it("moves one level up and one level down", () => {
    expect(stepFontScale(1, 1)).toBeGreaterThan(1)
    expect(stepFontScale(1, -1)).toBeLessThan(1)
  })

  it("stops at the ends instead of running off the scale", () => {
    expect(stepFontScale(MAX_FONT_SCALE, 1)).toBe(MAX_FONT_SCALE)
    expect(stepFontScale(MIN_FONT_SCALE, -1)).toBe(MIN_FONT_SCALE)
  })

  it("returns to the same level after a round trip", () => {
    expect(stepFontScale(stepFontScale(1, 1), -1)).toBe(1)
  })

  it("snaps an off-scale value onto the nearest level in the travel direction", () => {
    // A hand-edited localStorage entry, or a level dropped from the scale in a
    // later version, must not strand the user between steps.
    expect(stepFontScale(1.02, 1)).toBeGreaterThan(1.02)
    expect(stepFontScale(1.02, -1)).toBeLessThanOrEqual(1)
  })
})

describe("readFontScale / writeFontScale", () => {
  function store(initial: Record<string, string> = {}) {
    const data = { ...initial }
    return {
      data,
      getItem: (k: string) => data[k] ?? null,
      setItem: (k: string, v: string) => void (data[k] = v),
    }
  }

  it("round-trips a level", () => {
    const s = store()
    writeFontScale(s, 1.25)
    expect(readFontScale(s)).toBe(1.25)
  })

  it("falls back to the default when nothing is stored", () => {
    expect(readFontScale(store())).toBe(DEFAULT_FONT_SCALE)
  })

  it("rejects junk and out-of-range values", () => {
    // The scale multiplies every text size in the UI. A stored "0" or "abc"
    // would render the app unreadable with no way back.
    expect(readFontScale(store({ [FONT_SCALE_KEY]: "abc" }))).toBe(DEFAULT_FONT_SCALE)
    expect(readFontScale(store({ [FONT_SCALE_KEY]: "0" }))).toBe(DEFAULT_FONT_SCALE)
    expect(readFontScale(store({ [FONT_SCALE_KEY]: "40" }))).toBe(DEFAULT_FONT_SCALE)
  })

  it("survives a storage that throws", () => {
    const broken = {
      getItem: () => { throw new Error("denied") },
      setItem: () => { throw new Error("denied") },
    }
    expect(readFontScale(broken)).toBe(DEFAULT_FONT_SCALE)
    expect(() => writeFontScale(broken, 1.5)).not.toThrow()
  })
})

describe("termFontSize", () => {
  it("scales the base size and keeps it a whole number", () => {
    // xterm derives cell geometry from the font size; a fractional size gives
    // fractional cells and a blurred grid.
    expect(termFontSize(13, 1)).toBe(13)
    expect(termFontSize(13, 1.5)).toBe(20)
    expect(Number.isInteger(termFontSize(13, 1.1))).toBe(true)
  })

  it("never returns a size below 1", () => {
    expect(termFontSize(13, 0.01)).toBeGreaterThanOrEqual(1)
  })
})

describe("fontAction", () => {
  it("recognises the ctrl and cmd accelerators", () => {
    expect(fontAction({ key: "+", ctrlKey: true, metaKey: false })).toBe("in")
    expect(fontAction({ key: "=", ctrlKey: true, metaKey: false })).toBe("in")
    expect(fontAction({ key: "-", ctrlKey: true, metaKey: false })).toBe("out")
    expect(fontAction({ key: "0", ctrlKey: false, metaKey: true })).toBe("reset")
  })

  it("ignores the same keys without a modifier", () => {
    // Bare "-" and "0" are ordinary shell input; swallowing them would make
    // every terminal unusable.
    expect(fontAction({ key: "-", ctrlKey: false, metaKey: false })).toBe(null)
    expect(fontAction({ key: "0", ctrlKey: false, metaKey: false })).toBe(null)
  })

  it("ignores unrelated modified keys", () => {
    expect(fontAction({ key: "c", ctrlKey: true, metaKey: false })).toBe(null)
  })
})
