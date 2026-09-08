import { describe, it, expect } from "vitest"
import { zoneFor, targetFor } from "./dnd"

describe("zoneFor", () => {
  it("returns before in the top quarter", () => {
    expect(zoneFor(0, 26, true)).toBe("before")
    expect(zoneFor(5, 26, false)).toBe("before")
  })

  it("returns after in the bottom quarter", () => {
    expect(zoneFor(25, 26, true)).toBe("after")
    expect(zoneFor(21, 26, false)).toBe("after")
  })

  it("returns into for the middle of a folder", () => {
    expect(zoneFor(13, 26, true)).toBe("into")
  })

  it("returns after for the middle of a workspace", () => {
    expect(zoneFor(13, 26, false)).toBe("after")
  })

  it("clamps values outside the row", () => {
    expect(zoneFor(-40, 26, true)).toBe("before")
    expect(zoneFor(999, 26, true)).toBe("after")
  })

  it("detects boundary at top edge (6.5 with height 26)", () => {
    // 26 * 0.25 = 6.5, so y < 6.5 → "before", y >= 6.5 → middle zone
    expect(zoneFor(6, 26, true)).toBe("before")
    expect(zoneFor(7, 26, true)).toBe("into")
  })

  it("detects boundary at bottom edge (19.5 with height 26)", () => {
    // 26 * 0.75 = 19.5, so y <= 19.5 → middle zone, y > 19.5 → "after"
    expect(zoneFor(19, 26, true)).toBe("into")
    expect(zoneFor(20, 26, true)).toBe("after")
  })

  it("detects boundary behavior for workspace at middle (middle → after for non-folder)", () => {
    // Workspace (isFolder=false) in middle zone should return "after"
    expect(zoneFor(6, 26, false)).toBe("before")
    expect(zoneFor(7, 26, false)).toBe("after")
    expect(zoneFor(19, 26, false)).toBe("after")
    expect(zoneFor(20, 26, false)).toBe("after")
  })
})

describe("targetFor", () => {
  it("maps zones onto DropTarget shapes", () => {
    expect(targetFor("before", "n1")).toEqual({ type: "before", siblingId: "n1" })
    expect(targetFor("after", "n1")).toEqual({ type: "after", siblingId: "n1" })
    expect(targetFor("into", "n1")).toEqual({ type: "into", folderId: "n1" })
  })
})
