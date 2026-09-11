import { describe, it, expect } from "vitest"
import { logoKeyFor } from "./ailogo"

describe("logoKeyFor", () => {
  it("recognises the bare commands the default config ships", () => {
    expect(logoKeyFor("claude")).toBe("claude")
    expect(logoKeyFor("gemini")).toBe("gemini")
  })

  it("ignores case", () => {
    expect(logoKeyFor("Claude")).toBe("claude")
    expect(logoKeyFor("GEMINI")).toBe("gemini")
  })

  it("recognises a command given as a full path", () => {
    // Users may point at an explicit binary rather than relying on PATH.
    expect(logoKeyFor("C:\\tools\\claude.exe")).toBe("claude")
    expect(logoKeyFor("/usr/local/bin/gemini")).toBe("gemini")
  })

  it("recognises launcher-style commands that carry the name in an argument", () => {
    expect(logoKeyFor("npx @anthropic-ai/claude-code")).toBe("claude")
    expect(logoKeyFor("npx @google/gemini-cli")).toBe("gemini")
  })

  it("returns null for a tool it has no mark for", () => {
    // Custom tools added in settings fall back to an initial badge.
    expect(logoKeyFor("aider")).toBeNull()
    expect(logoKeyFor("")).toBeNull()
  })
})
