import * as logic from "./update"
import type { UpdateState } from "./update"

export type { UpdateStatus, UpdateState } from "./update"

/** Reactive update store. Mirrors the font store split: the reactive `$state`
 *  object lives here (`.svelte.ts`, compiled with runes), while the pure
 *  transition logic lives in `./update` (unit-tested without the Svelte
 *  plugin). A plain object here would NOT re-render components on mutation —
 *  it must be `$state`. */
export const update = $state<UpdateState>(logic.initialState())

export const runCheck = (silent: boolean): Promise<void> => logic.runCheck(update, silent)
export const runDownload = (): Promise<string | null> => logic.runDownload(update)
export const runInstall = (path: string): Promise<void> => logic.runInstall(update, path)
export const dismiss = (): void => logic.dismiss(update)
