import type { PaneStatus } from "./sessions.svelte"

export type { PaneStatus }

/**
 * Pure function — no runes, safe to import from vitest (node environment).
 * `statusOf` in sessions.svelte.ts is a thin wrapper over this.
 */
export function paneStatus(
  slot: { ids: (number | null)[]; exits: (number | null | undefined)[] } | undefined,
): PaneStatus {
  if (!slot) return "off"
  if (slot.exits.some((e) => e !== undefined)) return "dead"
  return slot.ids.some((id) => id !== null) ? "running" : "off"
}
