import { describe, it, expect } from "vitest"
import {
  isAppShortcut, isHelpChord, isNewTerminalChord, isNewWorkspaceChord, isSettingsChord,
  workspaceIndexChord,
} from "./shortcuts"

const key = (over: Partial<Parameters<typeof isAppShortcut>[0]>) => ({
  key: "a",
  ctrlKey: false,
  shiftKey: false,
  altKey: false,
  metaKey: false,
  ...over,
})

describe("isNewWorkspaceChord", () => {
  it("matches Ctrl+Shift+N and Cmd+Shift+N", () => {
    expect(isNewWorkspaceChord(key({ key: "N", ctrlKey: true, shiftKey: true }))).toBe(true)
    expect(isNewWorkspaceChord(key({ key: "n", metaKey: true, shiftKey: true }))).toBe(true)
  })

  it("leaves Ctrl+N to the shell", () => {
    expect(isNewWorkspaceChord(key({ key: "n", ctrlKey: true }))).toBe(false)
  })
})

describe("isNewTerminalChord", () => {
  it("matches Ctrl+Shift+T", () => {
    expect(isNewTerminalChord(key({ key: "T", ctrlKey: true, shiftKey: true }))).toBe(true)
  })

  it("ignores the chord when Alt is held", () => {
    expect(isNewTerminalChord(key({ key: "t", ctrlKey: true, shiftKey: true, altKey: true }))).toBe(
      false,
    )
  })
})

describe("isSettingsChord", () => {
  it("matches Ctrl+comma", () => {
    expect(isSettingsChord(key({ key: ",", ctrlKey: true }))).toBe(true)
  })

  it("does not fire with Shift", () => {
    expect(isSettingsChord(key({ key: ",", ctrlKey: true, shiftKey: true }))).toBe(false)
  })
})

describe("isHelpChord", () => {
  it("matches a bare F1", () => {
    expect(isHelpChord(key({ key: "F1" }))).toBe(true)
  })

  it("does not fire with a modifier", () => {
    expect(isHelpChord(key({ key: "F1", ctrlKey: true }))).toBe(false)
  })
})

describe("workspaceIndexChord", () => {
  it("maps Ctrl+1 … Ctrl+9 to zero-based positions", () => {
    expect(workspaceIndexChord(key({ key: "1", ctrlKey: true }))).toBe(0)
    expect(workspaceIndexChord(key({ key: "9", ctrlKey: true }))).toBe(8)
    expect(workspaceIndexChord(key({ key: "3", metaKey: true }))).toBe(2)
  })

  it("leaves Ctrl+0 to the text-size reset", () => {
    expect(workspaceIndexChord(key({ key: "0", ctrlKey: true }))).toBe(null)
  })

  it("ignores a bare digit and digits with Shift or Alt", () => {
    expect(workspaceIndexChord(key({ key: "1" }))).toBe(null)
    expect(workspaceIndexChord(key({ key: "1", ctrlKey: true, shiftKey: true }))).toBe(null)
    expect(workspaceIndexChord(key({ key: "1", ctrlKey: true, altKey: true }))).toBe(null)
  })

  it("counts as an app shortcut so the terminal hands it back", () => {
    expect(isAppShortcut(key({ key: "2", ctrlKey: true }))).toBe(true)
    expect(isAppShortcut(key({ key: "2" }))).toBe(false)
  })
})

describe("isAppShortcut", () => {
  it("is true for any window-owned chord", () => {
    expect(isAppShortcut(key({ key: "F1" }))).toBe(true)
    expect(isAppShortcut(key({ key: ",", ctrlKey: true }))).toBe(true)
  })

  it("is false for a plain letter the shell should see", () => {
    expect(isAppShortcut(key({ key: "a" }))).toBe(false)
  })
})
