// A Windows path starts with a drive letter or a UNC prefix; anything else is
// treated as POSIX. Deriving the flavour from the path itself rather than from
// the workspace's shell keeps this pure and correct for the mixed case (Git
// Bash on Windows still receives Windows paths).
const WINDOWS_PATH = /^(?:[A-Za-z]:[\\/]|\\\\)/
// Bare characters that no shell splits, expands or otherwise reinterprets.
const WINDOWS_BARE = /^[A-Za-z0-9_.:\\/+-]+$/
const POSIX_BARE = /^[A-Za-z0-9_./+-]+$/

/**
 * Quote a dropped file path for the shell, the way Windows Terminal does.
 *
 * Windows filenames cannot contain a double quote, so wrapping is enough there.
 * POSIX names can contain anything, so single quotes are used and any embedded
 * quote is closed, escaped and reopened.
 */
export function quotePath(path: string): string {
  if (WINDOWS_PATH.test(path)) return WINDOWS_BARE.test(path) ? path : `"${path}"`
  if (POSIX_BARE.test(path)) return path
  return `'${path.replace(/'/g, "'\\''")}'`
}

/** Text to inject for a drop: the quoted paths, space separated, no newline. */
export function dropText(paths: string[]): string {
  return paths.map(quotePath).join(" ")
}
