# miniterm M4-A — Config & Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Load a TOML config at startup to override terminal background/foreground/cursor colors and font family/size, falling back to today's hardcoded defaults on any failure.

**Architecture:** New self-contained `src/config` module deserializes `%APPDATA%\miniterm\config.toml` into a runtime `Config { font, theme }` (separate serde "Raw" wire types → runtime types, per-field hex fallback). The `Theme` is threaded by value (`Copy`) from `main.rs` into `App` → `Workspace::build_frame` → `snapshot_cells`; font size/family feed the atlas/metrics/font resolver; the renderer gains a settable clear color. Config is read exactly once at startup — no timers, no watchers.

**Tech Stack:** Rust 2021, serde (derive), toml 0.8, winit 0.29, wgpu 0.19, alacritty_terminal 0.24.

**Spec:** `docs/superpowers/specs/2026-08-26-miniterm-m4-config-theme-design.md`

## Global Constraints

- **cargo is NOT on the bash PATH.** Prefix every cargo command: `export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"` then run `cargo …`.
- **Verification standard (per task):** `cargo build` clean apart from the ONE pre-existing `view`/`sampler` dead-code warning in `src/render/atlas_gpu.rs`; PLUS `cargo test` all green; PLUS (for tasks touching startup/render/wiring) a smoke run: `timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation"; echo DONE` — exit 143 (SIGTERM) is OK, any `panic`/`Validation` line is a failure. Does NOT gate on clippy/fmt.
- **Idle-0%-CPU is sacred:** no new `window.request_redraw()`, no timers, no watchers. Config is read once at startup; the render path gains only constant-color lookups.
- **Forward-looking-API pattern:** for a symbol defined in an early task but not consumed until a later task, use a narrow `#[allow(dead_code)]` + a one-line comment naming the consuming task. Remove it in that task.
- **No gamma conversion:** hex channels map to `byte / 255.0` and are used directly as `wgpu::Color` / vertex colors (matches the existing pipeline).
- **Never write to the real `%APPDATA%` in tests.** Parsing tests operate on in-memory TOML strings.

---

### Task 1: Config module (deps, types, parse, load)

Self-contained parsing/loading logic with full unit tests. No other file consumes it yet except the `mod config;` declaration.

**Files:**
- Modify: `Cargo.toml:6-17` (add `serde` derive + `toml`)
- Create: `src/config/mod.rs`
- Modify: `src/main.rs:1-5` (add `mod config;` with a temporary module-level allow)

**Interfaces:**
- Consumes: nothing from earlier tasks.
- Produces (later tasks rely on these exact names/types):
  - `pub type Rgb = [f32; 3];`
  - `pub struct Theme { pub background: Rgb, pub foreground: Rgb, pub cursor: Rgb }` — `#[derive(Debug, Clone, Copy, PartialEq)]`, `Default` = background `[0.05,0.05,0.06]`, foreground `[0.85,0.85,0.85]`, cursor `[0.85,0.85,0.85]`.
  - `pub struct FontConfig { pub family: Option<String>, pub size: f32 }` — `Default` = `None`/`18.0`.
  - `pub struct Config { pub font: FontConfig, pub theme: Theme }` — `Default` composes the two.
  - `pub fn parse_hex(s: &str) -> Option<Rgb>`
  - `pub fn parse_str(s: &str) -> Config` (parse a TOML string; any error → `Config::default()`)
  - `pub fn config_path() -> Option<std::path::PathBuf>`
  - `pub fn load() -> Config`

- [ ] **Step 1: Add dependencies**

In `Cargo.toml`, under `[dependencies]` (after `serde_json = "1"` on line 17), add:

```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```

- [ ] **Step 2: Write the config module with its failing tests**

Create `src/config/mod.rs`:

```rust
//! Startup configuration: theme colors + font, loaded once from
//! `%APPDATA%\miniterm\config.toml`. Every failure path falls back to
//! `Config::default()` (today's hardcoded look) and logs one stderr line.

use serde::Deserialize;

/// RGB in 0..1, ready for the render pipeline (each channel = byte / 255.0).
pub type Rgb = [f32; 3];

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub background: Rgb,
    pub foreground: Rgb,
    pub cursor: Rgb,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            background: [0.05, 0.05, 0.06],
            foreground: [0.85, 0.85, 0.85],
            cursor: [0.85, 0.85, 0.85],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontConfig {
    pub family: Option<String>,
    pub size: f32,
}

impl Default for FontConfig {
    fn default() -> Self {
        FontConfig { family: None, size: 18.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Config {
    pub font: FontConfig,
    pub theme: Theme,
}

impl Default for Theme { /* defined above */ }

// --- Wire (serde) types: hex strings + optional fields deserialize cleanly ---

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawConfig {
    font: RawFont,
    colors: RawColors,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawFont {
    family: Option<String>,
    size: f32,
}
impl Default for RawFont {
    fn default() -> Self { RawFont { family: None, size: 18.0 } }
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawColors {
    background: Option<String>,
    foreground: Option<String>,
    cursor: Option<String>,
}

impl RawConfig {
    fn into_config(self) -> Config {
        let d = Theme::default();
        let theme = Theme {
            background: self.colors.background.as_deref().and_then(parse_hex).unwrap_or(d.background),
            foreground: self.colors.foreground.as_deref().and_then(parse_hex).unwrap_or(d.foreground),
            cursor: self.colors.cursor.as_deref().and_then(parse_hex).unwrap_or(d.cursor),
        };
        let font = FontConfig { family: self.font.family, size: self.font.size };
        Config { font, theme }
    }
}

/// Parse "#rrggbb" (case-insensitive, leading '#' optional) into Rgb.
/// Returns None on wrong length or a non-hex character.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}

/// Parse a TOML string into a Config; any parse error yields Config::default().
/// Test/helper seam for `load` (which adds the file IO around this).
pub fn parse_str(s: &str) -> Config {
    match toml::from_str::<RawConfig>(s) {
        Ok(raw) => raw.into_config(),
        Err(_) => Config::default(),
    }
}

/// `%APPDATA%\miniterm\config.toml`, or None if APPDATA is unset.
pub fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA")
        .map(|a| std::path::PathBuf::from(a).join("miniterm").join("config.toml"))
}

/// Read + parse the config. Any failure (no APPDATA, missing/unreadable file,
/// TOML parse error) yields Config::default() and a single stderr log line.
pub fn load() -> Config {
    let path = match config_path() {
        Some(p) => p,
        None => return Config::default(),
    };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => {
            eprintln!("[miniterm] config: none at {}, using defaults", path.display());
            return Config::default();
        }
    };
    match toml::from_str::<RawConfig>(&text) {
        Ok(raw) => raw.into_config(),
        Err(e) => {
            eprintln!("[miniterm] config: parse error ({e}), using defaults");
            Config::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_basic() {
        assert_eq!(parse_hex("#0d0d10"), Some([13.0 / 255.0, 13.0 / 255.0, 16.0 / 255.0]));
    }

    #[test]
    fn parse_hex_no_prefix_and_uppercase() {
        assert_eq!(parse_hex("0D0D10"), Some([13.0 / 255.0, 13.0 / 255.0, 16.0 / 255.0]));
    }

    #[test]
    fn parse_hex_rejects_bad_input() {
        assert_eq!(parse_hex("#fff"), None);      // wrong length
        assert_eq!(parse_hex("#12345g"), None);   // non-hex char
        assert_eq!(parse_hex(""), None);
    }

    #[test]
    fn empty_toml_is_all_defaults() {
        assert_eq!(parse_str(""), Config::default());
    }

    #[test]
    fn full_config_maps_all_fields() {
        let toml = r#"
            [font]
            family = "JetBrains Mono"
            size = 20.0
            [colors]
            background = "#101014"
            foreground = "#c0c0c0"
            cursor = "#ff8800"
        "#;
        let c = parse_str(toml);
        assert_eq!(c.font.family.as_deref(), Some("JetBrains Mono"));
        assert_eq!(c.font.size, 20.0);
        assert_eq!(c.theme.background, parse_hex("#101014").unwrap());
        assert_eq!(c.theme.foreground, parse_hex("#c0c0c0").unwrap());
        assert_eq!(c.theme.cursor, parse_hex("#ff8800").unwrap());
    }

    #[test]
    fn partial_colors_keep_defaults() {
        let c = parse_str("[colors]\nbackground = \"#101014\"\n");
        let d = Theme::default();
        assert_eq!(c.theme.background, parse_hex("#101014").unwrap());
        assert_eq!(c.theme.foreground, d.foreground);
        assert_eq!(c.theme.cursor, d.cursor);
    }

    #[test]
    fn bad_hex_on_one_field_defaults_only_that_field() {
        let c = parse_str("[colors]\nbackground = \"nothex\"\nforeground = \"#c0c0c0\"\n");
        let d = Theme::default();
        assert_eq!(c.theme.background, d.background); // bad hex -> default
        assert_eq!(c.theme.foreground, parse_hex("#c0c0c0").unwrap());
    }

    #[test]
    fn unknown_field_yields_default() {
        // deny_unknown_fields makes this a parse error -> default.
        let c = parse_str("[font]\nbogus = 1\n");
        assert_eq!(c, Config::default());
    }

    #[test]
    fn font_size_default_when_font_section_omitted() {
        let c = parse_str("[colors]\ncursor = \"#ff8800\"\n");
        assert_eq!(c.font.size, 18.0);
        assert_eq!(c.font.family, None);
    }

    #[test]
    fn config_path_shape_when_appdata_set() {
        std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        let p = config_path().unwrap();
        assert!(p.ends_with("miniterm\\config.toml") || p.ends_with("miniterm/config.toml"));
    }
}
```

> NOTE for the implementer: the block above shows `impl Default for Theme { /* defined above */ }` as a placeholder pointing at the real `impl Default for Theme` written earlier in the file — write the real one ONCE (the one with the color literals) and do NOT add a second. `Config` derives `Default` via `#[derive(..., Default)]`, which requires `FontConfig: Default` and `Theme: Default` (both provided). Delete the placeholder comment line entirely.

- [ ] **Step 3: Run the config tests to verify they compile and pass**

Run:
```bash
export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"
cargo test config:: -- --nocapture
```
Expected: the 10 config tests PASS. (They will not run until `mod config;` is declared — do Step 4 first if the module is not reachable.)

- [ ] **Step 4: Declare the module in main.rs**

In `src/main.rs`, the module declarations are lines 1–5 (`mod app;` … `mod text;`). Add a `config` module with a temporary allow (removed in Task 5, when `load` and friends get consumed):

```rust
mod app;
#[allow(dead_code)] // consumed in Task 5 (main.rs wiring); keeps build clean until then
mod config;
mod layout;
mod render;
mod terminal;
mod text;
```

- [ ] **Step 5: Full build + test**

Run:
```bash
export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"
cargo build 2>&1 | grep -v "atlas_gpu" | grep -Ei "warning|error"; echo BUILD_DONE
cargo test 2>&1 | tail -5
```
Expected: no warnings/errors except the known atlas_gpu one; all tests green.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/config/mod.rs src/main.rs
git commit -m "feat(config): add TOML config module (theme + font) with graceful fallback"
```

---

### Task 2: Font resolver accepts a preferred face

`resolve_font` gains a `preferred: Option<&str>`; when `Some(non-empty)`, it tries that face first, else falls through to today's auto-detect. The single `main.rs` call site passes `None` for now (behavior unchanged).

**Files:**
- Modify: `src/text/font_source.rs:13-23` (`resolve_font` signature + preferred branch)
- Modify: `src/main.rs:40` (call site passes `None`)

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub fn resolve_font(bundled: &[u8], preferred: Option<&str>) -> (Vec<u8>, String)` — Task 5 will pass `config.font.family.as_deref()`.

- [ ] **Step 1: Change the signature and prepend the preferred branch**

Replace `resolve_font` (currently `src/text/font_source.rs:13-23`) with:

```rust
/// Resolve font bytes for the terminal. Returns the bytes plus a human label of
/// what was chosen (for logging). When `preferred` is Some(non-empty), that face
/// is tried first; otherwise (or on failure) falls back to auto-detect, then the
/// bundled Consolas.
pub fn resolve_font(bundled: &[u8], preferred: Option<&str>) -> (Vec<u8>, String) {
    if let Some(face) = preferred {
        let face = face.trim();
        if !face.is_empty() {
            if let Some(path) = face_to_path(face) {
                if let Ok(bytes) = fs::read(&path) {
                    return (bytes, format!("{face} ({}) [config]", path.display()));
                }
            }
        }
    }
    if let Some(face) = detect_face() {
        if let Some(path) = face_to_path(&face) {
            if let Ok(bytes) = fs::read(&path) {
                return (bytes, format!("{face} ({})", path.display()));
            }
        }
        return (bundled.to_vec(), format!("{face} -> unresolved, bundled Consolas"));
    }
    (bundled.to_vec(), "Consolas (bundled default)".to_string())
}
```

- [ ] **Step 2: Update the call site**

In `src/main.rs:40`, change:
```rust
let (font_bytes, font_label) = text::font_source::resolve_font(BUNDLED_FONT);
```
to:
```rust
let (font_bytes, font_label) = text::font_source::resolve_font(BUNDLED_FONT, None);
```

- [ ] **Step 3: Build + test (existing font_source tests must still pass)**

Run:
```bash
export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"
cargo build 2>&1 | grep -v "atlas_gpu" | grep -Ei "warning|error"; echo BUILD_DONE
cargo test font_source:: 2>&1 | tail -5
```
Expected: no new warnings; the 8 existing `font_source` tests still pass (they exercise `parse_wt_face`/`match_font_file`, not `resolve_font`).

- [ ] **Step 4: Commit**

```bash
git add src/text/font_source.rs src/main.rs
git commit -m "feat(font): resolve_font accepts a preferred face name"
```

---

### Task 3: Renderer settable clear color

The renderer stops hardcoding the clear color; it holds a `clear_color` field (default = today's `{0.05,0.05,0.06,1.0}`) and exposes `set_clear_color`. Both render passes read the field. No caller sets it yet (behavior unchanged).

**Files:**
- Modify: `src/render/renderer.rs:3-15` (struct field), the `Renderer::new` struct literal (add field init), a new setter method, and the two clear sites at `renderer.rs:288` and `renderer.rs:344`.

**Interfaces:**
- Consumes: nothing new.
- Produces: `pub fn set_clear_color(&mut self, rgb: [f32; 3])` — Task 5 calls `renderer.set_clear_color(config.theme.background)` once at startup.

- [ ] **Step 1: Add the field to the struct**

In `src/render/renderer.rs`, add a field after `instance_capacity: usize,` (line 14) inside `pub struct Renderer`:

```rust
    instance_capacity: usize,
    clear_color: wgpu::Color,
```

- [ ] **Step 2: Initialize the field in the constructor**

At the end of `Renderer::new`, the function returns a `Renderer { … }` struct literal. Add the field initializer alongside the others (matching today's hardcoded clear):

```rust
            clear_color: wgpu::Color { r: 0.05, g: 0.05, b: 0.06, a: 1.0 },
```

- [ ] **Step 3: Add the setter (with a temporary allow)**

Add this method inside `impl Renderer` (near the other accessors). The allow is removed in Task 5, when `main.rs` calls it:

```rust
    /// Set the background clear color used by both render passes.
    #[allow(dead_code)] // consumed in Task 5 (main.rs wiring)
    pub fn set_clear_color(&mut self, rgb: [f32; 3]) {
        self.clear_color = wgpu::Color {
            r: rgb[0] as f64,
            g: rgb[1] as f64,
            b: rgb[2] as f64,
            a: 1.0,
        };
    }
```

- [ ] **Step 4: Use the field at both clear sites**

At `src/render/renderer.rs:288` (the `draw_quads_clear` empty-frame pass) change:
```rust
                            load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.06, a: 1.0 }),
```
to:
```rust
                            load: wgpu::LoadOp::Clear(self.clear_color),
```
And at `src/render/renderer.rs:344` (the `draw_quads` main pass) change:
```rust
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.05, g: 0.05, b: 0.06, a: 1.0 }),
```
to:
```rust
                        load: wgpu::LoadOp::Clear(self.clear_color),
```

- [ ] **Step 5: Build + test + smoke**

Run:
```bash
export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"
cargo build 2>&1 | grep -v "atlas_gpu" | grep -Ei "warning|error"; echo BUILD_DONE
cargo test 2>&1 | tail -3
timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation"; echo DONE
```
Expected: no new warnings; tests green; smoke prints only `DONE` (background still `{0.05,0.05,0.06}` — visually unchanged).

- [ ] **Step 6: Commit**

```bash
git add src/render/renderer.rs
git commit -m "feat(render): settable clear color on Renderer (default unchanged)"
```

---

### Task 4: Thread Theme through App → Workspace → snapshot_cells

Colors become data. `snapshot_cells` takes fg/bg; `Workspace::build_frame` takes a `Theme`; `App` gains a `theme` field set in `App::new`. The `main.rs` `App::new` call site passes `config::Theme::default()` temporarily (Task 5 swaps in the loaded theme), keeping the build green.

**Files:**
- Modify: `src/app/workspace.rs` — `snapshot_cells` signature+body (lines 14-36), `Workspace::build_frame` (lines 110-199), `App` struct (295-303), `App::new` (306-322), `App::build_frame` (396-407), and the `tests` module (`test_app`, plus a new test).
- Modify: `src/main.rs:72` (`App::new` call passes a theme argument).

**Interfaces:**
- Consumes: `crate::config::{Theme, Rgb}` from Task 1.
- Produces:
  - `pub fn snapshot_cells(session: &Session, fg: crate::config::Rgb, bg: crate::config::Rgb) -> (Vec<Vec<CellView>>, (usize, usize))`
  - `Workspace::build_frame(&mut self, queue: &wgpu::Queue, atlas: &mut GpuAtlas, theme: crate::config::Theme) -> (Vec<QuadInstance>, Vec<QuadInstance>)`
  - `App { …, pub theme: crate::config::Theme, … }`
  - `App::new(root_rect: Rect, metrics: CellMetrics, theme: crate::config::Theme, spawn: impl FnMut(u16,u16)->Session) -> App`

- [ ] **Step 1: Add the color-threading test (fails to compile first)**

In `src/app/workspace.rs`, inside `mod tests`, add:

```rust
    #[test]
    fn snapshot_uses_given_colors() {
        let s = spawn_stub(5, 10);
        let (rows, _) = snapshot_cells(&s, [0.1, 0.2, 0.3], [0.4, 0.5, 0.6]);
        assert!(!rows.is_empty());
        let cell = rows[0][0];
        assert_eq!(cell.fg, [0.1, 0.2, 0.3]);
        assert_eq!(cell.bg, [0.4, 0.5, 0.6]);
    }
```

- [ ] **Step 2: Update `snapshot_cells` to take fg/bg**

Replace the signature and the `CellView` literal in `src/app/workspace.rs:14-36`:

```rust
/// Snapshot one Term grid into CellView rows + cursor (line, col).
pub fn snapshot_cells(
    session: &Session,
    fg: crate::config::Rgb,
    bg: crate::config::Rgb,
) -> (Vec<Vec<CellView>>, (usize, usize)) {
    let term = session.term.lock().unwrap_or_else(|e| e.into_inner());
    let grid = term.grid();
    let actual_lines = term.screen_lines();
    let actual_cols = term.columns();
    let cursor_pt = grid.cursor.point;
    let cursor_line = cursor_pt.line.0.max(0) as usize;
    let cursor_col = cursor_pt.column.0;
    let mut out = Vec::with_capacity(actual_lines);
    for line in 0..actual_lines {
        let mut row = Vec::with_capacity(actual_cols);
        for col in 0..actual_cols {
            let cell = &grid[Point::new(Line(line as i32), Column(col))];
            row.push(CellView { ch: cell.c, fg, bg });
        }
        out.push(row);
    }
    (out, (cursor_line, cursor_col))
}
```

- [ ] **Step 3: Update `Workspace::build_frame` to take a Theme**

In `src/app/workspace.rs`, change the `Workspace::build_frame` signature (line 110-114) to add a `theme` parameter:

```rust
    pub fn build_frame(
        &mut self,
        queue: &wgpu::Queue,
        atlas: &mut GpuAtlas,
        theme: crate::config::Theme,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
```

Inside it, change the `snapshot_cells` call (currently line 125):
```rust
            let (cells, (cur_line, cur_col)) = snapshot_cells(session, theme.foreground, theme.background);
```
And change the focused-pane cursor block color (currently line 163) from the literal to the theme cursor:
```rust
                    color: [theme.cursor[0], theme.cursor[1], theme.cursor[2], 1.0],
```

- [ ] **Step 4: Add the `theme` field to `App` and its constructor**

In `src/app/workspace.rs`, add the field to `pub struct App` (after `pub gutter: f32,`, line 299):
```rust
    pub theme: crate::config::Theme,
```

Change `App::new` (lines 306-322) to accept and store the theme:
```rust
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        theme: crate::config::Theme,
        spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let gutter = 4.0;
        let ws = Workspace::new("1".to_string(), root_rect, metrics, gutter, spawn);
        App {
            workspaces: vec![ws],
            active: 0,
            metrics,
            gutter,
            root_rect,
            theme,
            editing: None,
            edit_buf: String::new(),
        }
    }
```

- [ ] **Step 5: Pass the theme down in `App::build_frame`**

In `src/app/workspace.rs`, `App::build_frame` (line 407) currently calls:
```rust
        let (mut bg, mut glyphs) = self.active_ws_mut().build_frame(queue, atlas);
```
`Theme` is `Copy`, so copy it out before the `&mut self` reborrow to avoid a borrow conflict:
```rust
        let theme = self.theme;
        let (mut bg, mut glyphs) = self.active_ws_mut().build_frame(queue, atlas, theme);
```

- [ ] **Step 6: Fix the test helper**

In `src/app/workspace.rs`, `test_app` (lines 493-496) must pass a theme. Change:
```rust
    fn test_app() -> App {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        App::new(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, m, crate::config::Theme::default(), spawn_stub)
    }
```

- [ ] **Step 7: Update the main.rs call site (temporary default)**

In `src/main.rs:72`, change:
```rust
    let mut app = App::new(pane_rect, metrics, spawn);
```
to (temporary — Task 5 replaces `default()` with the loaded theme):
```rust
    let mut app = App::new(pane_rect, metrics, config::Theme::default(), spawn);
```

- [ ] **Step 8: Build + test + smoke**

Run:
```bash
export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"
cargo build 2>&1 | grep -v "atlas_gpu" | grep -Ei "warning|error"; echo BUILD_DONE
cargo test 2>&1 | tail -5
timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation"; echo DONE
```
Expected: no new warnings; the new `snapshot_uses_given_colors` test plus all prior tests pass; smoke prints only `DONE` (default theme = visually unchanged).

- [ ] **Step 9: Commit**

```bash
git add src/app/workspace.rs src/main.rs
git commit -m "feat(app): thread Theme through App -> Workspace -> snapshot_cells"
```

---

### Task 5: Startup wiring — load config and apply font + theme + clear color

Final integration: load the config once, feed font size/family into the atlas/metrics/resolver, pass the real theme into `App::new`, and set the renderer clear color. Removes the temporary allows from Tasks 1 and 3.

**Files:**
- Modify: `src/main.rs` — remove `const FONT_PX` (line 21); the module allow (Task 1); the startup block (lines 40-72); the `resolve_font` call; the `App::new` call.

**Interfaces:**
- Consumes: `config::load`, `config::Config`, `config::Theme` (Task 1); `resolve_font(_, preferred)` (Task 2); `Renderer::set_clear_color` (Task 3); `App::new(_, _, theme, _)` (Task 4).
- Produces: nothing for later tasks (final task).

- [ ] **Step 1: Remove the temporary module allow**

In `src/main.rs`, change the config module declaration back to plain (its members are now consumed by the wiring below):
```rust
mod config;
```

- [ ] **Step 2: Remove the FONT_PX constant**

Delete `src/main.rs:21`:
```rust
const FONT_PX: f32 = 18.0;
```
(`BUNDLED_FONT` on line 20 stays.)

- [ ] **Step 3: Load config and wire font + theme + clear color**

In `src/main.rs`, after `let mut renderer = Renderer::new(&window);` (line 36) and before the font resolution, load the config; then thread it through. The startup block (lines 40-73) becomes:

```rust
    // Load user config once (theme + font); any failure falls back to defaults.
    let config = config::load();

    // Apply the background clear color immediately.
    renderer.set_clear_color(config.theme.background);

    // Resolve the terminal font: user-configured family first, else auto-detect.
    let (font_bytes, font_label) =
        text::font_source::resolve_font(BUNDLED_FONT, config.font.family.as_deref());
    eprintln!("[miniterm] font: {font_label}");

    let font_px = config.font.size;

    // Build the GpuAtlas once (stored alongside renderer).
    let mut atlas = GpuAtlas::new(
        renderer.device(),
        renderer.atlas_bind_group_layout(),
        font_bytes.clone(),
        font_px,
    );

    // Measure cell metrics using the same font bytes.
    let metrics: CellMetrics = measure(&font_bytes, font_px);

    // Build root rect from current surface size.
    let (sw, sh) = renderer.surface_size();
    let window_rect = Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };
    let pane_rect = app::pane_area_rect(window_rect);

    // Keep a proxy clone alive outside App so we can build spawn_one closures
    // inside the keyboard handler without moving our only proxy into App::new.
    let proxy = event_loop.create_proxy();

    let spawn = {
        let p = proxy.clone();
        move |rows: u16, cols: u16| -> Session {
            let pp = p.clone();
            Session::spawn(rows, cols, "cmd.exe", move || {
                let _ = pp.send_event(UserEvent::PtyOutput);
            })
        }
    };
    let mut app = App::new(pane_rect, metrics, config.theme, spawn);
```

> This replaces the block currently spanning lines 38-72 (the old font comment, `resolve_font(BUNDLED_FONT)`, `GpuAtlas::new(..., FONT_PX)`, `measure(..., FONT_PX)`, surface-size rect, proxy/spawn, and `App::new(pane_rect, metrics, spawn)`). Keep the two lines after it unchanged:
> ```rust
>     app.set_root_rect(pane_rect);
>     app.active_ws_mut().relayout(pane_rect);
> ```

- [ ] **Step 4: Remove the temporary allow on the renderer setter**

In `src/render/renderer.rs`, delete the `#[allow(dead_code)] // consumed in Task 5 …` line above `pub fn set_clear_color` (added in Task 3) — it now has a real caller.

- [ ] **Step 5: Build + test + smoke**

Run:
```bash
export PATH="$PATH:/c/Users/kadir.avci/.cargo/bin"
cargo build 2>&1 | grep -v "atlas_gpu" | grep -Ei "warning|error"; echo BUILD_DONE
cargo test 2>&1 | tail -5
timeout 6 ./target/debug/miniterm.exe 2>&1 | grep -Ei "panic|Validation"; echo DONE
```
Expected: no warnings except the known atlas_gpu one; all tests green; smoke prints only `DONE`. With no config file present, stderr shows one `[miniterm] config: none at …` line and the look is unchanged.

- [ ] **Step 6: Manual verification with a real config (optional, human-run)**

Create `%APPDATA%\miniterm\config.toml`:
```toml
[font]
size = 22.0
[colors]
background = "#101018"
cursor = "#ff8800"
```
Launch `miniterm.exe`: background should be dark blue-ish, cursor orange, font larger. Delete the file to restore defaults. (Not an automated gate.)

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/render/renderer.rs
git commit -m "feat(config): wire config into startup (font size/family, theme, clear color)"
```

---

## Self-Review

**1. Spec coverage:**
- TOML config at `%APPDATA%\miniterm\config.toml` → Task 1 `config_path`/`load`. ✅
- `[font] family/size` → Task 1 types; applied Task 2 (family) + Task 5 (size/family). ✅
- `[colors] background/foreground/cursor` #rrggbb → Task 1 `parse_hex`/`RawColors`; applied Task 4 (fg/bg/cursor) + Task 5 (clear color = background). ✅
- Graceful fallback (missing/unreadable/parse error/bad hex) + one stderr line → Task 1 `load` + per-field fallback in `into_config`. ✅
- Out of scope (ANSI palette, keybindings, scrollback, shell, chrome theming, live reload) → not touched. ✅
- Dependencies serde derive + toml → Task 1 Step 1. ✅
- `resolve_font` preferred param, existing tests valid → Task 2. ✅
- Renderer clear-color field + setter + two sites → Task 3. ✅
- `App.theme` + `App::new` param + `snapshot_cells(fg,bg)` + `Workspace::build_frame(theme)` + `App::build_frame` passes theme → Task 4. ✅
- Startup wiring (`mod config;`, load, font size/face, theme, clear color) → Tasks 1 & 5. ✅
- Idle-0%-CPU untouched → no redraw/timer added anywhere. ✅

**2. Placeholder scan:** The only "placeholder" is the deliberately-flagged `impl Default for Theme { /* defined above */ }` in Task 1 Step 2, with an explicit NOTE telling the implementer to write the real impl once and delete the comment. No TBD/TODO-as-work, no "add error handling", no untested code steps.

**3. Type consistency:** `Rgb = [f32;3]` used consistently in `parse_hex`, `snapshot_cells`, `set_clear_color`. `Theme`/`FontConfig`/`Config` names identical across Tasks 1/4/5. `App::new(root_rect, metrics, theme, spawn)` argument order matches the call site in Tasks 4/5 and the spec §7. `resolve_font(bundled, preferred)` matches call sites in Tasks 2/5. `build_frame(queue, atlas, theme)` matches `App::build_frame`'s call in Task 4 Step 5. Build stays green at every task boundary (temporary defaults/allows bridge Tasks 1/3 → 5 and Task 4 → 5).
