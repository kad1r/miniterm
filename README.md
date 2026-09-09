<div align="center">

<img src="docs/icon.png" alt="miniterm" width="96" />

# miniterm

**A multi-workspace terminal for running several AI CLI sessions side by side.**

Runs Claude Code, Gemini CLI and friends in one window, in a grid layout,
each workspace in its own directory.

Windows · macOS · Linux — Tauri 2 + Svelte 5 + xterm.js

[Türkçe](README.tr.md) · **English**

</div>

![Four Claude Code sessions in a 2×2 grid; the focused pane has an orange frame](docs/screenshots/workspace-grid.png)

---

## Why

Running an AI CLI across several repos — or several branches of one repo — means
a pile of hand-opened terminal tabs. Keeping track of which tab sits in which
directory, and which one is still working, is left to you.

miniterm turns that around: you define a **workspace** — a directory, an AI tool,
a grid layout — and the app sets the rest up. Switching between workspaces does
not kill processes; they keep running in the background and the output picks up
where it left off when you come back.

## What it does

**Workspaces**
- Tree view on the left; group with folders up to 5 levels deep, move by
  drag and drop
- Each workspace = one directory + one AI tool + a grid of 1–6 terminals
  (1×2, 2×2, 2×3 … layouts)
- The selected tool's command is typed into every terminal in the grid on start
- Split lines resize by dragging; the ratio is stored with the workspace
- The sidebar collapses; when collapsed, workspaces shrink to an icon rail with
  status dots

**Sessions**
- Processes in hidden workspaces stay alive
- Output is held in a 256 KB ring buffer per session on the Rust side; on return
  the screen is redrawn from that buffer
- When a process dies the pane dims and prints an info line — Enter restarts it
- The dot in the sidebar shows status: running / exited / stopped

**Native terminal behaviour**
- `Ctrl+C` copies when there is a selection, otherwise sends SIGINT — copying
  consumes the selection, so the next `Ctrl+C` interrupts again
- `Ctrl+V`, `Ctrl+Shift+V` and `Shift+Insert` paste; the right-click menu works too
- Drop a file or folder and the full path is typed at the cursor, quoted for the
  shell (Windows paths with spaces, UNC paths, POSIX quote escaping)
- `Ctrl` + `+` / `−` / `0` and `Ctrl` + wheel change the font size; only the
  terminal scales, not the window chrome
- `F2` renames the selected workspace

**Interface**
- Turkish and English; picked from the system language on first run, changed
  under Settings → Appearance
- The focused terminal is framed in Claude orange
- WebGL renderer; falls back to canvas silently on context loss
- Inter + Roboto Mono ship embedded, no dependency on system fonts

## Credentials

miniterm **stores no secrets.** Signing in is the AI CLIs' own job; miniterm only
keeps the text of the command to run. The config file is plain text and contains
no tokens.

## Install

Requirements: Node 20+, Rust stable, and the Tauri 2
[prerequisites](https://tauri.app/start/prerequisites/) for your platform.

```bash
npm install
npm run tauri dev
```

Release build:

```bash
npm run tauri build
```

## Your first workspace

On a fresh install the sidebar points you at a single job:

![Empty sidebar with no workspaces](docs/screenshots/empty-state.png)

The **+** button opens the single-screen wizard — directory, name, shell, AI
tool, terminal count and grid layout all in one place. Saved directories and
recent ones fill in with one click; the only required field is the directory.

![The single-screen new workspace wizard](docs/screenshots/new-workspace.png)

Hit **Create** and the grid is built with your chosen tool's command typed into
every terminal.

## Settings

**AI tools** — the commands each workspace can pick from. The preview shows that
the command is typed into the shell rather than passed as an argument.

![The AI tools tab](docs/screenshots/settings-ai-tools.png)

**Directories** — the shortcuts that appear as chips in the wizard, with recents
below them.

![The directory shortcuts tab](docs/screenshots/settings-directories.png)

**Terminal** — the default used by workspaces that have not picked their own
shell. Shells are discovered by scanning the disk; no need to type a path.

![The default shell tab](docs/screenshots/settings-terminal.png)

**Appearance** — language and font scale. The scale grows the interface and open
terminals together; boxes and spacing stay fixed.

![The language and font size tab](docs/screenshots/settings-appearance.png)

## Architecture

```
src/
  store/    Pure state logic — tree, layout, sessions, config
  term/     xterm helpers — clipboard, path quoting, exit notice
  i18n/     TR/EN message table; a missing key is a compile error
  ipc/      Tauri command wrappers
  lib/      Svelte 5 components (runes)
src-tauri/  Rust backend — PTY lifecycle, ring buffer, config I/O
```

Business logic lives in pure modules outside Svelte; components only bind. That
is why nearly every test runs without a DOM.

## Tests

```bash
npm test                      # TypeScript unit tests (vitest)
npm run check                 # svelte-check + tsc
cd src-tauri && cargo test    # Rust unit + integration tests
```

Some tests are **global constraint locks**: they guard constants that leave no
error behind when they quietly disappear — the value of `SAVE_DEBOUNCE_MS`, or
the presence of the window-destroy permission in the capability file. If one
breaks, confirm the change was intentional.

## Development notes

### Windows GNU toolchain

This repo builds with `x86_64-pc-windows-gnu`. `src-tauri/.cargo/config.toml`
holds machine-specific absolute paths and is therefore not in version control;
the setup steps are in Task 1 of `docs/superpowers/plans/2026-09-07-miniterm.md`.

### Design and plan

- Design: `docs/superpowers/specs/2026-09-07-miniterm-design.md`
- Implementation plan: `docs/superpowers/plans/2026-09-07-miniterm.md`
- Acceptance verification: `docs/verification/`
