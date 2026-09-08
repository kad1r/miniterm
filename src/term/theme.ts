import type { ITheme } from "@xterm/xterm"

/// Mirrors --font-mono in styles.css. xterm.js takes a plain string and cannot
/// read CSS custom properties, so the list is duplicated rather than shared.
export const TERM_FONT =
  '"Roboto Mono", "Cascadia Mono", "JetBrains Mono", "SF Mono", Menlo, Consolas, monospace'
export const TERM_FONT_SIZE = 13

export const TERM_THEME: ITheme = {
  background: "#0f1114",
  foreground: "#d6dae0",
  cursor: "#d6dae0",
  cursorAccent: "#0f1114",
  selectionBackground: "#3d7dff55",
  black: "#1c2027",
  red: "#f0574b",
  green: "#3fb950",
  yellow: "#d6a13a",
  blue: "#3d7dff",
  magenta: "#b07ce8",
  cyan: "#3bb0c4",
  white: "#c7ccd3",
  brightBlack: "#7d858f",
  brightRed: "#ff7367",
  brightGreen: "#57d16a",
  brightYellow: "#efbb50",
  brightBlue: "#5f95ff",
  brightMagenta: "#c795f5",
  brightCyan: "#54c9dd",
  brightWhite: "#eef1f5",
}
