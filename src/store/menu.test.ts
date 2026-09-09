import { describe, it, expect } from "vitest"
import { pickItem } from "./menu"

describe("pickItem", () => {
  it("runs the action before closing the menu", () => {
    // Order is load-bearing, not cosmetic. Closing first destroys the button
    // that is mid-click, and WebView2 silently suppresses any modal script
    // dialog (prompt/confirm) opened while its invoking element is being torn
    // down — which is how rename and delete came to do nothing at all.
    const order: string[] = []
    pickItem({ label: "Sil", action: () => order.push("action") }, () => order.push("close"))
    expect(order).toEqual(["action", "close"])
  })

  it("still closes when the action throws", () => {
    // A failing action must not leave the menu stuck open over the tree.
    let closed = false
    const boom = { label: "Sil", action: () => { throw new Error("boom") } }
    expect(() => pickItem(boom, () => (closed = true))).toThrow("boom")
    expect(closed).toBe(true)
  })
})
