import { describe, it, expect } from "vitest"
import { dropText, quotePath } from "./paths"

describe("quotePath", () => {
  it("leaves plain Windows and POSIX paths bare", () => {
    expect(quotePath("C:\\Users\\kadir\\shot.png")).toBe("C:\\Users\\kadir\\shot.png")
    expect(quotePath("/home/kadir/shot.png")).toBe("/home/kadir/shot.png")
  })

  it("double-quotes Windows paths with spaces or shell characters", () => {
    expect(quotePath("C:\\Users\\a b\\Screenshot 2026.png"))
      .toBe('"C:\\Users\\a b\\Screenshot 2026.png"')
    expect(quotePath("C:\\tmp\\re(port).png")).toBe('"C:\\tmp\\re(port).png"')
    expect(quotePath("\\\\server\\share\\a b.png")).toBe('"\\\\server\\share\\a b.png"')
  })

  it("single-quotes POSIX paths so nothing expands", () => {
    expect(quotePath("/home/a b/shot.png")).toBe("'/home/a b/shot.png'")
    expect(quotePath("/home/$USER/shot.png")).toBe("'/home/$USER/shot.png'")
  })

  it("escapes an embedded single quote in a POSIX path", () => {
    expect(quotePath("/home/it's/shot.png")).toBe("'/home/it'\\''s/shot.png'")
  })
})

describe("dropText", () => {
  it("joins several paths with a space", () => {
    expect(dropText(["C:\\a.png", "C:\\b c.png"])).toBe('C:\\a.png "C:\\b c.png"')
  })

  it("is empty for an empty drop", () => {
    expect(dropText([])).toBe("")
  })
})
