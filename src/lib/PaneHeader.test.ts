// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest"
import { flushSync, mount, unmount } from "svelte"
import PaneHeader from "./PaneHeader.svelte"

let app: Record<string, unknown> | null = null

function render(overrides: Record<string, unknown> = {}) {
  const props = {
    index: 0,
    name: "Zeus",
    folderName: "acme-api",
    path: "C:\\dev\\acme-api",
    dead: false,
    closable: true,
    maximized: false,
    canIntroduce: true,
    onrename: vi.fn(),
    onrenamedone: vi.fn(),
    onintroduce: vi.fn(),
    onminimize: vi.fn(),
    onmaximize: vi.fn(),
    onclose: vi.fn(),
    ...overrides,
  }
  const target = document.createElement("div")
  document.body.appendChild(target)
  app = mount(PaneHeader, { target, props })
  flushSync()
  return { target, props }
}

function key(el: Element, k: string) {
  el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }))
  flushSync()
}

function startRename(target: HTMLElement): HTMLInputElement {
  target.querySelector("button.agent")!.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }))
  flushSync()
  return target.querySelector("input.agent-input") as HTMLInputElement
}

afterEach(() => {
  if (app) unmount(app)
  app = null
  document.body.innerHTML = ""
})

describe("PaneHeader", () => {
  it("shows the agent name and location", () => {
    const { target } = render()
    expect(target.querySelector("button.agent")!.textContent!.trim()).toBe("Zeus")
    expect(target.querySelector(".pane-name")!.textContent).toBe("acme-api")
    expect(target.querySelector(".pane-path")!.textContent).toBe("C:\\dev\\acme-api")
  })

  it("renames on Enter and hands focus back", () => {
    const { target, props } = render()
    const input = startRename(target)
    expect(input.value).toBe("Zeus")
    input.value = "Kratos"
    input.dispatchEvent(new Event("input", { bubbles: true }))
    key(input, "Enter")
    expect(props.onrename).toHaveBeenCalledWith("Kratos")
    expect(props.onrenamedone).toHaveBeenCalledTimes(1)
    expect(target.querySelector("input.agent-input")).toBeNull()
  })

  it("cancels on Escape without renaming, even when the input then blurs", () => {
    const { target, props } = render()
    const input = startRename(target)
    input.value = "Nope"
    input.dispatchEvent(new Event("input", { bubbles: true }))
    key(input, "Escape")
    input.dispatchEvent(new FocusEvent("blur"))
    flushSync()
    expect(props.onrename).not.toHaveBeenCalled()
    expect(props.onrenamedone).toHaveBeenCalledTimes(1)
  })

  it("keeps keys typed while renaming from reaching the grid's shortcuts", () => {
    const { target } = render()
    // The grid listens natively on an ancestor of the header, below the app
    // root where Svelte delegates its own handlers.
    const grid = vi.fn()
    target.querySelector(".pane-header")!.addEventListener("keydown", grid)
    const input = startRename(target)
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "W", ctrlKey: true, shiftKey: true, bubbles: true }))
    key(input, "Enter")
    expect(grid).not.toHaveBeenCalled()
  })

  it("wires the window buttons and disables @ when there is no live session", () => {
    const { target, props } = render({ canIntroduce: false })
    const byLabel = (re: RegExp) =>
      [...target.querySelectorAll("button")].find((b) => re.test(b.getAttribute("aria-label") ?? ""))!
    byLabel(/minimi/i).click()
    byLabel(/maximi/i).click()
    byLabel(/close/i).click()
    expect(props.onminimize).toHaveBeenCalledTimes(1)
    expect(props.onmaximize).toHaveBeenCalledTimes(1)
    expect(props.onclose).toHaveBeenCalledTimes(1)
    expect((target.querySelector("button.intro") as HTMLButtonElement).disabled).toBe(true)
  })

  it("has no window buttons for the last pane", () => {
    const { target } = render({ closable: false })
    expect(target.querySelector(".pane-actions")).toBeNull()
  })
})
