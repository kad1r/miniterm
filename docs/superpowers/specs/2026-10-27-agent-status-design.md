# miniterm Agent Status — Design

**Date:** 2026-10-27
**Status:** Approved 2026-10-27 — hook in global `~/.claude/settings.json`; a seen `done` falls back to `running`
**Covers:** TODO items 1, 2, 3, 5 (`docs/superpowers/plans/2026-10-27-agent-features-todo.md`)

## Goal

Tell the user, at a glance, which agent needs them. Today the sidebar dot only knows
whether the *process* is alive (`off / running / dead`). An agent waiting on a permission
prompt in a hidden workspace looks exactly like one that is busy, so the user has to click
through every workspace to find it.

After this change each pane carries an **agent state**:

| State | Meaning | Source |
|---|---|---|
| `working` | The agent is running a turn, or its subagents still are | hook events |
| `waiting` | The agent asked for permission or input | hook `Notification` |
| `done` | The turn finished | hook `Stop` with no subagents left |
| `active` / `idle` | Fallback for tools without hooks: output in the last 2 s or not | PTY output timing |

Plus an **unread** flag, a **ring** on the pane that needs attention, a **Windows toast**
and a **taskbar flash**.

## Non-goals

- No status for Gemini / Codex beyond the `active / idle` fallback in this package. Codex's
  `notify` hook and Gemini's hooks can feed the same channel later.
- No background-*shell* counting (`run_in_background` Bash). Only subagents are counted —
  see "Subagent counter".
- No clickable toast that jumps to the pane. The notification plugin cannot route a click
  back reliably on Windows; the toast names the workspace and agent instead. "Jump to
  latest unread" is TODO item 4.
- No silent edits to `~/.claude/settings.json`. The hook is installed only from an
  explicit button.

## Key constraint: hidden workspaces have no xterm

Hidden workspaces keep their process but not their xterm.js instance; on return the screen
is redrawn from the Rust ring buffer. So the status cannot be parsed in the frontend's
xterm (`registerOscHandler`) — a waiting agent in a hidden workspace, the main use case,
would never be seen. **Everything is detected in Rust**, which sees every session at all
times, and pushed to the frontend as an event, like `session-exit` today.

## Channel: a JSONL file per session (not OSC)

Harp's hooks write an OSC 777 escape sequence to the terminal. On Windows that sequence
would have to pass through ConPTY, which re-renders the screen and is not guaranteed to
forward unknown OSC sequences. A hook process also has no tty of the pane to write to on
Windows (`/dev/tty` is a Git Bash emulation). Rejected as unverified and fragile.

Chosen: **the hook appends its stdin JSON as one line to a per-session file**.

- Rust sets two new environment variables on every spawned shell, alongside the existing
  `MINITERM_AGENT` / `MINITERM_WORKSPACE` / `MINITERM_AGENTS_DIR`:
  - `MINITERM_SESSION_ID` — the PTY session id. The id is taken *before* the child spawns
    (today it is taken after; a two-line reorder in `pty/mod.rs`).
  - `MINITERM_STATUS_DIR` — `<config_dir>/status/`.
- Keyed by session id, **not by agent name**: an agent can be renamed while it runs, and
  the shell's environment cannot follow the rename.
- Session ids restart at 1 every app run, so `status/` is emptied on startup and a file is
  deleted when its session is killed or exits.

### The hook

Claude Code passes every hook a JSON object on stdin that already carries
`hook_event_name`, `session_id` and (for tool events) `tool_name`. The hook therefore does
not need `jq`; it just forwards that object:

```sh
[ -n "$MINITERM_STATUS_DIR" ] && { tr -d '\r\n'; echo; } >> "$MINITERM_STATUS_DIR/$MINITERM_SESSION_ID.jsonl" || true
```

- The guard makes it a no-op for every Claude session started *outside* miniterm, which
  matters because the hook lives in the global `~/.claude/settings.json`.
- `tr -d '\r\n'; echo` forces one line per event.
- Always exits 0 and prints nothing, so it can never block or alter Claude.
- Registered for `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Notification`, `Stop`,
  `SubagentStop`. (`PostToolUse` added during implementation: the permission prompt comes
  *after* `PreToolUse`, so only `PostToolUse` can flip `waiting` back to `working`.)
  `PreToolUse` gets no matcher; it is needed both for the subagent counter and to flip
  `waiting` back to `working` once a permission is granted.

**To verify during implementation:** Claude Code on Windows runs hook commands through Git
Bash. The path in `MINITERM_STATUS_DIR` is a Windows path (`C:\Users\…`); Git Bash accepts
that inside double quotes for a redirect, but this must be checked on the real machine.
If it fails, Rust sets the variable with forward slashes, which both shells accept.

### Reading it (Rust)

A new module `src-tauri/src/agent_status.rs`:

- One watcher thread for the whole app (not per session). Every **500 ms** it lists
  `status/*.jsonl`, and for each file whose length grew, reads from the last offset, splits
  complete lines (a trailing partial line waits for the next tick), and feeds each event
  to that session's state machine.
- Cost: one `read_dir` plus one `metadata` per live Claude pane every 500 ms; nothing is
  read when nothing changed. Rejected: the `notify` crate (a new dependency for no
  measurable gain at this scale).
- A malformed line is skipped, never fatal.
- When a session's derived state changes, emit `agent-status`:

```ts
{ id: number, state: "working" | "waiting" | "done", subagents: number }
```

### State machine (per session)

| Event | Effect |
|---|---|
| `UserPromptSubmit` | `working` |
| `PreToolUse` | `working`; if `tool_name` is `Task` or `Agent` → `subagents += 1` |
| `Notification` | `waiting` |
| `SubagentStop` | `subagents = max(0, subagents − 1)`; if a `Stop` is pending and `subagents == 0` → `done` |
| `Stop` | `subagents > 0` → stay `working` (stop pending); else `done` |

Pure function, unit-tested with recorded event sequences, as Harp does with its replay
tests.

### Subagent counter (item 5)

Counting `PreToolUse(Task|Agent)` against `SubagentStop` is enough for subagents, which
is what keeps a turn alive in practice. Background shells are not counted (non-goal). The
sidebar shows `working · 2 agents` while the count is above zero.

### Fallback: tools without hooks

For a pane running an AI tool that has never produced a hook event, the existing reader
thread records the time of the last output chunk. The watcher tick turns that into
`active` (output in the last 2 s) or `idle` and emits the same event. This is a hint, not
a status: it cannot tell `waiting` from `done`, so it gets a neutral dot and **never**
triggers unread, toast or flash. Bare shells (no AI tool) get nothing new.

## Frontend

### Store

`sessions.svelte.ts` gains `agent: Record<sessionId, { state, subagents, unread }>`,
filled from the `agent-status` event and routed through the existing `owner` map, the same
way `session-exit` is.

**Unread** is set when a pane moves to `waiting` or `done` while it is not the focused pane
of the visible workspace, or the window is not focused. It clears when the pane gets
focus.

### Sidebar dot — one status per workspace, highest wins

`dead` > `waiting` > `done + unread` > `working` > `running` > `off`

`paneStatus()` in `store/status.ts` stays a pure function and gets the agent map as an
extra argument, so it remains testable under vitest in node. When collapsed to the icon
rail, the dot carries the same priority.

### Pane

- `PaneHeader` shows a small state chip next to the agent name: `working`, `waiting`,
  `done`, with the subagent count when above zero.
- **Attention ring (item 2):** a pane in `waiting`, or `done + unread`, gets an accent
  outline (distinct from the focused-pane border) until it is focused. One CSS class, no
  animation loop — a static ring, to stay within the performance budget.

### Toast + taskbar flash (item 3)

Triggered when a pane enters `waiting`, or `done` with unread, and **only if** the window
is unfocused or the pane's workspace is not the visible one.

- Taskbar: `getCurrentWindow().requestUserAttention(UserAttentionType.Informational)` —
  built into Tauri, no new dependency. Needs the `core:window:allow-request-user-attention`
  permission.
- Toast: `tauri-plugin-notification` (new dependency, official Tauri plugin) with
  `notification:default`. Text: "Athena needs you · <workspace>" or "Athena finished ·
  <workspace>".
- Rate-limited to one toast per pane per 10 s, so a Notification/Stop burst stays one toast.

## Settings

A new **Agents** section (Settings → AI tools):

- **Claude Code integration** — status line ("Installed / Not installed") and two buttons:
  - **Install hook:** shows exactly which entries will be added to
    `~/.claude/settings.json`, and writes only after the user confirms. Merges into
    existing `hooks` arrays and never replaces them; writes a `settings.json.miniterm-bak`
    first. Each entry carries the marker `"miniterm-agent-status"` in its command so it can
    be found again. All writes go through Rust (`config.rs` style, temp file + rename).
  - **Remove hook:** removes only the entries with that marker.
- **Notifications:** toggles for toast (default on) and taskbar flash (default on),
  stored in localStorage like the theme and font scale (per-machine preferences, no config
  change needed).

All strings in Turkish and English (`i18n/messages.ts`).

## Files touched

| File | Change |
|---|---|
| `src-tauri/src/pty/mod.rs` | take id before spawn; set the two env vars; record last-output time |
| `src-tauri/src/agent_status.rs` (new) | watcher thread, JSONL tail, state machine, tests |
| `src-tauri/src/claude_hooks.rs` (new) | read / merge / remove hook entries in `~/.claude/settings.json`, tests |
| `src-tauri/src/lib.rs`, `commands.rs` | start watcher, emit `agent-status`, hook install commands, clear `status/` on startup |
| `src-tauri/Cargo.toml`, `capabilities/default.json` | `tauri-plugin-notification`, two permissions |
| `src/ipc/index.ts` | `onAgentStatus`, hook install / remove / state |
| `src/store/sessions.svelte.ts`, `status.ts` | agent map, unread, dot priority |
| `src/lib/PaneHeader.svelte`, `TerminalPane.svelte`, `Sidebar.svelte`, `TreeItem.svelte` | chip, ring, dot |
| `src/lib/Settings.svelte` | Agents section |
| `src/i18n/messages.ts` | strings |
| `README.md`, `README.tr.md`, `CHANGELOG.md` | feature docs |

## Tests

- **Rust:** state-machine replay tests (prompt → tool → stop; permission → grant →
  stop; subagents outliving `Stop`; malformed line; partial trailing line); hook merge
  tests (empty file, existing hooks kept, idempotent install, remove only ours).
- **vitest:** dot priority, unread set/clear rules, toast rate limit.
- **Manual (user QA):** two Claude panes in different workspaces; one asks for permission
  while hidden → dot, ring, toast, flash; grant → `working`; finish → `done` + unread →
  focus clears it.

## Open questions

1. Hook install location: global `~/.claude/settings.json` (proposed — covers every
   workspace, and the guard makes it inert outside miniterm) or per-project
   `.claude/settings.local.json` in each workspace folder (no global edit, but it has to be
   written into every project directory).
2. Should `done` alone (without unread) still show on the dot, or fall back to `running`
   once seen? Proposed: fall back to `running` once seen, to keep the sidebar calm.
