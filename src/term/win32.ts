/**
 * ConPTY win32-input-mode.
 *
 * A plain VT stream cannot express most modified keys: Ctrl+Enter, Shift+Enter
 * and Enter all collapse into a single CR, so PSReadLine sees one key and the
 * shell cannot insert a continuation line. Windows Terminal solves this by
 * turning on ConPTY's win32-input-mode (DECSET 9001) and then handing ConPTY
 * whole key events — virtual key, unicode char and modifier state — which
 * conhost turns back into real INPUT_RECORDs.
 *
 * Mode 9001 is additive: ordinary VT input keeps working, so only the chords
 * that would otherwise lose their modifier are encoded here.
 *
 * Spec: microsoft/terminal doc/specs/#4999 - Improved keyboard handling in ConPTY
 */

// The mode itself is enabled Rust-side, once per session, right after the PTY is
// created (src-tauri/src/pty/mod.rs). It used to be written from the frontend on
// every attach, which echoed a literal `[?9001h` into whatever TUI was running:
// by attach time the application owns stdin in VT-input mode, so conhost forwards
// the sequence instead of consuming it.

// ControlKeyState bits (wincon.h). Chromium cannot tell left from right for a
// chord's modifiers, so the left variants stand in — conhost treats them alike.
const LEFT_CTRL_PRESSED = 0x0008;
const SHIFT_PRESSED = 0x0010;
const ENHANCED_KEY = 0x0100;

const VK_BACK = 0x08;
const VK_RETURN = 0x0d;

export type Win32Key = {
  key: string;
  code: string;
  ctrlKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
  metaKey: boolean;
};

/** Virtual key + the char Windows would produce for it, or null when xterm's
 *  own VT encoding already carries the modifier through. */
function keySpec(e: Win32Key): { vk: number; uc: number } | null {
  if (e.code === "Enter" || e.code === "NumpadEnter") {
    if (!e.ctrlKey && !e.shiftKey) return null;
    return { vk: VK_RETURN, uc: e.ctrlKey ? 0x0a : 0x0d };
  }
  if (e.code === "Backspace") {
    // Ctrl+Backspace alone already arrives as 0x08 and Backspace as 0x7f;
    // only Shift is the one that gets dropped.
    if (!e.shiftKey) return null;
    return { vk: VK_BACK, uc: e.ctrlKey ? 0x7f : 0x08 };
  }
  if (!e.ctrlKey || !e.shiftKey) return null;
  // Ctrl+Shift+X is byte-identical to Ctrl+X in VT. `code` is used instead of
  // `key` because Shift rewrites `key` ("A", "!") while the virtual key does
  // not change.
  const letter = /^Key([A-Z])$/.exec(e.code);
  if (letter) {
    const vk = letter[1].charCodeAt(0);
    return { vk, uc: vk - 64 };
  }
  const digit = /^Digit([0-9])$/.exec(e.code);
  if (digit) return { vk: digit[1].charCodeAt(0), uc: 0 };
  return null;
}

function record(vk: number, uc: number, down: 0 | 1, state: number): string {
  return `\x1b[${vk};0;${uc};${down};${state};1_`;
}

/** The bytes to send for a chord ConPTY would otherwise flatten, or null to let
 *  xterm encode the key itself. Both the press and the release are emitted,
 *  matching what a real keyboard delivers to the console. */
export function win32KeySequence(e: Win32Key): string | null {
  if (e.altKey || e.metaKey) return null;
  const spec = keySpec(e);
  if (!spec) return null;
  const state =
    (e.ctrlKey ? LEFT_CTRL_PRESSED : 0) |
    (e.shiftKey ? SHIFT_PRESSED : 0) |
    (e.code === "NumpadEnter" ? ENHANCED_KEY : 0);
  return record(spec.vk, spec.uc, 1, state) + record(spec.vk, spec.uc, 0, state);
}
