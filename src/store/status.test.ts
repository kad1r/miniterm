import { describe, it, expect } from "vitest"
import { paneStatus } from "./status"

describe("paneStatus", () => {
  it("returns 'off' when slot is undefined", () => {
    expect(paneStatus(undefined)).toBe("off")
  })

  it("returns 'off' when all ids are null and no exits recorded", () => {
    expect(paneStatus({ ids: [null, null], exits: [undefined, undefined] })).toBe("off")
  })

  it("returns 'running' when some id is non-null and no exits", () => {
    expect(paneStatus({ ids: [1, null], exits: [undefined, undefined] })).toBe("running")
  })

  it("returns 'dead' on spawn failure (ids[i] null, exits[i] null)", () => {
    expect(paneStatus({ ids: [null], exits: [null] })).toBe("dead")
  })

  it("returns 'dead' on partial death (one pane dead, others running)", () => {
    expect(paneStatus({ ids: [1, 2], exits: [0, undefined] })).toBe("dead")
  })

  it("returns 'running' after restart (exits back to undefined, ids non-null)", () => {
    expect(paneStatus({ ids: [3], exits: [undefined] })).toBe("running")
  })
})
