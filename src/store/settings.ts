import { translate, type Locale } from "../i18n/messages"
import { clearTool, toolsFor } from "./panes"
import type { AiTool, Config, DirAlias, Node, ShellInfo, Workspace } from "./types"

export const RECENT_LIMIT = 20

export function pushRecent(recents: string[], path: string): string[] {
  return [path, ...recents.filter((p) => p !== path)].slice(0, RECENT_LIMIT)
}

function mapWorkspaces(tree: Node[], fn: (ws: Workspace) => Workspace): Node[] {
  return tree.map((node) =>
    node.kind === "folder"
      ? { ...node, children: mapWorkspaces(node.children, fn) }
      : fn(node),
  )
}

export function commandPreview(shell: ShellInfo | null, command: string, locale: Locale): string {
  const shellPart = shell
    ? [shell.program, ...shell.args].join(" ")
    : translate(locale, "preview.noShell")
  const cmd = command.trim()
  // Komut asla argüman olarak geçirilmez; etkileşimli shell'in stdin'ine yazılır (§6.1).
  return cmd === ""
    ? translate(locale, "preview.noCommand", { shell: shellPart })
    : translate(locale, "preview.withCommand", { shell: shellPart, command: cmd })
}

/** How many workspaces deleting this tool would change. Counted per workspace,
 *  not per pane: this feeds the "used in N workspaces" warning, and a workspace
 *  running the tool in three of its four panes is still one workspace. */
export function toolUsage(tree: Node[], toolId: string): number {
  return tree.reduce(
    (n, node) =>
      node.kind === "folder"
        ? n + toolUsage(node.children, toolId)
        : n + (usesTool(node, toolId) ? 1 : 0),
    0,
  )
}

function usesTool(ws: Workspace, toolId: string): boolean {
  return ws.aiToolId === toolId || toolsFor(ws).includes(toolId)
}

export function upsertTool(config: Config, tool: AiTool): Config {
  const exists = config.aiTools.some((t) => t.id === tool.id)
  return {
    ...config,
    aiTools: exists ? config.aiTools.map((t) => (t.id === tool.id ? tool : t)) : [...config.aiTools, tool],
  }
}

export function removeTool(config: Config, toolId: string): Config {
  return {
    ...config,
    aiTools: config.aiTools.filter((t) => t.id !== toolId),
    // Both the workspace default and the per-pane overrides: a dangling id
    // spawns a bare shell anyway, so it is cleared rather than left in config.
    tree: mapWorkspaces(config.tree, (ws) =>
      usesTool(ws, toolId)
        ? {
            ...ws,
            aiToolId: ws.aiToolId === toolId ? null : ws.aiToolId,
            paneTools: clearTool(toolsFor(ws), toolId),
          }
        : ws,
    ),
  }
}

export function upsertDir(config: Config, dir: DirAlias): Config {
  const exists = config.directories.some((d) => d.id === dir.id)
  return {
    ...config,
    directories: exists
      ? config.directories.map((d) => (d.id === dir.id ? dir : d))
      : [...config.directories, dir],
  }
}

export function removeDir(config: Config, dirId: string): Config {
  return { ...config, directories: config.directories.filter((d) => d.id !== dirId) }
}
