/** How the pane's shell parses a quoted word. The drop is typed into that
 *  shell, so the quoting has to match it: a double-quoted string is inert in
 *  cmd but runs `$(...)` in PowerShell and in bash. */
export type ShellKind = "powershell" | "cmd" | "posix"

/** The quoting family of a shell, from its program path. Anything not
 *  recognised as PowerShell or cmd is treated as POSIX, whose single quotes
 *  expand nothing. */
export function shellKind(program: string | null | undefined): ShellKind {
  const base = (program ?? "").split(/[\\/]/).pop()!.toLowerCase().replace(/\.exe$/, "")
  if (base === "pwsh" || base === "powershell") return "powershell"
  if (base === "cmd") return "cmd"
  return "posix"
}

// A Windows path starts with a drive letter or a UNC prefix.
const WINDOWS_PATH = /^(?:[A-Za-z]:[\\/]|\\\\)/
// Bare characters that no shell splits, expands or otherwise reinterprets.
const WINDOWS_BARE = /^[A-Za-z0-9_.:\\/+-]+$/
const POSIX_BARE = /^[A-Za-z0-9_./+-]+$/
// PowerShell accepts the typographic single quotes as quote characters too, so
// every one of them has to be doubled inside a single-quoted string.
const PS_QUOTES = /['‘’‚‛]/g

/**
 * Quote a dropped file path for the pane's shell, the way Windows Terminal does.
 *
 * PowerShell and POSIX shells get single quotes, the only form in which nothing
 * is expanded. cmd has no single quotes but no command substitution either, and
 * a Windows filename cannot contain a double quote, so wrapping is enough there.
 */
export function quotePath(path: string, kind: ShellKind): string {
  const bare = WINDOWS_PATH.test(path) ? WINDOWS_BARE : POSIX_BARE
  if (bare.test(path)) return path
  switch (kind) {
    case "powershell":
      return `'${path.replace(PS_QUOTES, "$&$&")}'`
    case "cmd":
      return `"${path}"`
    case "posix":
      return `'${path.replace(/'/g, "'\\''")}'`
  }
}

/** Text to inject for a drop: the quoted paths, space separated, no newline. */
export function dropText(paths: string[], kind: ShellKind): string {
  return paths.map((p) => quotePath(p, kind)).join(" ")
}
