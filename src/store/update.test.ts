import { beforeEach, describe, expect, it, vi } from "vitest"

const ipc = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  downloadUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdateProgress: vi.fn(async () => () => {}),
}))
vi.mock("../ipc", () => ipc)

import { update, runCheck, dismiss } from "./update.svelte"

const info = (isNewer: boolean) => ({
  current: "0.9.0", latest: isNewer ? "0.10.0" : "0.9.0",
  isNewer, downloadUrl: "https://github.com/x.exe", notes: "notes",
})

beforeEach(() => {
  vi.clearAllMocks()
  dismiss()
})

describe("runCheck", () => {
  it("goes available when a newer release exists", async () => {
    ipc.checkUpdate.mockResolvedValue(info(true))
    await runCheck(false)
    expect(update.status).toBe("available")
    expect(update.info?.latest).toBe("0.10.0")
  })

  it("goes upToDate when current is latest", async () => {
    ipc.checkUpdate.mockResolvedValue(info(false))
    await runCheck(false)
    expect(update.status).toBe("upToDate")
  })

  it("stays idle on silent error", async () => {
    ipc.checkUpdate.mockRejectedValue(new Error("offline"))
    await runCheck(true)
    expect(update.status).toBe("idle")
    expect(update.error).toBeNull()
  })

  it("records error on non-silent failure", async () => {
    ipc.checkUpdate.mockRejectedValue(new Error("offline"))
    await runCheck(false)
    expect(update.status).toBe("error")
    expect(update.error).toBe("offline")
  })
})
