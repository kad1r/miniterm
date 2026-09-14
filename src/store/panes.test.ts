import { describe, expect, it } from "vitest"
import { clearTool, normalizeTools, toolsFor } from "./panes"
import type { Workspace } from "./types"

function ws(patch: Partial<Workspace> = {}): Workspace {
  return {
    id: "w",
    kind: "workspace",
    name: "spock",
    path: "C:\\projects\\spock",
    aiToolId: "claude",
    shellId: null,
    rows: 2,
    cols: 2,
    rowSizes: [50, 50],
    colSizes: [50, 50],
    paneTools: [],
    ...patch,
  }
}

describe("normalizeTools", () => {
  it("pads a short list with the workspace default", () => {
    expect(normalizeTools(["gemini"], 3, "claude")).toEqual(["gemini", "claude", "claude"])
  })

  it("fills a missing list entirely, which is what an older config looks like", () => {
    expect(normalizeTools([], 2, "claude")).toEqual(["claude", "claude"])
  })

  it("drops entries past the end of the grid", () => {
    expect(normalizeTools(["a", "b", "c"], 2, null)).toEqual(["a", "b"])
  })

  it("keeps an explicit bare shell rather than treating it as missing", () => {
    expect(normalizeTools([null, "gemini"], 2, "claude")).toEqual([null, "gemini"])
  })

  it("pads with null when the workspace itself has no tool", () => {
    expect(normalizeTools([], 2, null)).toEqual([null, null])
  })
})

describe("toolsFor", () => {
  it("gives one entry per cell of the grid", () => {
    expect(toolsFor(ws({ rows: 2, cols: 3 }))).toHaveLength(6)
  })

  it("keeps the assignments a workspace already has", () => {
    const w = ws({ rows: 1, cols: 4, paneTools: ["claude", "claude", null, "gemini"] })
    expect(toolsFor(w)).toEqual(["claude", "claude", null, "gemini"])
  })

  it("falls back to the workspace tool for cells added since", () => {
    const w = ws({ rows: 1, cols: 3, paneTools: ["gemini"] })
    expect(toolsFor(w)).toEqual(["gemini", "claude", "claude"])
  })
})

describe("clearTool", () => {
  it("nulls every cell that used the deleted tool", () => {
    expect(clearTool(["claude", "gemini", "claude"], "claude")).toEqual([null, "gemini", null])
  })

  it("leaves the others alone", () => {
    expect(clearTool(["gemini", null], "claude")).toEqual(["gemini", null])
  })
})
