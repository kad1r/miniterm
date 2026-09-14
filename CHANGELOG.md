# Changelog

Every released version of miniterm, newest first.

This file ships inside the app: the **About** tab in Settings shows the entry whose
version matches the running build, so it is also what `gh release create --notes-file`
should be pointed at. Keep the format — `## <version> — <date>`, `### section`,
`- bullet`, plain paragraphs, `**bold**` and `` `code` `` inline. Nothing else is rendered.

## 0.6.0 — 2026-09-14

### A different AI tool in every terminal

A workspace used to pick one AI tool for its whole grid. Running Claude in two panes,
Gemini in a third and a plain shell in a fourth meant three separate workspaces pointed
at the same directory.

- The **new workspace** wizard now shows a row per terminal once you ask for more than
  one. Leave them alone and they all follow the workspace tool; change one and only that
  pane changes.
- **Add terminal** offers the same choice for the panes it is about to create. The ones
  already running are shown but locked: a terminal's command is typed into it when it
  starts, so re-pointing it afterwards would change the label without changing what is
  running.
- Deleting a tool in Settings now clears it from individual panes too, not just from the
  workspaces that had it as their default.

Existing workspaces are untouched and keep running exactly one tool, which is still the
default any pane falls back to.

### The SmartScreen warning is documented

Installing miniterm shows `Windows protected your PC` with an unknown publisher. The
README now says what to click and why it happens — the installer is not code-signed,
and a certificate is a recurring cost the project does not carry. Nothing about the
warning changed; it was simply undocumented.

## 0.5.0 — 2026-09-14

### Minimize and maximize a terminal

A pane now carries window-style controls instead of a lone close button.

- **Ctrl+Shift+Z** blows the focused pane up to fill the whole workspace, and the same
  chord puts it back — the way tmux's `prefix z` works.
- **Ctrl+Shift+M** sends the focused pane to a strip below the grid. Click it there to
  bring it home.

Minimizing is not resizing. A fixed grid cannot shrink one cell without deforming its
whole row and column, so the pane leaves the grid rather than shrinking inside it.

A minimized terminal keeps running, scrollback and all, which means it still counts
against the six-terminal limit. **Add terminal** and **Change layout** reserve a place
for it, so it always has somewhere to come back to.

### An About tab in Settings

Settings now tells you which version you are running and what went into it. The version
is read from the installed build itself, and the notes ship inside the app, so they are
right even with no network and even on a build you made yourself.

## 0.4.0 — 2026-09-11

### Change a workspace's grid layout

Right-click a workspace in the sidebar and pick **Change layout**. A 2×2 can become a
1×4 without deleting anything — every shell keeps running, scrollback and all, and the
panes just re-arrange.

- The layout picker is no longer decoration: **Apply** follows whether anything changed
  at all, count or shape.
- A workspace at the six-terminal limit can still be reshaped — 1×6, 2×3, 3×2 and 6×1
  are all on offer.
- **Add terminal** is unchanged; it still opens on one more than today.

## 0.3.0 — 2026-09-11

### Close a terminal

Every pane now has a close button in its corner, and **Ctrl+Shift+W** does the same
thing from the keyboard — the chord Windows Terminal uses.

The grid closes ranks behind the pane that left. The layout keeps the orientation it
had, so a column of three becomes a column of two rather than flipping on its side. The
last terminal has no close button: an empty workspace would have nothing to show, and
removing the workspace is still the sidebar's job.

### Focus comes back where you left it

Each workspace remembers the pane that last held the keyboard and hands it back when you
return to it. Close the focused pane and the caret stays in that cell, which is now the
pane that took its place.

The first terminal is also focused on startup, so there is no longer a click needed
before typing.

### Fixes

- Switching workspaces no longer sprays `[?9001h` into whatever is running in the pane.
  ConPTY's win32-input-mode was being re-sent on every attach; it is now enabled once,
  when the session is created.
- Panes that survive a relayout now resize their shell as well as their own view.
  Previously only the panes whose session changed were told about the new geometry, so
  `clear` and full-screen programs could render at the old grid's size.

## 0.2.0 — 2026-09-11

First tagged build of miniterm — a multi-workspace terminal for running AI CLI sessions
side by side on Windows.

- **Add terminals to an existing workspace.** Right-click a workspace in the sidebar and
  pick **Add terminal** to walk the same layout picker the wizard uses, up to six panes.
- **Ctrl+Enter reaches the shell.** Modified chords are now sent to ConPTY as real key
  events, so PSReadLine inserts a continuation line instead of submitting the command.
- **Upgrades keep your settings.** Installing a newer version no longer offers to delete
  saved workspaces and directory shortcuts; that only happens on a deliberate uninstall.
- **Information toasts dismiss themselves** after three seconds. Errors still wait for you.
- Roomier sidebar rows.
