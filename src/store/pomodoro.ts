// Pure pomodoro timer logic, kept out of the `.svelte.ts` store so it can be
// unit-tested without the Svelte compiler. Same split as update.ts /
// update.svelte.ts: functions mutate the passed-in state, the reactive store
// binds them to a `$state` object.
//
// Time is tracked as a wall-clock deadline (`endAt`) rather than a decrementing
// counter: webview timers are throttled when the window is hidden, and a
// counter would silently drift behind real time.

export type PomodoroStatus = "idle" | "running" | "paused" | "done"

export type PomodoroState = {
  status: PomodoroStatus
  /** Selected session length in ms. */
  duration: number
  /** Deadline (epoch ms) while running. */
  endAt: number | null
  /** Time left, frozen while paused / idle. */
  remaining: number
}

export const PRESETS_MIN = [25, 45, 60] as const
export const MIN_MINUTES = 1
export const MAX_MINUTES = 180

export function minutes(n: number): number {
  return n * 60_000
}

export function initialState(durationMin: number = PRESETS_MIN[0]): PomodoroState {
  const duration = minutes(durationMin)
  return { status: "idle", duration, endAt: null, remaining: duration }
}

/** Clamp free-typed minutes into the supported range; NaN falls back to 25. */
export function clampMinutes(n: number): number {
  if (!Number.isFinite(n)) return PRESETS_MIN[0]
  return Math.min(MAX_MINUTES, Math.max(MIN_MINUTES, Math.round(n)))
}

/** Pick a length. Only allowed while not running, so a live session is never cut short. */
export function select(s: PomodoroState, durationMin: number): void {
  if (s.status === "running" || s.status === "paused") return
  s.duration = minutes(clampMinutes(durationMin))
  s.remaining = s.duration
  s.endAt = null
  s.status = "idle"
}

export function start(s: PomodoroState, now: number): void {
  if (s.status === "running") return
  if (s.status === "done" || s.remaining <= 0) s.remaining = s.duration
  s.endAt = now + s.remaining
  s.status = "running"
}

export function pause(s: PomodoroState, now: number): void {
  if (s.status !== "running" || s.endAt === null) return
  s.remaining = Math.max(0, s.endAt - now)
  s.endAt = null
  s.status = "paused"
}

export function reset(s: PomodoroState): void {
  s.remaining = s.duration
  s.endAt = null
  s.status = "idle"
}

/** Advance the clock. Returns true exactly once, on the tick that finishes the session. */
export function tick(s: PomodoroState, now: number): boolean {
  if (s.status !== "running" || s.endAt === null) return false
  s.remaining = Math.max(0, s.endAt - now)
  if (s.remaining > 0) return false
  s.endAt = null
  s.status = "done"
  return true
}

/** 0 → just started, 1 → finished. Drives the hourglass sand level. */
export function progress(s: PomodoroState): number {
  if (s.duration <= 0) return 0
  return Math.min(1, Math.max(0, 1 - s.remaining / s.duration))
}

/** Colour stage for the sidebar row's progress border: 0 at the start, one step
 *  per quarter of the session, 4 once finished (green). */
export function stage(s: PomodoroState): 0 | 1 | 2 | 3 | 4 {
  if (s.status === "done") return 4
  const p = progress(s)
  return p < 0.25 ? 0 : p < 0.5 ? 1 : p < 0.75 ? 2 : 3
}

/** mm:ss, or h:mm:ss past the hour. Rounds up so 0:00 only shows when truly done. */
export function formatClock(ms: number): string {
  const total = Math.ceil(Math.max(0, ms) / 1000)
  const h = Math.floor(total / 3600)
  const m = Math.floor((total % 3600) / 60)
  const sec = total % 60
  const pad = (n: number) => String(n).padStart(2, "0")
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${pad(m)}:${pad(sec)}`
}
