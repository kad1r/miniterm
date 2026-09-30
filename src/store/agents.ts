import type { Node, Workspace } from "./types"
import { terminalCount } from "./relayout"
import type { Locale } from "../i18n/messages"

/** One entry per cell, in cell order: the agent name that pane answers to. */
export type PaneNames = (string | null)[]

/**
 * Default names, handed out in order and never reused across workspaces, so
 * "Athena" always means one pane. Greek gods first, Norse after; ASCII
 * spellings because the name is also typed, spoken and used as a file stem.
 */
export const GREEK_GODS = [
  "Zeus", "Hera", "Athena", "Apollo", "Artemis", "Hermes", "Poseidon", "Ares",
  "Aphrodite", "Hephaestus", "Demeter", "Dionysus", "Hestia", "Hades", "Persephone",
  "Helios", "Selene", "Eos", "Nike", "Iris", "Hebe", "Eros", "Pan", "Hecate",
  "Nemesis", "Tyche", "Themis", "Gaia", "Rhea", "Kronos", "Atlas", "Prometheus",
  "Hyperion", "Mnemosyne", "Morpheus", "Hypnos", "Nyx", "Asclepius", "Aeolus", "Harmonia",
] as const

export const NORSE_GODS = [
  "Odin", "Thor", "Loki", "Freya", "Frigg", "Baldur", "Tyr", "Heimdall", "Freyr",
  "Njord", "Idun", "Bragi", "Skadi", "Vidar", "Vali", "Sif", "Ullr", "Forseti",
  "Hodr", "Eir", "Saga", "Mimir", "Mani", "Sol", "Hel",
] as const

export const AGENT_NAMES: readonly string[] = [...GREEK_GODS, ...NORSE_GODS]

export const MAX_NAME = 24

/**
 * Clean a typed name: letters (any script), digits, `-` and `_` only. No
 * spaces or dots — the name doubles as a file stem (`Zeus.log`) and as a
 * word you address an agent with. Returns "" when nothing usable is left.
 */
export function cleanName(raw: string): string {
  return raw.trim().replace(/\s+/g, "-").replace(/[^\p{L}\p{N}_-]/gu, "").slice(0, MAX_NAME)
}

const key = (s: string) => s.toLocaleLowerCase("tr")

/** The n-th default: the pool in order, then the pool again as Zeus2, Hera2… */
function defaultName(n: number): string {
  const round = Math.floor(n / AGENT_NAMES.length)
  const base = AGENT_NAMES[n % AGENT_NAMES.length]
  return round === 0 ? base : `${base}${round + 1}`
}

/** Pass one: keep each stored name that is valid and still free. Gaps are "". */
function claimStored(stored: PaneNames, count: number, taken: Set<string>): string[] {
  return Array.from({ length: count }, (_, i) => {
    const n = cleanName(stored[i] ?? "")
    if (!n || taken.has(key(n))) return ""
    taken.add(key(n))
    return n
  })
}

/** Pass two: give every gap the next free default. */
function fillGaps(kept: string[], taken: Set<string>): string[] {
  let next = 0
  return kept.map((name) => {
    if (name) return name
    let candidate = defaultName(next++)
    while (taken.has(key(candidate))) candidate = defaultName(next++)
    taken.add(key(candidate))
    return candidate
  })
}

/**
 * The per-cell names, always exactly one name per cell, none of them in
 * `taken` (lower-cased names owned elsewhere) or repeated. Same read-through
 * pattern as `toolsFor`: an older config has no names at all, and the list
 * has to track the grid's size. `taken` is filled in as it goes.
 */
export function resolveNames(stored: PaneNames, count: number, taken = new Set<string>()): string[] {
  return fillGaps(claimStored(stored, count, taken), taken)
}

function workspaces(tree: Node[], out: Workspace[] = []): Workspace[] {
  for (const n of tree) {
    if (n.kind === "workspace") out.push(n)
    else workspaces(n.children, out)
  }
  return out
}

/**
 * Names for every workspace, unique across the whole app. On a clash between
 * two stored names the workspace higher in the sidebar keeps it and the later
 * one is handed a fresh god.
 */
export function resolveTree(tree: Node[]): Map<string, string[]> {
  const taken = new Set<string>()
  const all = workspaces(tree)
  // Every stored name claims its spot before any default is handed out, so a
  // workspace that has never been opened can never take a name that another
  // one already owns on disk.
  const kept = all.map((ws) => claimStored(ws.paneNames ?? [], terminalCount(ws), taken))
  return new Map(all.map((ws, i) => [ws.id, fillGaps(kept[i], taken)]))
}

export function namesFor(ws: Workspace, tree: Node[]): string[] {
  const resolved = resolveTree(tree).get(ws.id)
  // The count check covers a caller holding a newer shape than the tree.
  if (resolved && resolved.length === terminalCount(ws)) return resolved
  return resolveNames(ws.paneNames ?? [], terminalCount(ws), takenOutside(tree, ws.id))
}

/** Lower-cased names owned by every workspace except `workspaceId`. */
export function takenOutside(tree: Node[], workspaceId: string): Set<string> {
  const taken = new Set<string>()
  for (const [id, names] of resolveTree(tree)) {
    if (id !== workspaceId) for (const n of names) taken.add(key(n))
  }
  return taken
}

export type RenameResult = { names: string[] } | { error: "empty" | "taken" }

/** Rename one cell. Names are unique across all workspaces, ignoring case;
 *  `taken` holds the lower-cased names of the other workspaces. */
export function renameAt(
  names: string[], index: number, raw: string, taken: Set<string> = new Set(),
): RenameResult {
  const name = cleanName(raw)
  if (!name) return { error: "empty" }
  if (taken.has(key(name)) || names.some((n, i) => i !== index && key(n) === key(name))) {
    return { error: "taken" }
  }
  const out = names.slice()
  out[index] = name
  return { names: out }
}

/** localStorage flag for the one-off reset below. */
export const NAMES_RESET_KEY = "miniterm.agentNames.v2"

/**
 * Forget every stored agent name. Used once, on the first start after the
 * switch to app-wide unique god names: names pinned by the earlier per-workspace
 * pool (Atlas, Nova, Orion… repeated in every workspace) would otherwise stay.
 * Returns the ids of the workspaces that had names, so their stale shared
 * folders can go too.
 */
export function clearPaneNames(tree: Node[]): { tree: Node[]; cleared: string[] } {
  const cleared: string[] = []
  const walk = (nodes: Node[]): Node[] =>
    nodes.map((n) => {
      if (n.kind === "folder") return { ...n, children: walk(n.children) }
      if (!n.paneNames || n.paneNames.length === 0) return n
      cleared.push(n.id)
      return { ...n, paneNames: [] }
    })
  return { tree: walk(tree), cleared }
}

export function sameNames(a: PaneNames | undefined, b: string[]): boolean {
  return !!a && a.length === b.length && a.every((n, i) => n === b[i])
}

export const logFile = (name: string) => `${name}.log`
export const inboxFile = (name: string) => `inbox/${name}.md`
export const ROSTER = "roster.md"

export type AgentInfo = { name: string; tool: string; status: "running" | "exited" | "starting" | "minimized" }

function join(dir: string, rel: string): string {
  const sep = dir.includes("\\") ? "\\" : "/"
  return `${dir}${sep}${rel.replace(/\//g, sep)}`
}

/**
 * The shared `roster.md`. Written in English on purpose: it is read by the
 * agents, not by the user, and every tool reads English instructions well.
 */
export function rosterMarkdown(workspace: string, dir: string, agents: AgentInfo[]): string {
  const rows = agents
    .map((a) => `| ${a.name} | ${a.tool} | ${a.status} | ${join(dir, logFile(a.name))} | ${join(dir, inboxFile(a.name))} |`)
    .join("\n")
  return `# Agents in workspace "${workspace}"

Managed by miniterm. Each agent is one terminal pane in this workspace. Your own
name is in the MINITERM_AGENT environment variable (or was told to you by the user).

| Agent | Tool | Status | Recent output | Inbox |
|---|---|---|---|---|
${rows}

## How to collaborate

- **See what another agent is doing:** read its \`<Name>.log\`. It holds the last
  few hundred lines of that pane's terminal, refreshed every few seconds.
- **Send a message:** append to \`inbox/<Name>.md\` — never overwrite. Start each
  message with \`## From <YourName> — <time>\`.
- **Check your own inbox** when the user asks, or before starting related work.
- The user may address any agent by name; if a request is for someone else,
  leave it in their inbox instead of doing it yourself.
`
}

/** One line, so pasting it into an agent's prompt never submits early. */
export function introText(
  locale: Locale, name: string, workspace: string, dir: string, others: string[],
): string {
  const roster = join(dir, ROSTER)
  const team = others.length > 0 ? others.join(", ") : "-"
  if (locale === "tr") {
    return `Senin adın ${name}. miniterm'deki "${workspace}" workspace'inde şu ajanlarla birlikte çalışıyorsun: ${team}. ` +
      `Önce ${roster} dosyasını oku: diğer ajanların son terminal çıktısı <İsim>.log dosyalarında, ` +
      `mesajlar inbox\\<İsim>.md dosyalarında. Bir ajana mesaj bırakmak için onun inbox dosyasına ekle. ` +
      `"${name}" diye seslenildiğinde sana hitap ediliyor.`
  }
  return `Your name is ${name}. You work in the miniterm workspace "${workspace}" alongside: ${team}. ` +
    `First read ${roster}: the other agents' recent terminal output is in <Name>.log, ` +
    `messages are in inbox/<Name>.md. To message an agent, append to its inbox file. ` +
    `When someone says "${name}", they mean you.`
}

/** Header + trimmed terminal text for `<Name>.log`. */
export function logText(name: string, lines: string[], at: Date): string {
  let end = lines.length
  while (end > 0 && lines[end - 1].trim() === "") end--
  const body = lines.slice(0, end).map((l) => l.trimEnd()).join("\n")
  return `# ${name} — terminal output, updated ${at.toISOString()}\n\n${body}\n`
}
