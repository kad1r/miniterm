/**
 * Agent status per session, as reported by the Rust watcher (`agent-status`
 * event). Pure functions — no runes, safe to import from vitest (node).
 *
 * working / waiting / done come from Claude Code hooks. active / idle are a
 * hint for tools without hooks: output in the last 2 s or not. A hint cannot
 * tell waiting from done, so it never marks unread and never alerts.
 */

export type AgentState = "working" | "waiting" | "done" | "active" | "idle"

export interface AgentUpdate {
  id: number
  state: AgentState
  subagents: number
}

export interface AgentInfo {
  state: AgentState
  subagents: number
  /** Finished or asked something while nobody was looking at the pane. */
  unread: boolean
}

export type Alert = "waiting" | "done"

const isHint = (s: AgentState) => s === "active" || s === "idle"

/** Folds one update in. `seen`: the pane is on screen, focused, and the window
 *  has focus — an event the user is watching happen is already read. */
export function applyUpdate(prev: AgentInfo | undefined, u: AgentUpdate, seen: boolean): AgentInfo {
  const entered = prev?.state !== u.state
  let unread = prev?.unread ?? false
  if (isHint(u.state) || u.state === "working") unread = false
  else if (entered) unread = !seen
  return { state: u.state, subagents: u.subagents, unread }
}

/** What, if anything, deserves a toast / taskbar flash for this update. Only on
 *  entering waiting or done, and only when the user is not already looking. */
export function alertFor(prev: AgentInfo | undefined, u: AgentUpdate, seen: boolean): Alert | null {
  if (seen || prev?.state === u.state) return null
  return u.state === "waiting" || u.state === "done" ? u.state : null
}

/** Workspace-level agent summary for the sidebar dot, highest first:
 *  waiting > unread done > working. A done that was already seen says nothing,
 *  so the dot falls back to plain running and the sidebar stays calm. */
export type AgentRollup = "waiting" | "unread" | "working" | null

export function rollup(infos: (AgentInfo | undefined)[]): AgentRollup {
  if (infos.some((i) => i?.state === "waiting")) return "waiting"
  if (infos.some((i) => i?.state === "done" && i.unread)) return "unread"
  if (infos.some((i) => i?.state === "working")) return "working"
  return null
}

/** At most one alert per key per `ms` — a Notification/Stop burst is one toast. */
export function createLimiter(ms: number): (key: number, now: number) => boolean {
  const last = new Map<number, number>()
  return (key, now) => {
    const at = last.get(key)
    if (at !== undefined && now - at < ms) return false
    last.set(key, now)
    return true
  }
}
