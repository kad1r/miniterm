/**
 * App-level shortcuts: the ones handled by miniterm's own chrome rather than by
 * the program running inside a terminal. They all use Ctrl/Cmd+Shift (never a
 * bare Ctrl+letter) or a function key, so the shell never loses a binding of its
 * own — Ctrl+C, Esc, the arrows and the rest still pass straight through.
 *
 * Kept in one place so the two callers stay in step: App.svelte dispatches them
 * at the window, and TerminalPane hands the same chords back to the window
 * instead of forwarding them to the shell (see isAppShortcut).
 */
export interface Chord {
  key: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
}

const ctrlOrMeta = (e: Chord) => e.ctrlKey || e.metaKey

/** Ctrl/Cmd+Shift+N — open the new-workspace wizard. */
export function isNewWorkspaceChord(e: Chord): boolean {
  return ctrlOrMeta(e) && e.shiftKey && !e.altKey && e.key.toLowerCase() === "n"
}

/** Ctrl/Cmd+Shift+T — add a terminal to the active workspace. */
export function isNewTerminalChord(e: Chord): boolean {
  return ctrlOrMeta(e) && e.shiftKey && !e.altKey && e.key.toLowerCase() === "t"
}

/** Ctrl/Cmd+, — toggle the settings view. Matches VS Code and the platform
 *  convention. No Shift: the comma itself carries the meaning. */
export function isSettingsChord(e: Chord): boolean {
  return ctrlOrMeta(e) && !e.shiftKey && !e.altKey && e.key === ","
}

/** F1 — toggle the shortcut list. The cheat-sheet's `?` cannot be borrowed: it
 *  is a printable character the shell needs, so a function key stands in. */
export function isHelpChord(e: Chord): boolean {
  return !e.ctrlKey && !e.metaKey && !e.altKey && !e.shiftKey && e.key === "F1"
}

/** True for any chord the window owns, so TerminalPane can let it escape xterm. */
export function isAppShortcut(e: Chord): boolean {
  return (
    isNewWorkspaceChord(e) || isNewTerminalChord(e) || isSettingsChord(e) || isHelpChord(e)
  )
}
