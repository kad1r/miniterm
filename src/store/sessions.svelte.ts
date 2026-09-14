import { app, commit, notify } from "./app.svelte"
import { t } from "../i18n/locale.svelte"
import { findNode, updateWorkspace } from "./tree"
import { defaultLayoutFor, relayout, terminalCount } from "./relayout"
import { MAX_TERMINALS } from "./layout"
import { focusAfterClose, maximizedAfterClose } from "./focus"
import { touch, LIVE_LIMIT } from "./lru"
import { killSession, onSessionExit, spawnSession } from "../ipc"
import type { Node, ShellInfo, Workspace } from "./types"
import { toolsFor } from "./panes"
import { paneStatus } from "./status"

export type PaneStatus = "off" | "running" | "dead"

/** A terminal that was sent to the strip below the grid: off the grid, still
 *  running. It keeps its own exit code because a minimized shell can die while
 *  it is down there. */
export interface MinimizedPane {
  id: number | null
  exit: number | null | undefined
  /** The AI tool of the cell it came from, carried down so restoring puts it
   *  back the way it was instead of silently turning into a bare shell. */
  toolId: string | null
}

interface WorkspaceSessions {
  ids: (number | null)[]
  exits: (number | null | undefined)[]
  /** Which cell had the keyboard last. Deliberately session-scoped rather than
   *  stored in config.json: a focus change happens on every click into a pane,
   *  and routing that through commit() would queue a disk write each time. */
  focused: number
  /** Minimized terminals, in the order they were sent down. Session-scoped for
   *  the same reason as `focused`, and because the shells do not outlive the app
   *  anyway — there is nothing for config.json to restore. */
  minimized: MinimizedPane[]
  /** Which cell is blown up to fill the workspace, or null for the whole grid. */
  maximized: number | null
}

export const sessions = $state({
  byWorkspace: {} as Record<string, WorkspaceSessions>,
  live: [] as string[],
})

/** sessionId -> hangi workspace. session-exit olayını yönlendirmek için gerekli.
 *  Deliberately not the cell index: closing, minimizing and restoring all move
 *  panes between cells, and a remembered index would go stale and paint the
 *  wrong pane dead. The position is looked up from the arrays when needed. */
const owner = new Map<number, string>()

function workspaceById(id: string): Workspace | null {
  const node = findNode(app.config.tree, id)
  return node && node.kind === "workspace" ? node : null
}

function shellFor(ws: Workspace): ShellInfo | null {
  const wanted = ws.shellId ?? app.config.defaultShellId
  return app.shells.find((s) => s.id === wanted) ?? app.shells[0] ?? null
}

/** The command a given cell opens with. Per-cell rather than per-workspace: one
 *  grid can run Claude in two panes, Gemini in a third and a bare shell in a
 *  fourth. A tool id that no longer exists falls through to a bare shell. */
function commandFor(ws: Workspace, index: number): string | null {
  const toolId = toolsFor(ws)[index] ?? null
  if (!toolId) return null
  return app.config.aiTools.find((t) => t.id === toolId)?.command ?? null
}

async function spawnOne(ws: Workspace, shell: ShellInfo, index: number): Promise<number | null> {
  try {
    const id = await spawnSession({
      cwd: ws.path,
      program: shell.program,
      args: shell.args,
      initialCommand: commandFor(ws, index),
      cols: 80,
      rows: 24,
    })
    owner.set(id, ws.id)
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
    // Minimized terminals are not part of the grid, so a relayout leaves them
    // alone. A maximized cell that no longer exists is dropped.
    minimized: existing?.minimized ?? [],
    maximized:
      existing?.maximized !== undefined && existing.maximized !== null && existing.maximized < count
        ? existing.maximized
        : null,
  }

  // Capture the slot reference before the first await. If the slot is replaced
  // (e.g. the workspace is deleted mid-spawn), compare by reference and clean up
  // the just-spawned session rather than orphaning it. The length check catches
  // the other race: closePane() splices the same object in place, so the slot for
  // index i may no longer exist by the time its session is ready.
  const mine = sessions.byWorkspace[workspaceId]
  for (let i = ids.length; i < count; i++) {
    const id = await spawnOne(ws, shell, i)
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

  const id = await spawnOne(ws, shell, index)
  slot.ids[index] = id
  if (id === null) slot.exits[index] = null
}

/** Take one pane out of the grid and let the rest close ranks behind it.
 *
 *  The grid is derived from rows × cols, so the new shape is committed to the
 *  config; App re-renders from that, and activate() is not needed because the ids
 *  array already matches the new count. Shared by close and minimize, which
 *  differ only in what happens to the session afterwards. */
function pullPane(workspaceId: string, index: number): MinimizedPane | null {
  const ws = workspaceById(workspaceId)
  const slot = sessions.byWorkspace[workspaceId]
  if (!ws || !slot) return null
  const count = terminalCount(ws)
  if (count <= 1 || index < 0 || index >= count) return null

  const [id] = slot.ids.splice(index, 1)
  const [exit] = slot.exits.splice(index, 1)
  slot.focused = focusAfterClose(slot.focused, index, slot.ids.length)
  slot.maximized = maximizedAfterClose(slot.maximized, index)

  // The tools are indexed by cell, so they have to close ranks with the panes.
  // Splicing before the relayout means every surviving pane keeps its own tool
  // rather than inheriting its neighbour's.
  const paneTools = toolsFor(ws)
  const [toolId] = paneTools.splice(index, 1)

  const patch = { ...relayout(ws, defaultLayoutFor(ws, count - 1)), paneTools }
  commit((c) => ({ ...c, tree: updateWorkspace(c.tree, workspaceId, patch) }))
  return { id: id ?? null, exit, toolId: toolId ?? null }
}

/** Close one pane. The last pane stays: a workspace with no terminal has nothing
 *  to show, and deleting the workspace is a different action. */
export async function closePane(workspaceId: string, index: number): Promise<void> {
  const pulled = pullPane(workspaceId, index)
  if (!pulled) return
  if (pulled.id !== null) {
    owner.delete(pulled.id)
    await killSession(pulled.id).catch(() => {})
  }
}

/** Send one pane down to the strip below the grid. Same grid bookkeeping as a
 *  close, except the shell keeps running and the session is parked in
 *  `minimized` so it can be brought back. */
export function minimizePane(workspaceId: string, index: number): void {
  const pulled = pullPane(workspaceId, index)
  if (!pulled) return
  sessions.byWorkspace[workspaceId]?.minimized.push(pulled)
}

/** Put a minimized terminal back into the grid, as the last cell.
 *
 *  Returns false when the grid is already at the terminal limit — the caller is
 *  the one that can tell the user why nothing happened. */
export function restorePane(workspaceId: string, stripIndex: number): boolean {
  const ws = workspaceById(workspaceId)
  const slot = sessions.byWorkspace[workspaceId]
  if (!ws || !slot) return false
  if (stripIndex < 0 || stripIndex >= slot.minimized.length) return false
  const count = terminalCount(ws)
  if (count >= MAX_TERMINALS) return false

  const [entry] = slot.minimized.splice(stripIndex, 1)
  slot.ids.push(entry.id)
  slot.exits.push(entry.exit)
  slot.focused = slot.ids.length - 1
  // The user asked to see this terminal, so a pane that was filling the screen
  // would be in the way.
  slot.maximized = null

  // It comes back as the last cell, so its tool goes on the end. The shell is
  // still the one that was spawned; this only keeps the config in step with it.
  const paneTools = [...toolsFor(ws), entry.toolId]

  const patch = { ...relayout(ws, defaultLayoutFor(ws, count + 1)), paneTools }
  commit((c) => ({ ...c, tree: updateWorkspace(c.tree, workspaceId, patch) }))
  return true
}

/** Blow one pane up to fill the workspace, or put the grid back. Purely a view
 *  state: no session is touched and no config is written. */
export function toggleMaximize(workspaceId: string, index: number): void {
  const ws = workspaceById(workspaceId)
  const slot = sessions.byWorkspace[workspaceId]
  if (!ws || !slot) return
  if (index < 0 || index >= terminalCount(ws)) return
  slot.maximized = slot.maximized === index ? null : index
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
  // Minimized terminals are off the grid but their shells and ring buffers are
  // not, so they have to be killed here too or they leak for the whole session.
  for (const id of [...slot.ids, ...slot.minimized.map((m) => m.id)]) {
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
    const workspaceId = owner.get(id)
    if (workspaceId === undefined) return
    owner.delete(id)
    const slot = sessions.byWorkspace[workspaceId]
    if (!slot) return
    // Deliberately does NOT null slot.ids[index] here. Only kill() removes the Rust map
    // entry (src-tauri/src/pty/mod.rs:394), so a nulled id would make closeSubtree skip it
    // and leak the session plus its 256 KB ring buffer. TerminalPane guards keystrokes on
    // exitCode, not on the id — see TerminalPane.svelte onData handler.
    const cell = slot.ids.indexOf(id)
    if (cell !== -1) {
      slot.exits[cell] = code
      return
    }
    const strip = slot.minimized.findIndex((m) => m.id === id)
    if (strip !== -1) slot.minimized[strip].exit = code
  })
}
