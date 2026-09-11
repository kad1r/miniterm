/** The brand marks the wizard can draw. Deliberately short: the default config
 *  ships Claude Code and Gemini, and anything else falls back to a letter badge
 *  rather than to a mark we would have to invent. */
export type LogoKey = "claude" | "gemini"

/** Ordered token → mark table. Substring matching is what makes this work
 *  across the shapes a command actually takes: a bare `claude`, an explicit
 *  path like `C:\tools\claude.exe`, and launcher forms like
 *  `npx @google/gemini-cli` all carry the vendor name somewhere in the string. */
const MARKS: ReadonlyArray<readonly [string, LogoKey]> = [
  ["claude", "claude"],
  ["gemini", "gemini"],
]

/** Pick the mark for an AI tool's command, or null when we have none.
 *  Keyed on the command rather than the tool id because ids are random UUIDs
 *  generated per install — see `default_config` in config.rs. */
export function logoKeyFor(command: string): LogoKey | null {
  const needle = command.toLowerCase()
  return MARKS.find(([token]) => needle.includes(token))?.[1] ?? null
}
