export interface PaneKey {
  key: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
}

/**
 * Ctrl+Shift+W — Windows Terminal's "close pane". The shell must never see it:
 * without Shift it would be Ctrl+W, which readline reads as "delete word", so the
 * Shift check is what keeps the two apart.
 */
export function isClosePaneChord(e: PaneKey): boolean {
  return isPaneChord(e, "w")
}

/** Ctrl+Shift+Z — tmux's `prefix z`: blow the focused pane up to fill the
 *  workspace, and the same chord puts it back. */
export function isMaximizePaneChord(e: PaneKey): boolean {
  return isPaneChord(e, "z")
}

/** Ctrl+Shift+M — send the focused pane down to the strip below the grid. */
export function isMinimizePaneChord(e: PaneKey): boolean {
  return isPaneChord(e, "m")
}

/** Ctrl+Tab — move the keyboard to the next terminal in the grid, wrapping at the
 *  end. Ctrl+Shift+Tab walks the other way. Tab (with no Ctrl) still reaches the
 *  shell as an ordinary tab, so completion in the running program is untouched. */
export function isFocusNextPaneChord(e: PaneKey): boolean {
  return isFocusNavChord(e) && !e.shiftKey
}

/** Ctrl+Shift+Tab — the reverse of {@link isFocusNextPaneChord}. */
export function isFocusPrevPaneChord(e: PaneKey): boolean {
  return isFocusNavChord(e) && e.shiftKey
}

function isFocusNavChord(e: PaneKey): boolean {
  if (e.altKey) return false
  if (!e.ctrlKey && !e.metaKey) return false
  return e.key === "Tab"
}

/** Shared shape of every pane chord: Ctrl (or Cmd) + Shift + a letter, never
 *  Alt. Shift is what keeps these off the shell's own bindings — plain Ctrl+W
 *  is readline's "delete word", Ctrl+Z is suspend. */
function isPaneChord(e: PaneKey, letter: string): boolean {
  if (e.altKey || !e.shiftKey) return false
  if (!e.ctrlKey && !e.metaKey) return false
  return e.key.toLowerCase() === letter
}
