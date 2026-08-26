# miniterm M4-A — Config & Theme Design

**Status:** approved (2026-08-26)
**Milestone:** M4-A (visual core: theme colors + font from a config file)
**Parent spec:** `docs/superpowers/specs/2026-08-25-miniterm-design.md`
**Branch:** `feature/m1-single-terminal` (M1–M3 committed here; M4 continues on same branch)

## Goal

Load a TOML config file at startup and use it to override the terminal
**background / foreground / cursor** colors and the **font family / size**.
When the file is absent or malformed, fall back to today's hardcoded defaults
so behavior is unchanged. In-memory only; no live reload (deferred).

## Scope

**In scope (M4-A):**
- TOML config at `%APPDATA%\miniterm\config.toml`.
- `[font]` — optional `family` (string) and `size` (float).
- `[colors]` — `background`, `foreground`, `cursor` as `#rrggbb` hex.
- Graceful fallback to defaults on missing file, unreadable file, parse error,
  or bad hex — never panic, log one line to stderr.

**Out of scope (deferred to M4-B / later):**
- 16 ANSI palette. Cells do not yet carry per-cell fg/bg (see "Current state"
  below); a palette in config would be dead until per-cell color parsing
  exists. Excluded to avoid unused config surface.
- Keybindings, scrollback, default shell configuration.
- Sidebar / tab-chrome color theming (chrome keeps its current constants).
- Live reload / file watching. Config is read once at startup.

## Current State (what the code hardcodes today)

- `src/main.rs`: `const FONT_PX: f32 = 18.0;`. Font **face** is auto-resolved
  by `text::font_source::resolve_font(BUNDLED_FONT)` (Windows Terminal
  settings.json → console registry → bundled Consolas). `resolve_font` takes
  only the bundled fallback bytes; it has no notion of a user-preferred face.
- `src/app/workspace.rs` `snapshot_cells`: every cell gets
  `fg: [0.85, 0.85, 0.85]`, `bg: [0.05, 0.05, 0.06]` — a single fg/bg for all
  text (no per-cell ANSI color).
- `src/app/workspace.rs` `Workspace::build_frame`: focused-pane cursor block
  color `[0.85, 0.85, 0.85, 1.0]`.
- `src/render/renderer.rs` lines 288 and 344: two render passes clear with
  `wgpu::Color { r: 0.05, g: 0.05, b: 0.06, a: 1.0 }` (matches the cell bg).
- `Cargo.toml`: has `serde_json` but **not** `serde` (derive) or `toml`.

## Design

### 1. Dependencies

Add to `Cargo.toml` `[dependencies]`:
```toml
serde = { version = "1", features = ["derive"] }
toml = "0.8"
```
(`serde_json` stays for `font_source`.)

### 2. New module: `src/config/mod.rs`

```rust
use serde::Deserialize;

/// RGB in linear 0..1, ready for the render pipeline.
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
            cursor:     [0.85, 0.85, 0.85],
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontConfig {
    pub family: Option<String>,
    pub size: f32,
}

impl Default for FontConfig {
    fn default() -> Self { FontConfig { family: None, size: 18.0 } }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    pub font: FontConfig,
    pub theme: Theme,
}

impl Default for Config {
    fn default() -> Self {
        Config { font: FontConfig::default(), theme: Theme::default() }
    }
}
```

**Wire (serde) types** are separate from the runtime types so hex strings and
optional fields deserialize cleanly, then convert into `Config`:

```rust
#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawConfig {
    font: RawFont,
    colors: RawColors,
}

#[derive(Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawFont { family: Option<String>, size: f32 }
impl Default for RawFont { fn default() -> Self { RawFont { family: None, size: 18.0 } } }

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct RawColors {
    background: Option<String>,
    foreground: Option<String>,
    cursor: Option<String>,
}
```

Conversion: each `Option<String>` hex → `parse_hex`; on `None` **or** a hex
that fails to parse, keep the corresponding `Theme::default()` component. A bad
hex on one field does not discard the others.

### 3. Hex parsing

```rust
/// Parse "#rrggbb" (case-insensitive, leading '#' optional) into linear-ish
/// Rgb (each channel byte / 255.0). Returns None on wrong length or non-hex.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let h = s.strip_prefix('#').unwrap_or(s);
    if h.len() != 6 { return None; }
    let r = u8::from_str_radix(&h[0..2], 16).ok()?;
    let g = u8::from_str_radix(&h[2..4], 16).ok()?;
    let b = u8::from_str_radix(&h[4..6], 16).ok()?;
    Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0])
}
```
(No gamma conversion — matches the existing pipeline, which uses raw 0..1
values directly as `wgpu::Color` / vertex colors.)

### 4. Loading

```rust
/// `%APPDATA%\miniterm\config.toml`, or None if APPDATA is unset.
pub fn config_path() -> Option<std::path::PathBuf> {
    std::env::var_os("APPDATA")
        .map(|a| std::path::PathBuf::from(a).join("miniterm").join("config.toml"))
}

/// Read + parse the config. Any failure (no APPDATA, missing/unreadable file,
/// TOML parse error) yields Config::default() and a single stderr log line.
pub fn load() -> Config {
    let path = match config_path() { Some(p) => p, None => return Config::default() };
    let text = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(_) => { eprintln!("[miniterm] config: none at {}, using defaults", path.display()); return Config::default(); }
    };
    match toml::from_str::<RawConfig>(&text) {
        Ok(raw) => raw.into_config(),
        Err(e) => { eprintln!("[miniterm] config: parse error ({e}), using defaults"); Config::default() }
    }
}
```

`RawConfig::into_config` maps raw → `Config`, applying the per-field hex
fallback described in §2.

### 5. Font application (`src/main.rs`)

- Replace `const FONT_PX: f32 = 18.0;` usage with `config.font.size` (feeds both
  `GpuAtlas::new(..., font_bytes.clone(), size)` and `measure(&font_bytes, size)`).
- Extend `resolve_font` to accept a preferred face:
  `resolve_font(bundled: &[u8], preferred: Option<&str>) -> (Vec<u8>, String)`.
  When `preferred` is `Some(face)`, try to resolve **that** face first (via the
  existing `face_to_path`); on success use it, otherwise fall through to today's
  auto-detect (WT → console registry → bundled). This keeps every existing
  `font_source` test valid (they call the internal helpers, not `resolve_font`);
  update the single call site in `main.rs`.

### 6. Color application

Introduce a `Theme` value threaded from `main.rs` into rendering:

- **`App`** gains a `pub theme: config::Theme` field, set in `App::new`
  (add a `theme` parameter) and stored for `build_frame`.
- **`snapshot_cells`** takes the theme (or just fg+bg) so each `CellView` uses
  `theme.foreground` / `theme.background` instead of the literals. Signature:
  `snapshot_cells(session: &Session, fg: Rgb, bg: Rgb)`.
- **`Workspace::build_frame`** receives the theme from `App::build_frame` and
  uses `theme.foreground`→text is already per-cell fg; `theme.cursor` for the
  cursor block; passes fg/bg into `snapshot_cells`. `Workspace` may either take
  the theme as a `build_frame` parameter or hold its own copy; **parameter is
  preferred** (keeps `Workspace` free of config coupling). `App::build_frame`
  already borrows `self` — pass `self.theme` down by value (`Theme` is `Copy`).
- **Renderer clear color** (renderer.rs:288, :344): the clear must use
  `theme.background`. Add a `clear_color: wgpu::Color` field to `Renderer`
  (default `{0.05,0.05,0.06,1.0}`) plus `set_clear_color(&mut self, rgb: Rgb)`;
  both render passes read the field. `main.rs` calls `renderer.set_clear_color(config.theme.background)` once at startup.

### 7. Startup wiring (`src/main.rs`)

```
let config = config::load();
renderer.set_clear_color(config.theme.background);
let (font_bytes, font_label) = text::font_source::resolve_font(
    BUNDLED_FONT, config.font.family.as_deref());
let size = config.font.size;
let mut atlas = GpuAtlas::new(renderer.device(), ..., font_bytes.clone(), size);
let metrics = measure(&font_bytes, size);
...
let mut app = App::new(pane_rect, metrics, config.theme, spawn);
```
Add `mod config;` at the crate root.

### 8. Idle-0%-CPU invariant

Config is read once at startup; no timers, no watchers, no extra
`request_redraw`. The rendering path gains only constant-color lookups. The
sacred idle-0%-CPU behavior is untouched.

## Error Handling

Every failure path returns `Config::default()` and logs exactly one stderr
line. No `unwrap`/`expect` on file or parse results. `deny_unknown_fields`
makes typos a parse error (→ defaults + log), which is acceptable for M4-A;
revisit if it proves annoying.

## Testing

- `parse_hex`: `#0d0d10` → `[13/255, 13/255, 16/255]`; without `#`; uppercase;
  wrong length → None; non-hex char → None.
- `RawConfig::into_config`: full config maps all fields; empty string `""`
  input → all defaults; partial (`[colors] background` only) → background set,
  foreground/cursor default; bad hex on one field → that field defaults, others
  honored; unknown field → parse error path returns default (tested via `load`
  helper or a `from_str` wrapper).
- Font size default when `[font]` omitted.
- `config_path` shape when `APPDATA` is set (join yields `.../miniterm/config.toml`).

No test writes to the real `%APPDATA%`; parsing tests operate on in-memory TOML
strings via a small `parse_str(&str) -> Config` helper that wraps
`toml::from_str` + `into_config`.

## File Structure

- Create: `src/config/mod.rs` (types, `parse_hex`, `parse_str`, `config_path`, `load`, tests)
- Modify: `Cargo.toml` (add `serde` derive + `toml`)
- Modify: `src/main.rs` (`mod config;`, load, wire font size/face + theme + clear color)
- Modify: `src/text/font_source.rs` (`resolve_font` gains `preferred: Option<&str>`)
- Modify: `src/render/renderer.rs` (clear-color field + setter, two pass sites)
- Modify: `src/app/workspace.rs` (`snapshot_cells` fg/bg params; `Workspace::build_frame` theme param; `App.theme` field + `App::new` param + `App::build_frame` passes theme down)
```
