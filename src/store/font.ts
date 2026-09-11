/** Text size, as multipliers of the design sizes.
 *  Discrete steps rather than free scaling: every step lands on a level that was
 *  eyeballed against the 13px base, and stepping is predictable from the
 *  keyboard. Both ends are deliberately conservative — beyond 2x the terminal
 *  grid stops fitting a useful number of columns. */
export const FONT_SCALES = [0.8, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2] as const

export const DEFAULT_FONT_SCALE = 1
export const MIN_FONT_SCALE = FONT_SCALES[0]
export const MAX_FONT_SCALE = FONT_SCALES[FONT_SCALES.length - 1]

export const FONT_SCALE_KEY = "miniterm.fontScale"

/** The slice of `Storage` this module needs. Narrow on purpose: it keeps the
 *  functions testable in the node environment, where `localStorage` is absent. */
export interface FontScaleStore {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

/** Move one level along the scale, stopping at either end.
 *
 * Defined as "the next level strictly past `current`" rather than as an index
 * shift, so a value that is not on the scale — a hand-edited storage entry, or
 * a level dropped in a later version — steps onto the scale instead of being
 * rejected or stranded.
 */
export function stepFontScale(current: number, direction: 1 | -1): number {
  if (direction === 1) {
    return FONT_SCALES.find((level) => level > current + 1e-9) ?? MAX_FONT_SCALE
  }
  return [...FONT_SCALES].reverse().find((level) => level < current - 1e-9) ?? MIN_FONT_SCALE
}

/** Read the persisted scale, defaulting to 1.
 *  Anything unusable resolves to the default: the scale multiplies every text
 *  size in the window, so a stored `0`, `NaN` or `40` would leave the app
 *  unreadable with no visible control to undo it. Storage access itself can
 *  throw (private browsing, disabled cookies) and is caught for the same
 *  reason — a preference must not stop the app from rendering. */
export function readFontScale(store: FontScaleStore): number {
  try {
    const value = Number(store.getItem(FONT_SCALE_KEY))
    if (!Number.isFinite(value) || value < MIN_FONT_SCALE || value > MAX_FONT_SCALE) {
      return DEFAULT_FONT_SCALE
    }
    return value
  } catch {
    return DEFAULT_FONT_SCALE
  }
}

/** Persist the scale. Failure is silent: this is a preference, and a quota or
 *  permission error must not break the control itself. */
export function writeFontScale(store: FontScaleStore, scale: number): void {
  try {
    store.setItem(FONT_SCALE_KEY, String(scale))
  } catch {
    // ignored — see doc comment
  }
}

/** The xterm font size for a scale.
 *  Rounded because xterm derives cell width and height by measuring a glyph at
 *  this size; a fractional size yields fractional cells, which the renderer
 *  then snaps per-cell and the grid drifts. */
export function termFontSize(base: number, scale: number): number {
  return Math.max(1, Math.round(base * scale))
}

export type FontAction = "in" | "out" | "reset"

/** Map a keydown to a font-size action, or null when the key is not ours.
 *
 * `=` is included because Ctrl+`+` on a US layout arrives unshifted as `=`;
 * `_` is its counterpart for the shifted minus. The modifier check is what
 * keeps `-` and `0` usable as ordinary terminal input.
 */
export function fontAction(
  e: { key: string; ctrlKey: boolean; metaKey: boolean },
): FontAction | null {
  if (!e.ctrlKey && !e.metaKey) return null
  if (e.key === "+" || e.key === "=") return "in"
  if (e.key === "-" || e.key === "_") return "out"
  if (e.key === "0") return "reset"
  return null
}
