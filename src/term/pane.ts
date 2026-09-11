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
  if (e.altKey || !e.shiftKey) return false
  if (!e.ctrlKey && !e.metaKey) return false
  return e.key.toLowerCase() === "w"
}
