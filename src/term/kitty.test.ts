// @vitest-environment jsdom
import { describe, it, expect } from "vitest"
import { Terminal } from "@xterm/xterm"
import {
  attachKittyKeyboard, kittyKeySequence, KittyKeyboardState, KITTY_DISAMBIGUATE,
  type KittyKey,
} from "./kitty"

function key(partial: Partial<KittyKey> & { code: string }): KittyKey {
  return { key: "", ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...partial }
}

const ON = KITTY_DISAMBIGUATE

describe("kittyKeySequence", () => {
  it("leaves every key to xterm while the protocol is off", () => {
    expect(kittyKeySequence(key({ code: "Escape", key: "Escape" }), 0)).toBeNull()
    expect(kittyKeySequence(key({ code: "KeyC", key: "c", ctrlKey: true }), 0)).toBeNull()
  })

  it("sends Esc as CSI 27 u, the whole point of the protocol", () => {
    expect(kittyKeySequence(key({ code: "Escape", key: "Escape" }), ON)).toBe("\x1b[27u")
    expect(kittyKeySequence(key({ code: "Escape", key: "Escape", shiftKey: true }), ON)).toBe("\x1b[27;2u")
  })

  it("keeps unmodified Enter, Tab and Backspace legacy so a shell survives a crashed app", () => {
    for (const code of ["Enter", "Tab", "Backspace"]) {
      expect(kittyKeySequence(key({ code, key: code }), ON)).toBeNull()
    }
  })

  it("encodes modified Enter, Tab and Backspace", () => {
    expect(kittyKeySequence(key({ code: "Enter", key: "Enter", shiftKey: true }), ON)).toBe("\x1b[13;2u")
    expect(kittyKeySequence(key({ code: "Enter", key: "Enter", ctrlKey: true }), ON)).toBe("\x1b[13;5u")
    expect(kittyKeySequence(key({ code: "Tab", key: "Tab", shiftKey: true }), ON)).toBe("\x1b[9;2u")
    expect(kittyKeySequence(key({ code: "Backspace", key: "Backspace", altKey: true }), ON)).toBe("\x1b[127;3u")
  })

  it("leaves plain and Shift-only text alone", () => {
    expect(kittyKeySequence(key({ code: "KeyA", key: "a" }), ON)).toBeNull()
    expect(kittyKeySequence(key({ code: "KeyA", key: "A", shiftKey: true }), ON)).toBeNull()
    expect(kittyKeySequence(key({ code: "Space", key: " " }), ON)).toBeNull()
    expect(kittyKeySequence(key({ code: "Space", key: " ", shiftKey: true }), ON)).toBeNull()
  })

  it("encodes Ctrl and Alt chords with the unshifted key", () => {
    expect(kittyKeySequence(key({ code: "KeyC", key: "c", ctrlKey: true }), ON)).toBe("\x1b[99;5u")
    expect(kittyKeySequence(key({ code: "KeyB", key: "b", altKey: true }), ON)).toBe("\x1b[98;3u")
    expect(kittyKeySequence(key({ code: "KeyA", key: "A", ctrlKey: true, shiftKey: true }), ON)).toBe("\x1b[97;6u")
    expect(kittyKeySequence(key({ code: "Digit1", key: "!", ctrlKey: true, shiftKey: true }), ON)).toBe("\x1b[49;6u")
    expect(kittyKeySequence(key({ code: "Space", key: " ", ctrlKey: true }), ON)).toBe("\x1b[32;5u")
    expect(kittyKeySequence(key({ code: "BracketLeft", key: "[", ctrlKey: true }), ON)).toBe("\x1b[91;5u")
  })

  it("encodes non-ASCII letters by code point", () => {
    expect(kittyKeySequence(key({ code: "Semicolon", key: "ş", altKey: true }), ON)).toBe("\x1b[351;3u")
  })

  it("does not guess the base key of a shifted symbol", () => {
    expect(kittyKeySequence(key({ code: "Slash", key: "?", altKey: true, shiftKey: true }), ON)).toBeNull()
  })

  it("treats AltGr as typing, not a chord", () => {
    expect(
      kittyKeySequence(key({ code: "KeyQ", key: "@", ctrlKey: true, altKey: true, altGraph: true }), ON),
    ).toBeNull()
    // Ctrl+Alt is AltGr on Windows even without the AltGr state.
    expect(kittyKeySequence(key({ code: "KeyQ", key: "@", ctrlKey: true, altKey: true }), ON)).toBeNull()
    expect(kittyKeySequence(key({ code: "KeyQ", key: "q", ctrlKey: true, altKey: true }), ON)).toBe("\x1b[113;7u")
  })

  it("leaves functional keys to xterm's legacy encoding", () => {
    expect(kittyKeySequence(key({ code: "ArrowUp", key: "ArrowUp", ctrlKey: true }), ON)).toBeNull()
    expect(kittyKeySequence(key({ code: "F5", key: "F5" }), ON)).toBeNull()
  })
})

describe("KittyKeyboardState", () => {
  it("pushes, pops and masks unsupported flags", () => {
    const s = new KittyKeyboardState()
    expect(s.flags("normal")).toBe(0)
    s.push(5, "normal")
    expect(s.flags("normal")).toBe(1)
    s.push(0, "normal")
    expect(s.flags("normal")).toBe(0)
    s.pop(1, "normal")
    expect(s.flags("normal")).toBe(1)
    s.pop(10, "normal")
    expect(s.flags("normal")).toBe(0)
  })

  it("keeps separate stacks for the two screens", () => {
    const s = new KittyKeyboardState()
    s.push(1, "alternate")
    expect(s.flags("alternate")).toBe(1)
    expect(s.flags("normal")).toBe(0)
  })

  it("sets, adds and clears bits on the current entry", () => {
    const s = new KittyKeyboardState()
    s.set(1, 1, "normal")
    expect(s.flags("normal")).toBe(1)
    s.set(1, 3, "normal")
    expect(s.flags("normal")).toBe(0)
    s.set(1, 2, "normal")
    expect(s.flags("normal")).toBe(1)
  })

  it("caps the stack", () => {
    const s = new KittyKeyboardState()
    for (let i = 0; i < 40; i++) s.push(1, "normal")
    s.pop(16, "normal")
    expect(s.flags("normal")).toBe(0)
  })
})

describe("attachKittyKeyboard", () => {
  function setup() {
    const term = new Terminal({ allowProposedApi: true })
    const replies: string[] = []
    const kitty = attachKittyKeyboard(term, (d) => replies.push(d))
    const write = (s: string) => new Promise<void>((r) => term.write(s, r))
    return { term, kitty, replies, write }
  }

  it("answers the query with the flags in effect", async () => {
    const { kitty, replies, write } = setup()
    await write("\x1b[?u")
    expect(replies).toEqual(["\x1b[?0u"])
    await write("\x1b[>5u\x1b[?u")
    expect(replies[1]).toBe("\x1b[?1u")
    expect(kitty.encode(key({ code: "Escape", key: "Escape" }))).toBe("\x1b[27u")
  })

  it("follows pop, set and a full reset", async () => {
    const { kitty, write } = setup()
    const esc = () => kitty.encode(key({ code: "Escape", key: "Escape" }))
    await write("\x1b[>1u\x1b[<u")
    expect(esc()).toBeNull()
    await write("\x1b[=1;1u")
    expect(esc()).toBe("\x1b[27u")
    await write("\x1bc")
    expect(esc()).toBeNull()
  })

  it("does not take over CSI u, which is still restore-cursor", async () => {
    const { term, write } = setup()
    await write("ab\x1b[s\r\nxyz\x1b[u!")
    expect(term.buffer.active.getLine(0)?.translateToString(true)).toBe("ab!")
  })

  it("scopes the flags to the screen that set them", async () => {
    const { kitty, write } = setup()
    const esc = () => kitty.encode(key({ code: "Escape", key: "Escape" }))
    await write("\x1b[?1049h\x1b[>1u")
    expect(esc()).toBe("\x1b[27u")
    await write("\x1b[?1049l")
    expect(esc()).toBeNull()
  })
})
