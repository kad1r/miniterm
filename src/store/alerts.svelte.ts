/** Whether an agent that needs you raises a Windows toast and/or flashes the
 *  taskbar button. Per-machine preferences, so localStorage like the theme and
 *  font scale rather than config.json. Both default to on. */
const TOAST_KEY = "miniterm.alerts.toast"
const FLASH_KEY = "miniterm.alerts.flash"

function read(key: string): boolean {
  if (typeof localStorage === "undefined") return true
  return localStorage.getItem(key) !== "0"
}

function write(key: string, on: boolean) {
  if (typeof localStorage !== "undefined") localStorage.setItem(key, on ? "1" : "0")
}

export const alerts = $state({ toast: read(TOAST_KEY), flash: read(FLASH_KEY) })

export function setToast(on: boolean) {
  alerts.toast = on
  write(TOAST_KEY, on)
}

export function setFlash(on: boolean) {
  alerts.flash = on
  write(FLASH_KEY, on)
}
