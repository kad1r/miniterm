// Sidebar typefaces: a text face and a mono face for its figures, picked in
// Settings → Appearance from the bundled Google Fonts below or from the fonts
// installed on the machine. The faces themselves are declared in fonts.css and
// sidebarFonts.css.

export type FontKind = "sans" | "mono"

export interface FontOption {
  id: string
  label: string
  /** CSS family name as declared by the @font-face rules. */
  family: string
  kind: FontKind
}

/** Popular Google Fonts, all bundled as variable fonts. Kept at 20. */
export const SIDEBAR_FONTS: readonly FontOption[] = [
  { id: "inter", label: "Inter", family: "Inter Variable", kind: "sans" },
  { id: "roboto", label: "Roboto", family: "Roboto Variable", kind: "sans" },
  { id: "open-sans", label: "Open Sans", family: "Open Sans Variable", kind: "sans" },
  { id: "montserrat", label: "Montserrat", family: "Montserrat Variable", kind: "sans" },
  { id: "nunito", label: "Nunito", family: "Nunito Variable", kind: "sans" },
  { id: "raleway", label: "Raleway", family: "Raleway Variable", kind: "sans" },
  { id: "work-sans", label: "Work Sans", family: "Work Sans Variable", kind: "sans" },
  { id: "dm-sans", label: "DM Sans", family: "DM Sans Variable", kind: "sans" },
  { id: "manrope", label: "Manrope", family: "Manrope Variable", kind: "sans" },
  { id: "source-sans-3", label: "Source Sans 3", family: "Source Sans 3 Variable", kind: "sans" },
  { id: "noto-sans", label: "Noto Sans", family: "Noto Sans Variable", kind: "sans" },
  { id: "ibm-plex-sans", label: "IBM Plex Sans", family: "IBM Plex Sans Variable", kind: "sans" },
  { id: "rubik", label: "Rubik", family: "Rubik Variable", kind: "sans" },
  { id: "plus-jakarta-sans", label: "Plus Jakarta Sans", family: "Plus Jakarta Sans Variable", kind: "sans" },
  { id: "geist", label: "Geist", family: "Geist Variable", kind: "sans" },
  { id: "roboto-mono", label: "Roboto Mono", family: "Roboto Mono Variable", kind: "mono" },
  { id: "jetbrains-mono", label: "JetBrains Mono", family: "JetBrains Mono Variable", kind: "mono" },
  { id: "fira-code", label: "Fira Code", family: "Fira Code Variable", kind: "mono" },
  { id: "source-code-pro", label: "Source Code Pro", family: "Source Code Pro Variable", kind: "mono" },
  { id: "geist-mono", label: "Geist Mono", family: "Geist Mono Variable", kind: "mono" },
]

/** A catalog id, or `system:<family>` for a font installed on the machine. */
export type FontChoice = string

export interface SidebarFonts {
  text: FontChoice
  mono: FontChoice
}

export const DEFAULT_SIDEBAR_FONTS: SidebarFonts = { text: "roboto", mono: "roboto-mono" }

const SYSTEM = "system:"

const FALLBACK: Record<FontKind, string> = {
  sans: 'system-ui, "Segoe UI", sans-serif',
  mono: '"Cascadia Mono", Consolas, ui-monospace, monospace',
}

export const systemChoice = (family: string): FontChoice => SYSTEM + family.trim()

export function systemFamily(choice: FontChoice): string | null {
  return choice.startsWith(SYSTEM) ? choice.slice(SYSTEM.length) : null
}

export const optionById = (id: FontChoice) => SIDEBAR_FONTS.find((f) => f.id === id) ?? null

export function isValidChoice(choice: unknown): choice is FontChoice {
  if (typeof choice !== "string") return false
  const system = systemFamily(choice)
  if (system !== null) return system.trim() !== "" && system.length <= 100
  return optionById(choice) !== null
}

/** A family name as a CSS string literal. */
export function quoteFamily(name: string): string {
  return `"${name.replace(/[\\"]/g, (c) => `\\${c}`).replace(/[\r\n]/g, " ")}"`
}

/** The `font-family` value for a choice, with a fallback of its kind. */
export function fontStack(choice: FontChoice, kind: FontKind): string {
  const system = systemFamily(choice)
  const family = system ?? optionById(choice)?.family ?? optionById(DEFAULT_SIDEBAR_FONTS[kind === "sans" ? "text" : "mono"])!.family
  return `${quoteFamily(family)}, ${FALLBACK[kind]}`
}

/** What a choice is called in the UI. */
export function choiceLabel(choice: FontChoice): string {
  return systemFamily(choice) ?? optionById(choice)?.label ?? choice
}

/** Stored JSON → settings; anything malformed falls back per field. */
export function parseSidebarFonts(raw: string | null): SidebarFonts {
  try {
    const v = raw ? (JSON.parse(raw) as Partial<SidebarFonts>) : {}
    return {
      text: isValidChoice(v.text) ? v.text : DEFAULT_SIDEBAR_FONTS.text,
      mono: isValidChoice(v.mono) ? v.mono : DEFAULT_SIDEBAR_FONTS.mono,
    }
  } catch {
    return { ...DEFAULT_SIDEBAR_FONTS }
  }
}
