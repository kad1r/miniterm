import type { PaneStatus } from "./sessions.svelte"
import type { AgentRollup } from "./agentStatus"

export type { PaneStatus }

/**
 * Pure function — no runes, safe to import from vitest (node environment).
 * `statusOf` in sessions.svelte.ts is a thin wrapper over this.
 */
export function paneStatus(
  slot:
    | {
        ids: (number | null)[]
        exits: (number | null | undefined)[]
        /** Terminals sent to the strip below the grid. Off screen, still alive,
         *  so the sidebar dot has to account for them too. */
        minimized?: { id: number | null; exit: number | null | undefined }[]
      }
    | undefined,
  /** The workspace's agent summary (see `rollup`). Ranks below dead and above
   *  plain running: dead > waiting > unread > working > running > off. */
  agent: AgentRollup = null,
): PaneStatus {
  if (!slot) return "off"
  const minimized = slot.minimized ?? []
  if (slot.exits.some((e) => e !== undefined)) return "dead"
  if (minimized.some((m) => m.exit !== undefined)) return "dead"
  const live = slot.ids.some((id) => id !== null) || minimized.some((m) => m.id !== null)
  if (!live) return "off"
  return agent ?? "running"
}
