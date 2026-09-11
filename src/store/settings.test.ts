import { describe, it, expect } from "vitest"
import { commandPreview, removeTool, toolUsage, upsertTool, upsertDir, removeDir } from "./settings"
import type { Config, Node, ShellInfo, Workspace } from "./types"

const pwsh: ShellInfo = { id: "pwsh", name: "PowerShell 7", program: "pwsh.exe", args: ["-NoLogo"] }

function ws(id: string, aiToolId: string | null): Workspace {
  return {
    id, kind: "workspace", name: id, path: "/x", aiToolId, shellId: null,
    rows: 1, cols: 1, rowSizes: [1], colSizes: [1],
  }
}

function config(tree: Node[]): Config {
  return {
    version: 1,
    aiTools: [{ id: "t1", name: "Claude Code", command: "claude" }],
    directories: [{ id: "d1", alias: "api", path: "/x/api" }],
    recentDirs: [],
    defaultShellId: "pwsh",
    tree,
  }
}

describe("commandPreview", () => {
  it("says the command is typed into the shell, never passed as an argument", () => {
    const text = commandPreview(pwsh, "claude --model sonnet-5", "tr")
    expect(text).toBe('pwsh.exe -NoLogo açılır, ardından "claude --model sonnet-5" yazılır')
    expect(text).not.toContain("-Command")
  })

  it("says no command runs when the command is blank", () => {
    expect(commandPreview(pwsh, "   ", "tr")).toBe("pwsh.exe -NoLogo açılır, komut çalıştırılmaz")
  })

  it("handles a missing shell", () => {
    expect(commandPreview(null, "claude", "tr")).toBe(
      '(shell seçilmedi) açılır, ardından "claude" yazılır',
    )
  })

  it("previews in the caller's locale", () => {
    expect(commandPreview(pwsh, "claude", "en")).toBe(
      'pwsh.exe -NoLogo opens, then "claude" is typed',
    )
  })
})

describe("toolUsage", () => {
  it("counts workspaces using the tool at any depth", () => {
    const tree: Node[] = [
      ws("a", "t1"),
      { id: "f", kind: "folder", name: "f", expanded: true, children: [ws("b", "t1"), ws("c", null)] },
    ]
    expect(toolUsage(tree, "t1")).toBe(2)
    expect(toolUsage(tree, "t9")).toBe(0)
  })
})

describe("removeTool", () => {
  it("drops the tool and clears it from every workspace", () => {
    const before = config([ws("a", "t1"), ws("b", null)])
    const after = removeTool(before, "t1")
    expect(after.aiTools).toEqual([])
    expect((after.tree[0] as Workspace).aiToolId).toBeNull()
    expect((after.tree[1] as Workspace).aiToolId).toBeNull()
  })

  it("does not mutate the original config", () => {
    const before = config([ws("a", "t1")])
    removeTool(before, "t1")
    expect(before.aiTools.length).toBe(1)
    expect((before.tree[0] as Workspace).aiToolId).toBe("t1")
  })
})

describe("upsertTool", () => {
  it("replaces a tool with the same id", () => {
    const after = upsertTool(config([]), { id: "t1", name: "Claude", command: "claude -c" })
    expect(after.aiTools).toEqual([{ id: "t1", name: "Claude", command: "claude -c" }])
  })

  it("appends a new tool", () => {
    const after = upsertTool(config([]), { id: "t2", name: "Gemini", command: "gemini" })
    expect(after.aiTools.map((t) => t.id)).toEqual(["t1", "t2"])
  })
})

describe("directory aliases", () => {
  it("upserts by id", () => {
    const after = upsertDir(config([]), { id: "d1", alias: "api-v2", path: "/x/api" })
    expect(after.directories).toEqual([{ id: "d1", alias: "api-v2", path: "/x/api" }])
  })

  it("removes by id and leaves workspaces alone", () => {
    const before = config([ws("a", null)])
    const after = removeDir(before, "d1")
    expect(after.directories).toEqual([])
    expect(after.tree.length).toBe(1)
  })
})
