import type { Workspace } from "./types"
import { terminalCount } from "./relayout"

/** One entry per cell of the grid, in cell order: the AI tool that cell's shell
 *  was opened with, or null for a bare shell. */
export type PaneTools = (string | null)[]

/**
 * The list, trimmed or padded to the grid's real size.
 *
 * Nothing guarantees the two stay in step on their own: `paneTools` was added
 * after `rows`/`cols`, so a config written by an older build has none at all,
 * and every path that changes the terminal count has to remember to resize it.
 * Reading through here means a mismatch degrades to the workspace default
 * instead of leaving a pane with no command, or none with an undefined one.
 *
 * `fallback` is the workspace's own tool — still the default for any cell that
 * has not been given one of its own.
 */
export function normalizeTools(tools: PaneTools, count: number, fallback: string | null): PaneTools {
  const out = tools.slice(0, Math.max(count, 0))
  while (out.length < count) out.push(fallback)
  return out
}

/** The per-cell tools of a workspace, always exactly one per cell. */
export function toolsFor(ws: Workspace): PaneTools {
  return normalizeTools(ws.paneTools, terminalCount(ws), ws.aiToolId)
}

/** Forget a tool that no longer exists. Mirrors what `removeTool` already does
 *  to the workspace's own `aiToolId`: a dangling id would spawn a bare shell
 *  anyway, so it is cleared rather than left to rot in the config. */
export function clearTool(tools: PaneTools, toolId: string): PaneTools {
  return tools.map((id) => (id === toolId ? null : id))
}
