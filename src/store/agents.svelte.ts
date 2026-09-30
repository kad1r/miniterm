import { app, commit, notify } from "./app.svelte"
import { t, locale } from "../i18n/locale.svelte"
import { updateWorkspace } from "./tree"
import {
  agentsDir, removeAgentFile, renameAgentFile, writeAgentFile, writeSession,
} from "../ipc"
import type { Workspace } from "./types"
import {
  inboxFile, introText, logFile, namesFor, renameAt, rosterMarkdown, ROSTER, takenOutside,
  type AgentInfo,
} from "./agents"

/** Resolved agent folder per workspace id. Reactive so the pane header can
 *  show the path once it is known. */
export const agentDirs = $state<Record<string, string>>({})

const pending = new Map<string, Promise<string>>()

export function ensureAgentsDir(workspaceId: string): Promise<string> {
  const known = agentDirs[workspaceId]
  if (known) return Promise.resolve(known)
  let p = pending.get(workspaceId)
  if (!p) {
    p = agentsDir(workspaceId).then((dir) => {
      agentDirs[workspaceId] = dir
      pending.delete(workspaceId)
      return dir
    })
    pending.set(workspaceId, p)
  }
  return p
}

/** Rename the agent in one cell. Its log and inbox follow it; the env var of a
 *  process that is already running cannot, which is what the roster is for. */
export function renameAgent(ws: Workspace, index: number, raw: string): boolean {
  const names = namesFor(ws, app.config.tree)
  const old = names[index]
  const res = renameAt(names, index, raw, takenOutside(app.config.tree, ws.id))
  if ("error" in res) {
    notify(t(res.error === "taken" ? "agents.nameTaken" : "agents.nameEmpty"), "error")
    return false
  }
  const next = res.names[index]
  if (next === old) return true
  commit((c) => ({ ...c, tree: updateWorkspace(c.tree, ws.id, { paneNames: res.names }) }))
  void (async () => {
    await renameAgentFile(ws.id, inboxFile(old), inboxFile(next))
    await renameAgentFile(ws.id, logFile(old), logFile(next))
  })().catch(() => {})
  return true
}

/** Type the introduction into the agent's prompt without pressing Enter, so
 *  the user can read it and send it themselves. */
export async function introduceAgent(ws: Workspace, index: number, sessionId: number): Promise<void> {
  const names = namesFor(ws, app.config.tree)
  const dir = await ensureAgentsDir(ws.id)
  const others = names.filter((_, i) => i !== index)
  const text = introText(locale.current, names[index], ws.name, dir, others)
  await writeSession(sessionId, new TextEncoder().encode(text))
}

const lastRoster = new Map<string, string>()

export async function syncRoster(ws: Workspace, agents: AgentInfo[]): Promise<void> {
  const dir = await ensureAgentsDir(ws.id)
  const md = rosterMarkdown(ws.name, dir, agents)
  if (lastRoster.get(ws.id) === md) return
  lastRoster.set(ws.id, md)
  await writeAgentFile(ws.id, ROSTER, md)
}

export function writeAgentLog(workspaceId: string, name: string, text: string): void {
  void writeAgentFile(workspaceId, logFile(name), text).catch(() => {})
}

/** A pane closed for good: its log would only mislead the others. The inbox is
 *  kept — messages the user may still want. */
export function dropAgentLog(workspaceId: string, name: string): void {
  void removeAgentFile(workspaceId, logFile(name)).catch(() => {})
}

/** Tool label for the roster: the AI tool's name, or "shell". */
export function toolLabel(toolId: string | null): string {
  if (!toolId) return "shell"
  return app.config.aiTools.find((x) => x.id === toolId)?.name ?? "shell"
}
