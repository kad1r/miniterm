import { describe, expect, it } from "vitest"
import {
  DEFAULT_SIDEBAR_FONTS, SIDEBAR_FONTS, choiceLabel, fontStack, isValidChoice, parseSidebarFonts,
  quoteFamily, systemChoice, systemFamily,
} from "./sidebarFont"
import fontsCss from "../fonts.css?raw"
import sidebarCss from "../sidebarFonts.css?raw"

describe("catalog", () => {
  it("offers at most 20 fonts with unique ids", () => {
    expect(SIDEBAR_FONTS.length).toBeLessThanOrEqual(20)
    expect(new Set(SIDEBAR_FONTS.map((f) => f.id)).size).toBe(SIDEBAR_FONTS.length)
  })

  // A catalog entry without a matching @font-face silently renders in the
  // fallback, which looks like a working picker that does nothing.
  it("declares every catalog family in the bundled CSS, latin and latin-ext", () => {
    const css = fontsCss + sidebarCss
    for (const f of SIDEBAR_FONTS) {
      const faces = css.split("@font-face").filter((b) => b.includes(`font-family: "${f.family}"`))
      expect(faces.some((b) => b.includes("-latin-wght-normal")), f.family).toBe(true)
      expect(faces.some((b) => b.includes("-latin-ext-wght-normal")), f.family).toBe(true)
    }
  })
})

describe("choices", () => {
  it("round-trips a system font", () => {
    const c = systemChoice("  Segoe UI ")
    expect(systemFamily(c)).toBe("Segoe UI")
    expect(choiceLabel(c)).toBe("Segoe UI")
    expect(isValidChoice(c)).toBe(true)
  })

  it("rejects unknown ids and empty system names", () => {
    expect(isValidChoice("comic-sans")).toBe(false)
    expect(isValidChoice("system:  ")).toBe(false)
    expect(isValidChoice(42)).toBe(false)
  })

  it("builds a stack with a fallback of the right kind", () => {
    expect(fontStack("inter", "sans")).toBe('"Inter Variable", system-ui, "Segoe UI", sans-serif')
    expect(fontStack(systemChoice("Consolas"), "mono")).toMatch(/^"Consolas", "Cascadia Mono"/)
    expect(fontStack("nope", "mono")).toMatch(/^"Roboto Mono Variable"/)
  })

  it("escapes quotes so a typed name cannot break out of the declaration", () => {
    expect(quoteFamily('a"b\\c')).toBe('"a\\"b\\\\c"')
  })
})

describe("parseSidebarFonts", () => {
  it("falls back field by field", () => {
    expect(parseSidebarFonts(null)).toEqual(DEFAULT_SIDEBAR_FONTS)
    expect(parseSidebarFonts("{bad")).toEqual(DEFAULT_SIDEBAR_FONTS)
    expect(parseSidebarFonts('{"text":"inter","mono":"bogus"}')).toEqual({
      text: "inter",
      mono: DEFAULT_SIDEBAR_FONTS.mono,
    })
  })
})
