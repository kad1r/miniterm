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

export function commandPreview(shell: ShellInfo | null, command: string): string {
  const shellPart = shell ? [shell.program, ...shell.args].join(" ") : "(shell seçilmedi)"
  const cmd = command.trim()
  // Komut asla argüman olarak geçirilmez; etkileşimli shell'in stdin'ine yazılır (§6.1).
  return cmd === ""
    ? `${shellPart} açılır, komut çalıştırılmaz`
    : `${shellPart} açılır, ardından "${cmd}" yazılır`
}

export function toolUsage(tree: Node[], toolId: string): number {
  return tree.reduce(
    (n, node) =>
      node.kind === "folder"
        ? n + toolUsage(node.children, toolId)
        : n + (node.aiToolId === toolId ? 1 : 0),
    0,
  )
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
    tree: mapWorkspaces(config.tree, (ws) =>
      ws.aiToolId === toolId ? { ...ws, aiToolId: null } : ws,
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
