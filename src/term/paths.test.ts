import { describe, it, expect } from "vitest"
import { dropText, quotePath, shellKind } from "./paths"

describe("shellKind", () => {
  it("recognises PowerShell, cmd and falls back to POSIX", () => {
    expect(shellKind("C:\\Program Files\\PowerShell\\7\\pwsh.exe")).toBe("powershell")
    expect(shellKind("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe")).toBe("powershell")
    expect(shellKind("C:\\Windows\\System32\\cmd.exe")).toBe("cmd")
    expect(shellKind("C:\\Program Files\\Git\\bin\\bash.exe")).toBe("posix")
    expect(shellKind("/bin/zsh")).toBe("posix")
    expect(shellKind(undefined)).toBe("posix")
  })
})

describe("quotePath", () => {
  it("leaves plain Windows and POSIX paths bare in every shell", () => {
    for (const kind of ["powershell", "cmd", "posix"] as const) {
      expect(quotePath("C:\\Users\\kadir\\shot.png", kind)).toBe("C:\\Users\\kadir\\shot.png")
      expect(quotePath("/home/kadir/shot.png", kind)).toBe("/home/kadir/shot.png")
    }
  })

  it("single-quotes for PowerShell so $(...) and $var never expand", () => {
    expect(quotePath("C:\\Users\\a b\\Screenshot 2026.png", "powershell"))
      .toBe("'C:\\Users\\a b\\Screenshot 2026.png'")
    expect(quotePath("C:\\dl\\a $(Start-Process calc).txt", "powershell"))
      .toBe("'C:\\dl\\a $(Start-Process calc).txt'")
    expect(quotePath("C:\\dl\\`$x.txt", "powershell")).toBe("'C:\\dl\\`$x.txt'")
  })

  it("doubles every quote PowerShell treats as a single quote", () => {
    expect(quotePath("C:\\it's\\x.png", "powershell")).toBe("'C:\\it''s\\x.png'")
    expect(quotePath("C:\\it\u2019s\\x.png", "powershell")).toBe("'C:\\it\u2019\u2019s\\x.png'")
  })

  it("double-quotes for cmd, which has no command substitution", () => {
    expect(quotePath("C:\\Users\\a b\\Screenshot 2026.png", "cmd"))
      .toBe('"C:\\Users\\a b\\Screenshot 2026.png"')
    expect(quotePath("\\\\server\\share\\a b.png", "cmd")).toBe('"\\\\server\\share\\a b.png"')
  })

  it("single-quotes for POSIX shells, Windows paths included (Git Bash)", () => {
    expect(quotePath("/home/a b/shot.png", "posix")).toBe("'/home/a b/shot.png'")
    expect(quotePath("/home/$USER/shot.png", "posix")).toBe("'/home/$USER/shot.png'")
    expect(quotePath("C:\\dl\\a $(id).txt", "posix")).toBe("'C:\\dl\\a $(id).txt'")
  })

  it("escapes an embedded single quote for POSIX", () => {
    expect(quotePath("/home/it's/shot.png", "posix")).toBe("'/home/it'\\''s/shot.png'")
  })
})

describe("dropText", () => {
  it("joins several paths with a space", () => {
    expect(dropText(["C:\\a.png", "C:\\b c.png"], "cmd")).toBe('C:\\a.png "C:\\b c.png"')
    expect(dropText(["C:\\a.png", "C:\\b c.png"], "powershell")).toBe("C:\\a.png 'C:\\b c.png'")
  })

  it("is empty for an empty drop", () => {
    expect(dropText([], "posix")).toBe("")
  })
})
