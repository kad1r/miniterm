import { describe, it, expect } from "vitest"
import { alertFor, applyUpdate, createLimiter, rollup, type AgentInfo } from "./agentStatus"

const up = (state: AgentInfo["state"], subagents = 0) => ({ id: 1, state, subagents })
const info = (state: AgentInfo["state"], unread = false): AgentInfo => ({ state, subagents: 0, unread })

describe("applyUpdate", () => {
  it("marks done as unread when nobody is looking", () => {
    expect(applyUpdate(info("working"), up("done"), false).unread).toBe(true)
  })

  it("leaves done read when the pane is being watched", () => {
    expect(applyUpdate(info("working"), up("done"), true).unread).toBe(false)
  })

  it("a new turn clears unread", () => {
    expect(applyUpdate(info("done", true), up("working"), false).unread).toBe(false)
  })

  it("a repeated state keeps the flag as it was", () => {
    expect(applyUpdate(info("waiting", true), up("waiting", 1), true)).toEqual({
      state: "waiting", subagents: 1, unread: true,
    })
  })

  it("hints never mark unread", () => {
    expect(applyUpdate(undefined, up("idle"), false).unread).toBe(false)
    expect(applyUpdate(undefined, up("active"), false).unread).toBe(false)
  })
})

describe("alertFor", () => {
  it("alerts on entering waiting or done while unseen", () => {
    expect(alertFor(info("working"), up("waiting"), false)).toBe("waiting")
    expect(alertFor(info("working"), up("done"), false)).toBe("done")
  })

  it("stays quiet when seen, repeated, or a hint", () => {
    expect(alertFor(info("working"), up("done"), true)).toBeNull()
    expect(alertFor(info("waiting"), up("waiting"), false)).toBeNull()
    expect(alertFor(info("active"), up("idle"), false)).toBeNull()
    expect(alertFor(info("done"), up("working"), false)).toBeNull()
  })
})

describe("rollup", () => {
  it("ranks waiting over unread done over working", () => {
    expect(rollup([info("working"), info("done", true), info("waiting")])).toBe("waiting")
    expect(rollup([info("working"), info("done", true)])).toBe("unread")
    expect(rollup([info("working"), undefined])).toBe("working")
  })

  it("a seen done and hints say nothing", () => {
    expect(rollup([info("done"), info("idle"), info("active"), undefined])).toBeNull()
  })
})

describe("createLimiter", () => {
  it("lets one through per key per window", () => {
    const allow = createLimiter(10_000)
    expect(allow(1, 0)).toBe(true)
    expect(allow(1, 9_999)).toBe(false)
    expect(allow(2, 9_999)).toBe(true)
    expect(allow(1, 10_000)).toBe(true)
  })
})
