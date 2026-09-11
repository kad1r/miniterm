import { describe, it, expect } from "vitest"
import { clipboardAction, type ClipboardKey } from "./clipboard"

function key(partial: Partial<ClipboardKey> & { key: string }): ClipboardKey {
  return { ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...partial }
}

describe("clipboardAction", () => {
  it("pastes on Ctrl+V, Ctrl+Shift+V and Shift+Insert", () => {
    expect(clipboardAction(key({ key: "v", ctrlKey: true }), false)).toBe("paste")
    expect(clipboardAction(key({ key: "V", ctrlKey: true, shiftKey: true }), false)).toBe("paste")
    expect(clipboardAction(key({ key: "Insert", shiftKey: true }), false)).toBe("paste")
  })

  it("copies on Ctrl+C only while something is selected", () => {
    expect(clipboardAction(key({ key: "c", ctrlKey: true }), true)).toBe("copy")
    expect(clipboardAction(key({ key: "Insert", ctrlKey: true }), true)).toBe("copy")
  })

  it("leaves Ctrl+C alone without a selection so it stays SIGINT", () => {
    expect(clipboardAction(key({ key: "c", ctrlKey: true }), false)).toBe(null)
    expect(clipboardAction(key({ key: "Insert", ctrlKey: true }), false)).toBe(null)
  })

  it("ignores the same keys without a modifier", () => {
    expect(clipboardAction(key({ key: "v" }), false)).toBe(null)
    expect(clipboardAction(key({ key: "c" }), true)).toBe(null)
    expect(clipboardAction(key({ key: "Insert" }), true)).toBe(null)
  })

  it("ignores Alt combinations, which the shell owns", () => {
    expect(clipboardAction(key({ key: "v", ctrlKey: true, altKey: true }), false)).toBe(null)
    expect(clipboardAction(key({ key: "c", ctrlKey: true, altKey: true }), true)).toBe(null)
  })

  it("ignores unrelated control keys", () => {
    expect(clipboardAction(key({ key: "a", ctrlKey: true }), true)).toBe(null)
    expect(clipboardAction(key({ key: "x", ctrlKey: true }), true)).toBe(null)
    expect(clipboardAction(key({ key: "d", ctrlKey: true }), false)).toBe(null)
  })
})
