import { checkUpdate, downloadUpdate, installUpdate, onUpdateProgress, type Progress, type UpdateInfo } from "../ipc"

export type UpdateStatus =
  | "idle" | "checking" | "upToDate" | "available" | "downloading" | "ready" | "error"

// Plain object backing so this module is testable under vitest (no Svelte plugin
// in vitest.config.ts → $state runes are not transformed in tests).
// Components read the same properties reactively via Svelte 5 fine-grained
// reactivity when the object is declared with $state in a .svelte file context;
// the store logic and public API are identical either way.
export const update: {
  status: UpdateStatus
  info: UpdateInfo | null
  progress: Progress | null
  error: string | null
} = { status: "idle", info: null, progress: null, error: null }

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

/** Check GitHub. `silent` = startup path: swallow errors and up-to-date quietly. */
export async function runCheck(silent: boolean): Promise<void> {
  update.status = "checking"
  update.error = null
  try {
    const info = await checkUpdate()
    update.info = info
    if (info.isNewer) update.status = "available"
    else update.status = silent ? "idle" : "upToDate"
  } catch (e) {
    if (silent) {
      update.status = "idle"
    } else {
      update.status = "error"
      update.error = message(e)
    }
  }
}

/** Download the available update, reporting progress; leaves status "ready". */
export async function runDownload(): Promise<string | null> {
  if (!update.info) return null
  update.status = "downloading"
  update.progress = null
  const stop = await onUpdateProgress((p) => (update.progress = p))
  try {
    const path = await downloadUpdate(update.info.downloadUrl)
    update.status = "ready"
    return path
  } catch (e) {
    update.status = "error"
    update.error = message(e)
    return null
  } finally {
    stop()
  }
}

/** Launch the installer; the app exits on success so this never resolves in practice. */
export async function runInstall(path: string): Promise<void> {
  try {
    await installUpdate(path)
  } catch (e) {
    update.status = "error"
    update.error = message(e)
  }
}

export function dismiss(): void {
  update.status = "idle"
  update.info = null
  update.progress = null
  update.error = null
}
