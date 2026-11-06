/**
 * Kitty keyboard protocol (progressive enhancement), the part that matters:
 * flag 1, "disambiguate escape codes".
 *
 * In the legacy encoding Esc is a lone 0x1b — the same byte every escape
 * sequence starts with — so an application has to guess, with a timeout,
 * whether a bare ESC is the key or the start of something longer. TUIs such as
 * Claude Code ask for this protocol (`CSI ? u`, then `CSI > flags u`) and, when
 * the terminal answers, get Esc as the unambiguous `CSI 27 u`. Windows Terminal
 * answers since 1.25; xterm.js 6 does not, so miniterm does it here.
 *
 * Only flag 1 is implemented. A query reports the flags actually in effect, as
 * the spec requires, so an application asking for more (Claude Code pushes 5)
 * learns that the others are off.
 *
 * Spec: https://sw.kovidgoyal.net/kitty/keyboard-protocol/
 */

import type { IDisposable, Terminal } from "@xterm/xterm"

export const KITTY_DISAMBIGUATE = 0b1
export const KITTY_SUPPORTED = KITTY_DISAMBIGUATE

/** Bound on each screen's stack; the oldest entry goes first, as in kitty. */
const STACK_LIMIT = 16

export type Screen = "normal" | "alternate"

/**
 * The per-screen flag stacks. The main and alternate screens keep separate
 * stacks, so a full-screen app that pushes flags and then leaves the alternate
 * screen without popping them cannot leak them into the shell.
 */
export class KittyKeyboardState {
  private stacks: Record<Screen, number[]> = { normal: [], alternate: [] }

  flags(screen: Screen): number {
    const s = this.stacks[screen]
    return s.length > 0 ? s[s.length - 1] : 0
  }

  push(flags: number, screen: Screen) {
    const s = this.stacks[screen]
    s.push(flags & KITTY_SUPPORTED)
    if (s.length > STACK_LIMIT) s.shift()
  }

  pop(count: number, screen: Screen) {
    const s = this.stacks[screen]
    s.splice(Math.max(0, s.length - Math.max(1, count)))
  }

  /** `CSI = flags ; mode u`: 1 replaces, 2 sets bits, 3 clears bits. */
  set(flags: number, mode: number, screen: Screen) {
    const s = this.stacks[screen]
    const cur = this.flags(screen)
    const want = flags & KITTY_SUPPORTED
    const next = mode === 2 ? cur | want : mode === 3 ? cur & ~want : want
    if (s.length === 0) s.push(next)
    else s[s.length - 1] = next
  }

  reset() {
    this.stacks = { normal: [], alternate: [] }
  }
}

export type KittyKey = {
  key: string
  code: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
  /** True for AltGr on layouts that have it (Turkish Q: AltGr+Q is `@`). */
  altGraph?: boolean
}

function modifierValue(e: KittyKey): number {
  return 1 + (e.shiftKey ? 1 : 0) + (e.altKey ? 2 : 0) + (e.ctrlKey ? 4 : 0) + (e.metaKey ? 8 : 0)
}

function csiU(code: number, mods: number): string {
  return mods === 1 ? `\x1b[${code}u` : `\x1b[${code};${mods}u`
}

/** The unshifted key's code point for a printable key, or null when it cannot
 *  be told reliably (Shift on a non-letter, non-digit key: the base character
 *  depends on the layout and the browser does not expose it). */
function textKeyCode(e: KittyKey): number | null {
  const digit = /^Digit([0-9])$/.exec(e.code)
  if (digit) return digit[1].charCodeAt(0)
  if ([...e.key].length !== 1) return null
  const lower = e.key.toLowerCase()
  if (e.shiftKey && lower === e.key) return null
  if ([...lower].length !== 1) return null
  return lower.codePointAt(0)!
}

/**
 * The bytes for a key press under the given flags, or null to let xterm
 * encode it the legacy way. Functional keys (arrows, Home, F1…) keep their
 * legacy form, which flag 1 leaves unchanged; so do Enter, Tab and Backspace
 * without modifiers, so a shell is still usable after a program crashes with
 * the flag set.
 */
export function kittyKeySequence(e: KittyKey, flags: number): string | null {
  if ((flags & KITTY_DISAMBIGUATE) === 0) return null
  // AltGr arrives as Ctrl+Alt on Windows and produces text; it is typing, not a chord.
  if (e.altGraph) return null
  const mods = modifierValue(e)
  const chord = e.ctrlKey || e.altKey || e.metaKey

  switch (e.code) {
    case "Escape":
      return csiU(27, mods)
    case "Enter":
    case "NumpadEnter":
      return mods === 1 ? null : csiU(13, mods)
    case "Tab":
      return mods === 1 ? null : csiU(9, mods)
    case "Backspace":
      return mods === 1 ? null : csiU(127, mods)
    case "Space":
      return chord ? csiU(32, mods) : null
  }

  // Plain and Shift-only text is still sent as text.
  if (!chord) return null
  // Windows treats Ctrl+Alt as AltGr, so Ctrl+Alt+Q can type `@` without the
  // AltGr state set. Encode it as a chord only when the key still produces its
  // own letter or digit; anything else is text from the third level.
  if (e.ctrlKey && e.altKey) {
    const m = /^(?:Key|Digit)([A-Z0-9])$/.exec(e.code)
    if (!m || e.key.toUpperCase() !== m[1]) return null
  }
  const code = textKeyCode(e)
  return code === null ? null : csiU(code, mods)
}

function param(params: (number | number[])[], i: number): number {
  const p = params[i]
  return typeof p === "number" ? p : (p?.[0] ?? 0)
}

/**
 * Teach an xterm.js terminal the protocol's control sequences. Replies go
 * through `reply` (the pane writes them to the PTY like any terminal report);
 * `encode` turns a keydown into the bytes to send, or null for legacy.
 */
export function attachKittyKeyboard(
  term: Terminal,
  reply: (data: string) => void,
): IDisposable & { state: KittyKeyboardState; encode: (e: KittyKey) => string | null } {
  const state = new KittyKeyboardState()
  const screen = (): Screen => term.buffer.active.type
  const hooks = [
    term.parser.registerCsiHandler({ prefix: "?", final: "u" }, () => {
      reply(`\x1b[?${state.flags(screen())}u`)
      return true
    }),
    term.parser.registerCsiHandler({ prefix: ">", final: "u" }, (p) => {
      state.push(param(p, 0), screen())
      return true
    }),
    term.parser.registerCsiHandler({ prefix: "<", final: "u" }, (p) => {
      state.pop(param(p, 0), screen())
      return true
    }),
    term.parser.registerCsiHandler({ prefix: "=", final: "u" }, (p) => {
      state.set(param(p, 0), param(p, 1) || 1, screen())
      return true
    }),
    // RIS: a full reset drops the enhancements along with everything else.
    // Returning false lets xterm run its own reset as well.
    term.parser.registerEscHandler({ final: "c" }, () => {
      state.reset()
      return false
    }),
  ]
  return {
    state,
    encode: (e) => kittyKeySequence(e, state.flags(screen())),
    dispose: () => hooks.forEach((h) => h.dispose()),
  }
}
