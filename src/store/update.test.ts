import { beforeEach, describe, expect, it, vi } from "vitest"

const ipc = vi.hoisted(() => ({
  checkUpdate: vi.fn(),
  downloadUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onUpdateProgress: vi.fn(async () => () => {}),
}))
vi.mock("../ipc", () => ipc)

import { initialState, runCheck, type UpdateState } from "./update"

const info = (isNewer: boolean) => ({
  current: "0.9.0", latest: isNewer ? "0.10.0" : "0.9.0",
  isNewer, downloadUrl: "https://github.com/x.exe", notes: "notes",
})

let state: UpdateState
beforeEach(() => {
  vi.clearAllMocks()
  state = initialState()
})

describe("runCheck", () => {
  it("goes available when a newer release exists", async () => {
    ipc.checkUpdate.mockResolvedValue(info(true))
    await runCheck(state, false)
    expect(state.status).toBe("available")
    expect(state.info?.latest).toBe("0.10.0")
  })

  it("goes upToDate when current is latest", async () => {
    ipc.checkUpdate.mockResolvedValue(info(false))
    await runCheck(state, false)
    expect(state.status).toBe("upToDate")
  })

  it("stays idle on silent error", async () => {
    ipc.checkUpdate.mockRejectedValue(new Error("offline"))
    await runCheck(state, true)
    expect(state.status).toBe("idle")
    expect(state.error).toBeNull()
  })

  it("records error on non-silent failure", async () => {
    ipc.checkUpdate.mockRejectedValue(new Error("offline"))
    await runCheck(state, false)
    expect(state.status).toBe("error")
    expect(state.error).toBe("offline")
  })
})
