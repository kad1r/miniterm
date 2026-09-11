import { describe, it, expect } from "vitest"
import { exitNotice } from "./exit"

describe("exitNotice", () => {
  it("includes the exit code and the restart hint", () => {
    expect(exitNotice(3, "tr")).toBe(
      "\r\n\x1b[90m[process exited: 3] — yeniden başlatmak için Enter\x1b[0m\r\n",
    )
    expect(exitNotice(3, "en")).toBe(
      "\r\n\x1b[90m[process exited: 3] — press Enter to restart\x1b[0m\r\n",
    )
  })

  it("says the code is unknown when there is none", () => {
    expect(exitNotice(null, "tr")).toContain("[process exited: bilinmiyor]")
    expect(exitNotice(null, "en")).toContain("[process exited: unknown]")
  })

  it("keeps zero as a real code, not as missing", () => {
    expect(exitNotice(0, "en")).toContain("[process exited: 0]")
  })
})
