import {
  DEFAULT_SIDEBAR_FONTS, fontStack, parseSidebarFonts, type FontChoice, type SidebarFonts,
} from "./sidebarFont"

/** A per-machine look preference, so localStorage like the theme and font
 *  scale rather than config.json. Published as two CSS variables the sidebar
 *  reads, so a change restyles it in place. */
const KEY = "miniterm.sidebarFonts"

export const sidebarFonts = $state<SidebarFonts>({ ...DEFAULT_SIDEBAR_FONTS })

function publish() {
  if (typeof document === "undefined") return
  const root = document.documentElement.style
  root.setProperty("--sidebar-font", fontStack(sidebarFonts.text, "sans"))
  root.setProperty("--sidebar-font-mono", fontStack(sidebarFonts.mono, "mono"))
}

export function initSidebarFonts() {
  let raw: string | null = null
  try {
    raw = localStorage.getItem(KEY)
  } catch {
    // Storage unavailable: the defaults stand.
  }
  Object.assign(sidebarFonts, parseSidebarFonts(raw))
  publish()
}

export function setSidebarFont(field: keyof SidebarFonts, choice: FontChoice) {
  sidebarFonts[field] = choice
  publish()
  try {
    localStorage.setItem(KEY, JSON.stringify({ text: sidebarFonts.text, mono: sidebarFonts.mono }))
  } catch {
    // Not persisted; still applied for this run.
  }
}

export function resetSidebarFonts() {
  setSidebarFont("text", DEFAULT_SIDEBAR_FONTS.text)
  setSidebarFont("mono", DEFAULT_SIDEBAR_FONTS.mono)
}
