import { describe, it, expect } from "vitest"
import { win32KeySequence, type Win32Key } from "./win32"

function key(partial: Partial<Win32Key> & { code: string }): Win32Key {
  return { key: "", ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...partial }
}

describe("win32KeySequence", () => {
  it("leaves unmodified keys to xterm's own VT encoding", () => {
    expect(win32KeySequence(key({ code: "Enter" }))).toBeNull()
    expect(win32KeySequence(key({ code: "Backspace" }))).toBeNull()
    expect(win32KeySequence(key({ code: "KeyA", ctrlKey: true }))).toBeNull()
    expect(win32KeySequence(key({ code: "KeyA" }))).toBeNull()
  })

  it("encodes Ctrl+Enter as a key press and release with the Ctrl bit set", () => {
    expect(win32KeySequence(key({ code: "Enter", ctrlKey: true }))).toBe(
      "\x1b[13;0;10;1;8;1_\x1b[13;0;10;0;8;1_",
    )
  })

  it("encodes Shift+Enter with CR, since only Ctrl turns Enter into LF", () => {
    expect(win32KeySequence(key({ code: "Enter", shiftKey: true }))).toBe(
      "\x1b[13;0;13;1;16;1_\x1b[13;0;13;0;16;1_",
    )
  })

  it("marks the numpad Enter as an enhanced key", () => {
    expect(win32KeySequence(key({ code: "NumpadEnter", ctrlKey: true }))).toBe(
      "\x1b[13;0;10;1;264;1_\x1b[13;0;10;0;264;1_",
    )
  })

  it("encodes Shift+Backspace, which VT drops entirely", () => {
    expect(win32KeySequence(key({ code: "Backspace", shiftKey: true }))).toBe(
      "\x1b[8;0;8;1;16;1_\x1b[8;0;8;0;16;1_",
    )
  })

  it("separates Ctrl+Shift+letter from Ctrl+letter", () => {
    expect(win32KeySequence(key({ code: "KeyK", ctrlKey: true, shiftKey: true }))).toBe(
      "\x1b[75;0;11;1;24;1_\x1b[75;0;11;0;24;1_",
    )
  })

  it("uses the physical key for digits, whose `key` Shift rewrites", () => {
    expect(win32KeySequence(key({ code: "Digit1", key: "!", ctrlKey: true, shiftKey: true }))).toBe(
      "\x1b[49;0;0;1;24;1_\x1b[49;0;0;0;24;1_",
    )
  })

  it("stays out of the way of Alt and Win chords, which belong to the OS", () => {
    expect(win32KeySequence(key({ code: "Enter", ctrlKey: true, altKey: true }))).toBeNull()
    expect(win32KeySequence(key({ code: "Enter", shiftKey: true, metaKey: true }))).toBeNull()
  })
})
