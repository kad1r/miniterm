import type { ITheme } from "@xterm/xterm"

/// Mirrors --font-mono in styles.css, character for character. xterm.js takes a
/// plain string and cannot read CSS custom properties, so the list is duplicated
/// rather than shared — keep the two in step when either changes.
export const TERM_FONT =
  '"JetBrains Mono Variable", "Cascadia Mono", "SF Mono", Menlo, Consolas, ui-monospace, monospace'
export const TERM_FONT_SIZE = 13

export type ThemeName = "dark" | "light"

/// xterm cannot read CSS custom properties, so each palette is spelled out here
/// and swapped imperatively when the app theme changes (see TerminalPane). The
/// surface colours match --bg-term / --text-1 in styles.css for each theme.
const DARK: ITheme = {
  background: "#0f1217",
  foreground: "#c3c9d3",
  cursor: "#e6e9ee",
  cursorAccent: "#0f1217",
  selectionBackground: "#e8825a55",
  black: "#1c2027",
  red: "#e0776c",
  green: "#4ec48a",
  yellow: "#dda63b",
  blue: "#7fc4e8",
  magenta: "#a68bd8",
  cyan: "#54c9dd",
  white: "#c7ccd3",
  brightBlack: "#6b727e",
  brightRed: "#ef8c81",
  brightGreen: "#63d59d",
  brightYellow: "#efbb50",
  brightBlue: "#95d3f2",
  brightMagenta: "#bda2e8",
  brightCyan: "#6fd8e8",
  brightWhite: "#eef1f5",
}

const LIGHT: ITheme = {
  background: "#ffffff",
  foreground: "#191a1c",
  cursor: "#191a1c",
  cursorAccent: "#ffffff",
  selectionBackground: "#c2552e33",
  black: "#191a1c",
  red: "#b03a2e",
  green: "#17795a",
  yellow: "#a4700c",
  blue: "#2f6fb0",
  magenta: "#7a4fb0",
  cyan: "#0e7490",
  white: "#56595f",
  brightBlack: "#797c82",
  brightRed: "#c2552e",
  brightGreen: "#1f8f6c",
  brightYellow: "#b8820f",
  brightBlue: "#3a80c8",
  brightMagenta: "#8f5fc8",
  brightCyan: "#128aa8",
  brightWhite: "#191a1c",
}

export const TERM_THEMES: Record<ThemeName, ITheme> = { dark: DARK, light: LIGHT }

/// Back-compat default: the dark palette. New code should read TERM_THEMES by
/// the active theme name instead.
export const TERM_THEME: ITheme = DARK
