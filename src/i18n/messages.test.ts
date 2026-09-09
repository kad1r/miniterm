import { describe, it, expect } from "vitest"
import {
  LOCALES, LOCALE_KEY, isLocale, readLocale, resolveLocale, translate, writeLocale,
} from "./messages"

function store(initial: Record<string, string> = {}) {
  const data = { ...initial }
  return {
    data,
    getItem: (k: string) => data[k] ?? null,
    setItem: (k: string, v: string) => void (data[k] = v),
  }
}

describe("translate", () => {
  it("returns the message for the requested locale", () => {
    expect(translate("en", "settings.save")).toBe("Save")
    expect(translate("tr", "settings.save")).toBe("Kaydet")
  })

  it("fills placeholders", () => {
    expect(translate("en", "settings.terminal.found", { count: 4 })).toBe("4 shells found")
    expect(translate("tr", "sidebar.deleted", { name: "API" })).toBe('"API" silindi')
  })

  it("leaves an unknown placeholder verbatim so the typo is visible", () => {
    expect(translate("en", "sidebar.deleted", { wrong: "x" })).toBe('"{name}" deleted')
  })

  // Key coverage itself is a compile-time guarantee: the Turkish table is typed
  // as Record<keyof typeof EN, string>. This only guards against a blank entry.
  it("has a non-empty message in every locale", () => {
    for (const locale of LOCALES) {
      expect(translate(locale, "wizard.create").trim().length).toBeGreaterThan(0)
      expect(translate(locale, "app.loading").trim().length).toBeGreaterThan(0)
    }
  })
})

describe("resolveLocale", () => {
  it("prefers the stored choice", () => {
    expect(resolveLocale("tr", ["en-US"])).toBe("tr")
    expect(resolveLocale("en", ["tr-TR"])).toBe("en")
  })

  it("falls back to the first supported system language", () => {
    expect(resolveLocale(null, ["de-DE", "tr-TR", "en"])).toBe("tr")
    expect(resolveLocale("klingon", ["en-GB"])).toBe("en")
  })

  it("defaults to English when nothing matches", () => {
    expect(resolveLocale(null, ["de", "fr"])).toBe("en")
    expect(resolveLocale(null, [])).toBe("en")
  })
})

describe("isLocale", () => {
  it("accepts only the supported tags", () => {
    expect(isLocale("tr")).toBe(true)
    expect(isLocale("en")).toBe(true)
    expect(isLocale("de")).toBe(false)
    expect(isLocale(null)).toBe(false)
  })
})

describe("readLocale / writeLocale", () => {
  it("round-trips through storage", () => {
    const s = store()
    writeLocale(s, "tr")
    expect(s.data[LOCALE_KEY]).toBe("tr")
    expect(readLocale(s, ["en"])).toBe("tr")
  })

  it("detects when nothing is stored", () => {
    expect(readLocale(store(), ["tr-TR"])).toBe("tr")
  })

  it("survives storage that throws", () => {
    const broken = {
      getItem() { throw new Error("denied") },
      setItem() { throw new Error("denied") },
    }
    expect(readLocale(broken, ["tr"])).toBe("tr")
    expect(() => writeLocale(broken, "en")).not.toThrow()
  })

  it("works without storage at all", () => {
    expect(readLocale(undefined, ["en"])).toBe("en")
    expect(() => writeLocale(undefined, "tr")).not.toThrow()
  })
})
