import { app, commit, notify } from "./app.svelte"
import { t } from "../i18n/locale.svelte"
import { findNode, updateWorkspace } from "./tree"
import { defaultLayoutFor, relayout, terminalCount } from "./relayout"
import { focusAfterClose } from "./focus"
import { touch, LIVE_LIMIT } from "./lru"
import { killSession, onSessionExit, spawnSession } from "../ipc"
import type { Node, ShellInfo, Workspace } from "./types"
import { paneStatus } from "./status"

export type PaneStatus = "off" | "running" | "dead"

interface WorkspaceSessions {
  ids: (number | null)[]
  exits: (number | null | undefined)[]
  /** Which cell had the keyboard last. Deliberately session-scoped rather than
   *  stored in config.json: a focus change happens on every click into a pane,
   *  and routing that through commit() would queue a disk write each time. */
  focused: number
}

export const sessions = $state({
  byWorkspace: {} as Record<string, WorkspaceSessions>,
  live: [] as string[],
})

/** sessionId -> hangi workspace'in kaçıncı hücresi. session-exit için gerekli. */
const owner = new Map<number, { workspaceId: string; index: number }>()

function workspaceById(id: string): Workspace | null {
  const node = findNode(app.config.tree, id)
  return node && node.kind === "workspace" ? node : null
}

function shellFor(ws: Workspace): ShellInfo | null {
  const wanted = ws.shellId ?? app.config.defaultShellId
  return app.shells.find((s) => s.id === wanted) ?? app.shells[0] ?? null
}

function commandFor(ws: Workspace): string | null {
  if (!ws.aiToolId) return null
  return app.config.aiTools.find((t) => t.id === ws.aiToolId)?.command ?? null
}

async function spawnOne(ws: Workspace, index: number, shell: ShellInfo): Promise<number | null> {
  try {
    const id = await spawnSession({
      cwd: ws.path,
      program: shell.program,
      args: shell.args,
      initialCommand: commandFor(ws),
      cols: 80,
      rows: 24,
    })
    owner.set(id, { workspaceId: ws.id, index })
    return id
  } catch (err) {
    notify(t("sessions.spawnFailed", { path: ws.path, error: String(err) }), "error")
    return null
  }
}

export async function activate(workspaceId: string): Promise<void> {
  const ws = workspaceById(workspaceId)
  if (!ws) return

  const count = terminalCount(ws)
  const existing = sessions.byWorkspace[workspaceId]
  if (existing && existing.ids.length === count) {
    // Already fully spawned — just update LRU order.
    const moved = touch(sessions.live, workspaceId, LIVE_LIMIT)
    sessions.live = moved.list
    return
  }

  const shell = shellFor(ws)
  if (!shell) {
    notify(t("sessions.noShell"), "error")
    return
  }

  // Move into the LRU only after confirming we have a shell to spawn.
  // A workspace with no shell must not consume one of the three LRU slots.
  const moved = touch(sessions.live, workspaceId, LIVE_LIMIT)
  sessions.live = moved.list
  // Tahliye edilenlerin süreçleri yaşar; sadece xterm örnekleri kaldırılır.

  // Yerleşim değiştiyse fazla panelleri kapat, eksikleri aç.
  // This is how "add a terminal" lands: LayoutDialog writes the new rows/cols and
  // calls activate() again. Exits are carried forward so surviving dead panes keep
  // their status dots instead of resetting to "running".
  const ids: (number | null)[] = existing ? existing.ids.slice(0, count) : []
  if (existing) {
    for (const extra of existing.ids.slice(count)) {
      if (extra !== null) {
        owner.delete(extra)
        void killSession(extra)
      }
    }
  }
  sessions.byWorkspace[workspaceId] = {
    ids: [...ids, ...Array(count - ids.length).fill(null)],
    exits: [
      ...(existing?.exits.slice(0, count) ?? []),
      ...Array(count - ids.length).fill(undefined),
    ],
    focused: Math.min(existing?.focused ?? 0, count - 1),
  }

  // Capture the slot reference before the first await. If the slot is replaced
  // (e.g. the workspace is deleted mid-spawn), compare by reference and clean up
  // the just-spawned session rather than orphaning it. The length check catches
  // the other race: closePane() splices the same object in place, so the slot for
  // index i may no longer exist by the time its session is ready.
  const mine = sessions.byWorkspace[workspaceId]
  for (let i = ids.length; i < count; i++) {
    const id = await spawnOne(ws, i, shell)
    if (sessions.byWorkspace[workspaceId] !== mine || mine.ids.length !== count) {
      if (id !== null) {
        owner.delete(id)
        void killSession(id).catch(() => {})
      }
      return
    }
    mine.ids[i] = id
    if (id === null) mine.exits[i] = null
  }
}

export async function restart(workspaceId: string, index: number): Promise<void> {
  const ws = workspaceById(workspaceId)
  const slot = sessions.byWorkspace[workspaceId]
  if (!ws || !slot) return
  const shell = shellFor(ws)
  if (!shell) return

  const old = slot.ids[index]
  if (old !== null && old !== undefined) {
    owner.delete(old)
    // R17.2: Kill the dead session to free Rust-side entry and ring buffer.
    killSession(old).catch(() => {})
  }
  slot.ids[index] = null
  slot.exits[index] = undefined

  const id = await spawnOne(ws, index, shell)
  slot.ids[index] = id
  if (id === null) slot.exits[index] = null
}

/** Close one pane and let the grid close ranks behind it.
 *
 *  The panes after the closed one shift down a cell, so the owner map has to be
 *  rewritten for them — it is what routes a session-exit event to a cell, and a
 *  stale index would paint the wrong pane dead. The grid itself is derived from
 *  rows × cols, so the new shape is committed to the config; App re-renders from
 *  that, and activate() is not needed because the ids array already matches the
 *  new count. The last pane stays: a workspace with no terminal has nothing to
 *  show, and deleting the workspace is a different action. */
export async function closePane(workspaceId: string, index: number): Promise<void> {
  const ws = workspaceById(workspaceId)
  const slot = sessions.byWorkspace[workspaceId]
  if (!ws || !slot) return
  const count = terminalCount(ws)
  if (count <= 1 || index < 0 || index >= count) return

  const id = slot.ids[index]
  slot.ids.splice(index, 1)
  slot.exits.splice(index, 1)
  slot.focused = focusAfterClose(slot.focused, index, slot.ids.length)
  for (let i = index; i < slot.ids.length; i++) {
    const moved = slot.ids[i]
    if (moved !== null && moved !== undefined) owner.set(moved, { workspaceId, index: i })
  }

  const patch = relayout(ws, defaultLayoutFor(ws, count - 1))
  commit((c) => ({ ...c, tree: updateWorkspace(c.tree, workspaceId, patch) }))

  if (id !== null && id !== undefined) {
    owner.delete(id)
    await killSession(id).catch(() => {})
  }
}

export async function closeSubtree(node: Node): Promise<void> {
  if (node.kind === "folder") {
    for (const child of node.children) await closeSubtree(child)
    return
  }
  // Remove from LRU before the slot check: a workspace deleted before it ever
  // spawned (no slot) would otherwise remain in live and cap the app at two grids.
  sessions.live = sessions.live.filter((x) => x !== node.id)
  const slot = sessions.byWorkspace[node.id]
  if (!slot) return
  for (const id of slot.ids) {
    if (id !== null) {
      owner.delete(id)
      await killSession(id).catch(() => {})
    }
  }
  delete sessions.byWorkspace[node.id]
}

/** The cell a workspace should hand the keyboard to when it comes back on screen. */
export function focusedPaneOf(workspaceId: string): number {
  return sessions.byWorkspace[workspaceId]?.focused ?? 0
}

export function rememberFocus(workspaceId: string, index: number): void {
  const slot = sessions.byWorkspace[workspaceId]
  if (!slot || index < 0 || index >= slot.ids.length) return
  slot.focused = index
}

export function statusOf(workspaceId: string): PaneStatus {
  return paneStatus(sessions.byWorkspace[workspaceId])
}

export async function listenExits(): Promise<void> {
  await onSessionExit(({ id, code }) => {
    const where = owner.get(id)
    if (!where) return
    owner.delete(id)
    const slot = sessions.byWorkspace[where.workspaceId]
    if (!slot) return
    // Deliberately does NOT null slot.ids[index] here. Only kill() removes the Rust map
    // entry (src-tauri/src/pty/mod.rs:394), so a nulled id would make closeSubtree skip it
    // and leak the session plus its 256 KB ring buffer. TerminalPane guards keystrokes on
    // exitCode, not on the id — see TerminalPane.svelte onData handler.
    slot.exits[where.index] = code
  })
}
