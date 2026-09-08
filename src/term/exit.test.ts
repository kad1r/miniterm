import { describe, it, expect } from "vitest"
import { exitNotice } from "./exit"

describe("exitNotice", () => {
  it("includes the exit code and the restart hint", () => {
    expect(exitNotice(3)).toBe(
      "\r\n\x1b[90m[process exited: 3] — yeniden başlatmak için Enter\x1b[0m\r\n",
    )
  })

  it("says bilinmiyor when there is no code", () => {
    expect(exitNotice(null)).toBe(
      "\r\n\x1b[90m[process exited: bilinmiyor] — yeniden başlatmak için Enter\x1b[0m\r\n",
    )
  })

  it("keeps zero as a real code, not as missing", () => {
    expect(exitNotice(0)).toContain("[process exited: 0]")
  })
})
