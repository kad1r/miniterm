import { describe, it, expect } from "vitest"
import { touch, LIVE_LIMIT } from "./lru"

// Global Constraint lock: changing LIVE_LIMIT affects RAM budget (one full xterm
// DOM tree per slot). Any change must be deliberate — raise this test with it.
it("LIVE_LIMIT is 3", () => {
  expect(LIVE_LIMIT).toBe(3)
})

describe("touch", () => {
  it("puts a new id at the front", () => {
    expect(touch([], "a", 3)).toEqual({ list: ["a"], evicted: [] })
  })

  it("moves an existing id to the front without growing the list", () => {
    expect(touch(["a", "b", "c"], "c", 3)).toEqual({ list: ["c", "a", "b"], evicted: [] })
  })

  it("evicts the least recently used id past the limit", () => {
    expect(touch(["c", "b", "a"], "d", 3)).toEqual({ list: ["d", "c", "b"], evicted: ["a"] })
  })

  it("evicts everything past the limit at once", () => {
    expect(touch(["d", "c", "b", "a"], "e", 2)).toEqual({
      list: ["e", "d"],
      evicted: ["c", "b", "a"],
    })
  })

  it("never evicts the id being touched", () => {
    const { list, evicted } = touch(["b", "c", "a"], "a", 1)
    expect(list).toEqual(["a"])
    expect(evicted).toEqual(["b", "c"])
  })
})
