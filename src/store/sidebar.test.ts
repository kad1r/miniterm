import { describe, it, expect } from "vitest"
import { flatten, deletePrompt, newFolder } from "./sidebar"
import type { Folder, Node, Workspace } from "./types"

function ws(id: string, name: string, rows = 1, cols = 1): Workspace {
  return {
    id, kind: "workspace", name, path: `C:/${id}`, aiToolId: null, shellId: null,
    rows, cols, rowSizes: [1], colSizes: [1],
  }
}
function folder(id: string, expanded: boolean, children: Node[]): Folder {
  return { id, kind: "folder", name: id, expanded, children }
}

describe("flatten", () => {
  it("returns root nodes at depth 1", () => {
    const rows = flatten([ws("a", "A"), ws("b", "B")])
    expect(rows.map((r) => [r.node.id, r.depth])).toEqual([["a", 1], ["b", 1]])
  })

  it("includes children of expanded folders with increasing depth", () => {
    const rows = flatten([folder("f", true, [ws("a", "A")])])
    expect(rows.map((r) => [r.node.id, r.depth])).toEqual([["f", 1], ["a", 2]])
  })

  it("hides children of collapsed folders", () => {
    const rows = flatten([folder("f", false, [ws("a", "A")])])
    expect(rows.map((r) => r.node.id)).toEqual(["f"])
  })
})

describe("deletePrompt", () => {
  it("reports zero when the workspace was never activated", () => {
    // liveSessions always returns 0 — workspace was never activated.
    expect(deletePrompt(ws("a", "API", 2, 3), () => 0)).toBe(
      '"API" silinecek. Açık terminal yok. Devam?',
    )
  })

  it("reports the live session count when the workspace is active", () => {
    // 2×3 workspace that has all 6 sessions running.
    expect(deletePrompt(ws("a", "API", 2, 3), () => 6)).toBe(
      '"API" silinecek. 6 terminal kapanacak. Devam?',
    )
  })

  it("sums live sessions under a folder", () => {
    const f = folder("f", true, [ws("a", "A", 2, 2), ws("b", "B", 1, 3)])
    // "a" has 4 live, "b" has 3 live.
    const live = (id: string) => id === "a" ? 4 : 3
    expect(deletePrompt(f, live)).toBe('"f" silinecek. 7 terminal kapanacak. Devam?')
  })

  it("says so when nothing would close", () => {
    expect(deletePrompt(folder("f", true, []), () => 0)).toBe(
      '"f" silinecek. Açık terminal yok. Devam?',
    )
  })
})

describe("newFolder", () => {
  it("creates an expanded empty folder with a unique id", () => {
    const a = newFolder()
    const b = newFolder()
    expect(a.kind).toBe("folder")
    expect(a.name).toBe("Yeni klasör")
    expect(a.expanded).toBe(true)
    expect(a.children).toEqual([])
    expect(a.id).not.toBe(b.id)
  })
})
