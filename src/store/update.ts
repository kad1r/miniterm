import { checkUpdate, downloadUpdate, installUpdate, onUpdateProgress, type Progress, type UpdateInfo } from "../ipc"

export type UpdateStatus =
  | "idle" | "checking" | "upToDate" | "available" | "downloading" | "ready" | "error"

export type UpdateState = {
  status: UpdateStatus
  info: UpdateInfo | null
  progress: Progress | null
  error: string | null
}

export function initialState(): UpdateState {
  return { status: "idle", info: null, progress: null, error: null }
}

function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}

// Pure transition logic, kept out of the `.svelte.ts` store so it can be
// unit-tested without the Svelte compiler (vitest has no Svelte plugin). Each
// function mutates the passed-in state; the reactive store in update.svelte.ts
// binds these to a `$state` object so component reads stay reactive. Same split
// as font.ts (logic) / font.svelte.ts (reactive store).

/** Check GitHub. `silent` = startup path: swallow errors and up-to-date quietly. */
export async function runCheck(s: UpdateState, silent: boolean): Promise<void> {
  s.status = "checking"
  s.error = null
  try {
    const info = await checkUpdate()
    s.info = info
    if (info.isNewer) s.status = "available"
    else s.status = silent ? "idle" : "upToDate"
  } catch (e) {
    if (silent) {
      s.status = "idle"
    } else {
      s.status = "error"
      s.error = message(e)
    }
  }
}

/** Download the available update, reporting progress; leaves status "ready". */
export async function runDownload(s: UpdateState): Promise<string | null> {
  if (!s.info) return null
  s.status = "downloading"
  s.progress = null
  const stop = await onUpdateProgress((p) => (s.progress = p))
  try {
    const path = await downloadUpdate(s.info.downloadUrl)
    s.status = "ready"
    return path
  } catch (e) {
    s.status = "error"
    s.error = message(e)
    return null
  } finally {
    stop()
  }
}

/** Launch the installer; the app exits on success so this never resolves in practice. */
export async function runInstall(s: UpdateState, path: string): Promise<void> {
  try {
    await installUpdate(path)
  } catch (e) {
    s.status = "error"
    s.error = message(e)
  }
}

export function dismiss(s: UpdateState): void {
  s.status = "idle"
  s.info = null
  s.progress = null
  s.error = null
}
