import { app, notify } from "./app.svelte"
import { findNode } from "./tree"
import { touch, LIVE_LIMIT } from "./lru"
import { killSession, onSessionExit, spawnSession } from "../ipc"
import type { Node, ShellInfo, Workspace } from "./types"
import { paneStatus } from "./status"

export type PaneStatus = "off" | "running" | "dead"

interface WorkspaceSessions {
  ids: (number | null)[]
  exits: (number | null | undefined)[]
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
    notify(`Terminal açılamadı (${ws.path}): ${String(err)}`, "error")
    return null
  }
}

export async function activate(workspaceId: string): Promise<void> {
  const ws = workspaceById(workspaceId)
  if (!ws) return

  const count = ws.rows * ws.cols
  const existing = sessions.byWorkspace[workspaceId]
  if (existing && existing.ids.length === count) {
    // Already fully spawned — just update LRU order.
    const moved = touch(sessions.live, workspaceId, LIVE_LIMIT)
    sessions.live = moved.list
    return
  }

  const shell = shellFor(ws)
  if (!shell) {
    notify("Kullanılabilir shell bulunamadı", "error")
    return
  }

  // Move into the LRU only after confirming we have a shell to spawn.
  // A workspace with no shell must not consume one of the three LRU slots.
  const moved = touch(sessions.live, workspaceId, LIVE_LIMIT)
  sessions.live = moved.list
  // Tahliye edilenlerin süreçleri yaşar; sadece xterm örnekleri kaldırılır.

  // Yerleşim değiştiyse fazla panelleri kapat, eksikleri aç.
  // NOTE: This branch is currently unreachable — layout (rows/cols) is fixed at workspace
  // creation and never mutated. It is kept as the natural seam for a future layout-editing
  // feature; carry exits forward so surviving dead panes keep their status dots.
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
  }

  // Capture the slot reference before the first await. If the slot is replaced
  // (e.g. the workspace is deleted mid-spawn), compare by reference and clean up
  // the just-spawned session rather than orphaning it.
  const mine = sessions.byWorkspace[workspaceId]
  for (let i = ids.length; i < count; i++) {
    const id = await spawnOne(ws, i, shell)
    if (sessions.byWorkspace[workspaceId] !== mine) {
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
