import { notify } from "./app.svelte"
import { t } from "../i18n/locale.svelte"
import * as logic from "./pomodoro"

/** Reactive pomodoro store. Logic lives in pomodoro.ts; this file owns the
 *  ticking interval, which only runs while a session is live so an idle timer
 *  costs nothing. */
export const pomodoro = $state(logic.initialState())

/** Popover placement, shared so both the footer button and the pinned sidebar
 *  row can open the same panel anchored above themselves. */
export const pomodoroPanel = $state({ open: false, left: 0, bottom: 0 })

export function togglePomodoroPanel(anchor: HTMLElement | undefined) {
  if (!pomodoroPanel.open && anchor) {
    const r = anchor.getBoundingClientRect()
    pomodoroPanel.left = r.left
    pomodoroPanel.bottom = window.innerHeight - r.top + 8
  }
  pomodoroPanel.open = !pomodoroPanel.open
}

let timer: ReturnType<typeof setInterval> | null = null

function stopTicking() {
  if (timer !== null) clearInterval(timer)
  timer = null
}

function startTicking() {
  stopTicking()
  // 250ms keeps the seconds display crisp without drifting a whole second late.
  timer = setInterval(() => {
    if (logic.tick(pomodoro, Date.now())) {
      stopTicking()
      chime()
      notify(t("pomodoro.done"))
    }
  }, 250)
}

export function selectPomodoro(min: number) {
  logic.select(pomodoro, min)
}

export function startPomodoro() {
  logic.start(pomodoro, Date.now())
  startTicking()
}

export function pausePomodoro() {
  logic.pause(pomodoro, Date.now())
  stopTicking()
}

export function resetPomodoro() {
  logic.reset(pomodoro)
  stopTicking()
}

/** Three soft sine blips via WebAudio — no asset to bundle. Best effort only. */
function chime() {
  try {
    const ctx = new AudioContext()
    const t0 = ctx.currentTime
    ;[0, 0.22, 0.44].forEach((offset, i) => {
      const osc = ctx.createOscillator()
      const gain = ctx.createGain()
      osc.type = "sine"
      osc.frequency.value = i === 2 ? 1046.5 : 784
      gain.gain.setValueAtTime(0, t0 + offset)
      gain.gain.linearRampToValueAtTime(0.18, t0 + offset + 0.02)
      gain.gain.exponentialRampToValueAtTime(0.001, t0 + offset + 0.35)
      osc.connect(gain).connect(ctx.destination)
      osc.start(t0 + offset)
      osc.stop(t0 + offset + 0.4)
    })
    setTimeout(() => void ctx.close(), 1500)
  } catch {
    // No audio device / autoplay policy — the toast still fires.
  }
}
