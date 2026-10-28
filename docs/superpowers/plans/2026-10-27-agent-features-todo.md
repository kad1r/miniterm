# Agent-workflow features — TODO

**Date:** 2026-10-27
**Source:** review of [dominikmartn/harp](https://github.com/dominikmartn/harp) and similar
agent-terminal projects (cmux, ccmanager, agent-deck, claude-squad, crystal, emdash, opcode).
Ideas only — Harp is AGPL-3.0, no code is copied.

Legend: `[ ]` open · `[~]` in progress · `[x]` done · ⏳ later · ❌ out of scope

## Package 1 — agent status (approved, spec first)

Spec: `docs/superpowers/specs/2026-10-27-agent-status-design.md`

- [x] 1. Agent status detection: working / waiting / done per pane, shown on the sidebar
      dot and pane header (Harp, cmux, ccmanager)
- [x] 2. Unread mark + attention ring on the waiting pane (Harp, cmux)
- [x] 3. Windows toast + taskbar flash when an agent in a hidden workspace needs you (cmux)
- [x] 5. Subagent / background-task counter: stay "working" after a turn ends while
      subagents or background shells still run (Harp)

## Backlog

- [ ] 4. "Jump to latest unread" shortcut + notification list (cmux)
- [ ] 6. "Set aside" flag for a workspace (moon icon), excluded from tallies (Harp)
- [ ] 7. Limit rings: Claude + Codex 5 h / weekly usage in the sidebar footer (Harp).
      Claude needs a `statusLine` entry in `~/.claude/settings.json` — opt-in only
- [ ] 8. Git branch, linked PR and listening ports on the sidebar row (cmux)
- [ ] 9. Restore sessions on app restart, e.g. `claude --continue` (ccmanager, cmux)
- [ ] 10. Command palette + per-workspace custom commands (cmux)
- ⏳ 11. Per-pane git worktree + diff review (claude-squad, crystal, emdash)
- ⏳ 12. Usage / cost tracking (opcode) — may merge with 7
- ❌ 13. Kanban task board (vibe-kanban)
- ❌ 14. Web / mobile remote access (claudecodeui)
- ❌ 15. tmux sync, embedded browser, AI auto-approval — not meaningful on Windows or a
      security risk
