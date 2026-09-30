import { describe, it, expect } from "vitest"
import {
  clampMinutes, formatClock, initialState, minutes, pause, progress, reset, select, stage, start, tick,
} from "./pomodoro"

describe("pomodoro", () => {
  it("starts idle at 25 minutes", () => {
    const s = initialState()
    expect(s.status).toBe("idle")
    expect(s.remaining).toBe(minutes(25))
  })

  it("counts down from a wall-clock deadline", () => {
    const s = initialState(25)
    start(s, 1_000)
    expect(s.status).toBe("running")
    expect(tick(s, 1_000 + minutes(10))).toBe(false)
    expect(s.remaining).toBe(minutes(15))
  })

  it("pause freezes remaining time and resume continues from it", () => {
    const s = initialState(25)
    start(s, 0)
    pause(s, minutes(5))
    expect(s.status).toBe("paused")
    expect(s.remaining).toBe(minutes(20))
    // Time passing while paused does not count.
    start(s, minutes(100))
    tick(s, minutes(101))
    expect(s.remaining).toBe(minutes(19))
  })

  it("finishes exactly once", () => {
    const s = initialState(1)
    start(s, 0)
    expect(tick(s, minutes(2))).toBe(true)
    expect(s.status).toBe("done")
    expect(s.remaining).toBe(0)
    expect(tick(s, minutes(3))).toBe(false)
  })

  it("start after done begins a fresh session", () => {
    const s = initialState(1)
    start(s, 0)
    tick(s, minutes(1))
    start(s, minutes(5))
    expect(s.status).toBe("running")
    expect(s.remaining).toBe(minutes(1))
  })

  it("reset returns to the full selected length", () => {
    const s = initialState(45)
    start(s, 0)
    tick(s, minutes(30))
    reset(s)
    expect(s.status).toBe("idle")
    expect(s.remaining).toBe(minutes(45))
  })

  it("select is ignored during a live session", () => {
    const s = initialState(25)
    start(s, 0)
    select(s, 60)
    expect(s.duration).toBe(minutes(25))
    reset(s)
    select(s, 60)
    expect(s.duration).toBe(minutes(60))
    expect(s.remaining).toBe(minutes(60))
  })

  it("clamps custom minutes", () => {
    expect(clampMinutes(0)).toBe(1)
    expect(clampMinutes(999)).toBe(180)
    expect(clampMinutes(Number.NaN)).toBe(25)
    expect(clampMinutes(12.6)).toBe(13)
  })

  it("reports progress for the hourglass", () => {
    const s = initialState(10)
    expect(progress(s)).toBe(0)
    start(s, 0)
    tick(s, minutes(5))
    expect(progress(s)).toBeCloseTo(0.5)
  })

  it("steps the border colour toward green each quarter", () => {
    const s = initialState(100)
    start(s, 0)
    expect(stage(s)).toBe(0)
    tick(s, minutes(30))
    expect(stage(s)).toBe(1)
    tick(s, minutes(60))
    expect(stage(s)).toBe(2)
    tick(s, minutes(99))
    expect(stage(s)).toBe(3)
    tick(s, minutes(100))
    expect(stage(s)).toBe(4)
  })

  it("formats the clock", () => {
    expect(formatClock(minutes(25))).toBe("25:00")
    expect(formatClock(1_001)).toBe("00:02")
    expect(formatClock(minutes(60))).toBe("1:00:00")
    expect(formatClock(0)).toBe("00:00")
  })
})
