import { describe, it, expect } from "vitest"
import {
  flatten,
  deletePrompt,
  newFolder,
  readCollapsed,
  writeCollapsed,
  COLLAPSE_KEY,
} from "./sidebar"
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
    expect(deletePrompt(ws("a", "API", 2, 3), () => 0, "tr")).toBe(
      '"API" silinecek. Açık terminal yok. Devam?',
    )
  })

  it("reports the live session count when the workspace is active", () => {
    // 2×3 workspace that has all 6 sessions running.
    expect(deletePrompt(ws("a", "API", 2, 3), () => 6, "tr")).toBe(
      '"API" silinecek. 6 terminal kapanacak. Devam?',
    )
  })

  it("sums live sessions under a folder", () => {
    const f = folder("f", true, [ws("a", "A", 2, 2), ws("b", "B", 1, 3)])
    // "a" has 4 live, "b" has 3 live.
    const live = (id: string) => id === "a" ? 4 : 3
    expect(deletePrompt(f, live, "tr")).toBe('"f" silinecek. 7 terminal kapanacak. Devam?')
  })

  it("says so when nothing would close", () => {
    expect(deletePrompt(folder("f", true, []), () => 0, "tr")).toBe(
      '"f" silinecek. Açık terminal yok. Devam?',
    )
  })

  it("prompts in the caller's locale", () => {
    expect(deletePrompt(ws("a", "API", 2, 3), () => 6, "en")).toBe(
      '"API" will be deleted. 6 terminals will close. Continue?',
    )
  })
})

describe("newFolder", () => {
  it("creates an expanded empty folder with a unique id", () => {
    const a = newFolder("Yeni klasör")
    const b = newFolder("Yeni klasör")
    expect(a.kind).toBe("folder")
    expect(a.name).toBe("Yeni klasör")
    expect(a.expanded).toBe(true)
    expect(a.children).toEqual([])
    expect(a.id).not.toBe(b.id)
  })
})

describe("collapse persistence", () => {
  /** Minimal in-memory stand-in for Storage; the real one is not in the node env. */
  function fakeStore(seed: Record<string, string> = {}) {
    const map = new Map(Object.entries(seed))
    return {
      getItem: (k: string) => map.get(k) ?? null,
      setItem: (k: string, v: string) => void map.set(k, v),
      read: (k: string) => map.get(k) ?? null,
    }
  }

  it("defaults to expanded when nothing was ever stored", () => {
    expect(readCollapsed(fakeStore())).toBe(false)
  })

  it("round-trips both states", () => {
    const store = fakeStore()
    writeCollapsed(store, true)
    expect(readCollapsed(store)).toBe(true)
    writeCollapsed(store, false)
    expect(readCollapsed(store)).toBe(false)
  })

  it("treats unparseable values as expanded rather than throwing", () => {
    // A hand-edited or half-written localStorage entry must not brick the UI.
    expect(readCollapsed(fakeStore({ [COLLAPSE_KEY]: "garbage" }))).toBe(false)
  })

  it("survives a storage that throws (private mode, quota exceeded)", () => {
    const throwing = {
      getItem: () => {
        throw new Error("denied")
      },
      setItem: () => {
        throw new Error("quota")
      },
    }
    expect(readCollapsed(throwing)).toBe(false)
    expect(() => writeCollapsed(throwing, true)).not.toThrow()
  })
})
