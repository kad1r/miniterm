import {
  DEFAULT_FONT_SCALE, readFontScale, stepFontScale, writeFontScale, type FontAction,
} from "./font"

/** Global text scale. Kept out of `config.json` on purpose: it is a per-display
 *  preference rather than part of the workspace tree, and storing it in the
 *  config would mean a version bump plus a migration for every install (see the
 *  version gate in config.rs). localStorage is the same home the sidebar
 *  collapse state uses. */
export const fontScale = $state({ level: initialLevel() })

function initialLevel(): number {
  return typeof localStorage === "undefined" ? DEFAULT_FONT_SCALE : readFontScale(localStorage)
}

/** Publish the level to CSS. Only `font-size` declarations multiply by
 *  `--font-scale`: padding, borders and icon boxes keep their design sizes, so
 *  the layout stays put and only the text grows. The terminal reads the same
 *  level directly — see the font-size effect in TerminalPane. */
function publish(level: number) {
  if (typeof document === "undefined") return
  document.documentElement.style.setProperty("--font-scale", String(level))
}

export function setFontScale(level: number) {
  fontScale.level = level
  publish(level)
  if (typeof localStorage !== "undefined") writeFontScale(localStorage, level)
}

export function applyFontAction(action: FontAction) {
  if (action === "reset") return setFontScale(DEFAULT_FONT_SCALE)
  setFontScale(stepFontScale(fontScale.level, action === "in" ? 1 : -1))
}

/** Push the persisted level into CSS at startup. Without this the stored value
 *  would live in state while the document still rendered at 1. */
export function initFontScale() {
  publish(fontScale.level)
}
