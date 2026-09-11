export type ClipboardAction = "copy" | "paste"

export interface ClipboardKey {
  key: string
  ctrlKey: boolean
  shiftKey: boolean
  altKey: boolean
  metaKey: boolean
}

/**
 * Classify a keydown as a clipboard shortcut that the browser must service itself.
 *
 * xterm.js turns Ctrl+letter into a control code (Ctrl+V -> 0x16) and cancels the
 * event, so the WebView never performs its own copy/paste — which is why the
 * right-click menu pasted fine while Ctrl+V did nothing. The fix is to hand these
 * keys back to the WebView untouched: xterm already listens for the resulting
 * `copy` and `paste` DOM events on its textarea, so the native path does the work.
 *
 * Ctrl+C only counts as copy when there is a selection; with nothing selected it
 * must stay SIGINT.
 */
export function clipboardAction(e: ClipboardKey, hasSelection: boolean): ClipboardAction | null {
  if (e.altKey) return null
  const mod = e.ctrlKey || e.metaKey

  // Shift+Insert / Ctrl+Insert, the terminal-native spelling of paste and copy.
  if (e.key === "Insert") {
    if (e.shiftKey && !mod) return "paste"
    if (mod && !e.shiftKey) return hasSelection ? "copy" : null
    return null
  }

  if (!mod) return null
  const key = e.key.toLowerCase()
  // Chromium pastes on both Ctrl+V and Ctrl+Shift+V (the latter as plain text),
  // which is the same thing inside a textarea.
  if (key === "v") return "paste"
  if (key === "c") return !e.shiftKey && hasSelection ? "copy" : null
  return null
}
