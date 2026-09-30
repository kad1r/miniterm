import { describe, it, expect } from "vitest"
import {
  AGENT_NAMES, GREEK_GODS, NORSE_GODS, clearPaneNames, cleanName, introText, logText, namesFor, renameAt,
  resolveNames, resolveTree, rosterMarkdown, sameNames, takenOutside,
} from "./agents"
import type { Node, Workspace } from "./types"

describe("cleanName", () => {
  it("keeps letters, digits, - and _", () => {
    expect(cleanName("  Çağrı_2 ")).toBe("Çağrı_2")
    expect(cleanName("back end")).toBe("back-end")
    expect(cleanName("../evil.md")).toBe("evilmd")
    expect(cleanName("   ")).toBe("")
  })

  it("caps the length", () => {
    expect(cleanName("a".repeat(40))).toHaveLength(24)
  })
})

describe("resolveNames", () => {
  it("hands out Greek gods in order when nothing is stored", () => {
    expect(resolveNames([], 3)).toEqual(["Zeus", "Hera", "Athena"])
  })

  it("keeps stored names and fills the gaps without collisions", () => {
    expect(resolveNames(["Hera", null, "Mert"], 3)).toEqual(["Hera", "Zeus", "Mert"])
  })

  it("drops duplicates (case-insensitive) and trims to the grid", () => {
    expect(resolveNames(["Kim", "kim", "X", "Y"], 2)).toEqual(["Kim", "Zeus"])
  })

  it("skips names taken elsewhere", () => {
    expect(resolveNames(["Zeus"], 2, new Set(["zeus", "hera"]))).toEqual(["Athena", "Apollo"])
  })

  it("moves on to Norse gods, then numbered rounds, never repeating", () => {
    const names = resolveNames([], AGENT_NAMES.length + 2)
    expect(names[GREEK_GODS.length]).toBe(NORSE_GODS[0])
    expect(names.at(-2)).toBe("Zeus2")
    expect(new Set(names).size).toBe(names.length)
  })
})

describe("resolveTree", () => {
  const ws = (id: string, paneNames: (string | null)[] = [], cols = 2): Workspace => ({
    id, kind: "workspace", name: id, path: "", aiToolId: null, shellId: null,
    rows: 1, cols, rowSizes: [1], colSizes: Array(cols).fill(1 / cols), paneTools: [], paneNames,
  })

  it("never gives two workspaces the same name", () => {
    const tree: Node[] = [ws("a"), { id: "f", kind: "folder", name: "f", expanded: true, children: [ws("b")] }]
    const all = [...resolveTree(tree).values()].flat()
    expect(all).toEqual(["Zeus", "Hera", "Athena", "Apollo"])
  })

  it("stored names beat defaults, whatever the tree order", () => {
    const tree: Node[] = [ws("a"), ws("b", ["Zeus", "Hera"])]
    const r = resolveTree(tree)
    expect(r.get("b")).toEqual(["Zeus", "Hera"])
    expect(r.get("a")).toEqual(["Athena", "Apollo"])
  })

  it("on a stored clash the earlier workspace keeps the name", () => {
    const tree: Node[] = [ws("a", ["Odin", null]), ws("b", ["odin", "Thor"])]
    const r = resolveTree(tree)
    expect(r.get("a")).toEqual(["Odin", "Zeus"])
    expect(r.get("b")).toEqual(["Hera", "Thor"])
  })

  it("namesFor and takenOutside agree with the tree", () => {
    const a = ws("a", ["Loki", "Freya"])
    const tree: Node[] = [a, ws("b")]
    expect(namesFor(a, tree)).toEqual(["Loki", "Freya"])
    expect([...takenOutside(tree, "a")]).toEqual(["zeus", "hera"])
  })
})

describe("renameAt", () => {
  it("renames one cell", () => {
    expect(renameAt(["Zeus", "Hera"], 1, "Mert")).toEqual({ names: ["Zeus", "Mert"] })
  })

  it("rejects empty names and names taken here or in another workspace", () => {
    expect(renameAt(["Zeus", "Hera"], 1, "  ")).toEqual({ error: "empty" })
    expect(renameAt(["Zeus", "Hera"], 1, "zeus")).toEqual({ error: "taken" })
    expect(renameAt(["Zeus", "Hera"], 1, "Odin", new Set(["odin"]))).toEqual({ error: "taken" })
  })

  it("allows re-casing your own name", () => {
    expect(renameAt(["Zeus"], 0, "ZEUS")).toEqual({ names: ["ZEUS"] })
  })
})

describe("clearPaneNames", () => {
  it("drops stored names everywhere and reports which workspaces had them", () => {
    const base = {
      kind: "workspace" as const, path: "", aiToolId: null, shellId: null,
      rows: 1, cols: 1, rowSizes: [1], colSizes: [1], paneTools: [],
    }
    const tree: Node[] = [
      { ...base, id: "a", name: "a", paneNames: ["Atlas"] },
      { id: "f", kind: "folder", name: "f", expanded: true, children: [{ ...base, id: "b", name: "b" }] },
    ]
    const r = clearPaneNames(tree)
    expect(r.cleared).toEqual(["a"])
    expect((r.tree[0] as Workspace).paneNames).toEqual([])
    expect(r.tree[1]).toEqual(tree[1])
  })
})

describe("sameNames", () => {
  it("compares stored against resolved", () => {
    expect(sameNames(undefined, ["A"])).toBe(false)
    expect(sameNames(["A", null], ["A", "B"])).toBe(false)
    expect(sameNames(["A", "B"], ["A", "B"])).toBe(true)
  })
})

describe("files", () => {
  const dir = "C:\\cfg\\agents\\ws1"

  it("roster lists every agent with its files", () => {
    const md = rosterMarkdown("linepulse", dir, [
      { name: "Atlas", tool: "Claude", status: "running" },
      { name: "Nova", tool: "shell", status: "exited" },
    ])
    expect(md).toContain('"linepulse"')
    expect(md).toContain("| Atlas | Claude | running | C:\\cfg\\agents\\ws1\\Atlas.log | C:\\cfg\\agents\\ws1\\inbox\\Atlas.md |")
    expect(md).toContain("| Nova | shell | exited |")
  })

  it("intro is a single line naming the agent and its team", () => {
    const tr = introText("tr", "Atlas", "linepulse", dir, ["Nova", "Orion"])
    expect(tr).not.toContain("\n")
    expect(tr).toContain("Senin adın Atlas")
    expect(tr).toContain("Nova, Orion")
    expect(introText("en", "Atlas", "w", dir, [])).toContain("Your name is Atlas")
  })

  it("log trims trailing blank lines and spaces", () => {
    const text = logText("Atlas", ["hello   ", "world", "", "  "], new Date(0))
    expect(text).toBe("# Atlas — terminal output, updated 1970-01-01T00:00:00.000Z\n\nhello\nworld\n")
  })
})
