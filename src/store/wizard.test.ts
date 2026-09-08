import { describe, it, expect } from "vitest"
import { basename, emptyDraft, toWorkspace } from "./wizard"
import { pushRecent, RECENT_LIMIT } from "./settings"

describe("basename", () => {
  it("takes the last segment of a Windows path", () => {
    expect(basename("C:\\Development\\Cursor Apps\\miniterm")).toBe("miniterm")
  })

  it("takes the last segment of a POSIX path", () => {
    expect(basename("/home/kadir/projects/api")).toBe("api")
  })

  it("ignores a trailing separator", () => {
    expect(basename("D:/work/spock/")).toBe("spock")
  })

  it("falls back to the whole string for a drive root", () => {
    expect(basename("C:\\")).toBe("C:")
  })

  it("returns an empty string for an empty path", () => {
    expect(basename("")).toBe("")
  })
})

describe("emptyDraft", () => {
  it("starts blank", () => {
    expect(emptyDraft().path).toBe("")
    expect(emptyDraft().name).toBe("")
  })
})

describe("toWorkspace", () => {
  it("builds a workspace node with equal track sizes", () => {
    const ws = toWorkspace({
      path: "/x/api", name: "api", aiToolId: "t1", shellId: null,
      count: 6, layout: { rows: 2, cols: 3 },
    })
    expect(ws.kind).toBe("workspace")
    expect(ws.name).toBe("api")
    expect(ws.path).toBe("/x/api")
    expect(ws.aiToolId).toBe("t1")
    expect(ws.shellId).toBeNull()
    expect(ws.rows).toBe(2)
    expect(ws.cols).toBe(3)
    expect(ws.rowSizes).toEqual([0.5, 0.5])
    expect(ws.colSizes.length).toBe(3)
    expect(ws.colSizes.reduce((a, b) => a + b, 0)).toBeCloseTo(1)
    expect(ws.id).toMatch(/[0-9a-f-]{36}/)
  })

  it("gives every workspace a distinct id", () => {
    const d = { ...emptyDraft(), path: "/x/api", name: "api" }
    expect(toWorkspace(d).id).not.toBe(toWorkspace(d).id)
  })
})

describe("pushRecent", () => {
  it("puts the newest path first", () => {
    expect(pushRecent(["/b"], "/a")).toEqual(["/a", "/b"])
  })

  it("moves an existing path to the front instead of duplicating it", () => {
    expect(pushRecent(["/a", "/b", "/c"], "/c")).toEqual(["/c", "/a", "/b"])
  })

  it("keeps at most RECENT_LIMIT entries", () => {
    const many = Array.from({ length: RECENT_LIMIT }, (_, i) => `/p${i}`)
    const out = pushRecent(many, "/new")
    expect(out.length).toBe(RECENT_LIMIT)
    expect(out[0]).toBe("/new")
    expect(out).not.toContain(`/p${RECENT_LIMIT - 1}`)
  })
})
