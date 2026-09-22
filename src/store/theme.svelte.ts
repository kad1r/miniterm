import type { ThemeName } from "../term/theme"

/** UI colour scheme (dark/light). Kept out of `config.json` on purpose: it is a
 *  per-display preference rather than part of the workspace tree, and storing it
 *  in the config would mean a version bump plus a migration for every install.
 *  localStorage is the same home the font scale and sidebar collapse state use.
 *
 *  Reactive so `theme.current` read through markup (the toggle icon, the xterm
 *  palette effect) re-renders on change with no reload — remounting would tear
 *  down every live xterm instance. */
const KEY = "miniterm.theme"
const DEFAULT: ThemeName = "dark"

export const theme = $state({ current: initial() })

function initial(): ThemeName {
  if (typeof localStorage === "undefined") return DEFAULT
  const stored = localStorage.getItem(KEY)
  return stored === "light" || stored === "dark" ? stored : DEFAULT
}

/** Publish the theme to the document so the CSS token overrides in styles.css
 *  ([data-theme="light"]) and `color-scheme` take effect. */
function publish(name: ThemeName) {
  if (typeof document === "undefined") return
  // Dark is the :root default, so the attribute is only present for light — this
  // keeps the default path attribute-free and matches the design's token table.
  if (name === "light") document.documentElement.setAttribute("data-theme", "light")
  else document.documentElement.removeAttribute("data-theme")
}

export function setTheme(name: ThemeName) {
  theme.current = name
  publish(name)
  if (typeof localStorage !== "undefined") localStorage.setItem(KEY, name)
}

export function toggleTheme() {
  setTheme(theme.current === "dark" ? "light" : "dark")
}

/** Push the persisted theme into the document at startup. Without this the stored
 *  value would live in state while the document still rendered with the default. */
export function initTheme() {
  publish(theme.current)
}
