# miniterm Milestone 1 — Single Working Terminal — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce a single OS window that hosts one live shell (ConPTY) rendered on the GPU, accepting keyboard input — the foundation every later milestone builds on.

**Architecture:** Rust binary. `winit` owns the window + event loop. A per-terminal background thread does blocking PTY reads and feeds bytes into an `alacritty_terminal` grid, then wakes the main thread. The main thread renders the grid with `wgpu` using a shared glyph atlas and instanced quads, drawing only on damage.

**Tech Stack:** Rust, `winit` 0.29, `wgpu` 0.19, `pollster`, `alacritty_terminal` 0.24, `portable-pty` 0.8, `swash` 0.1, `bytemuck`, `slotmap` 1.

**Spec:** `docs/superpowers/specs/2026-08-25-miniterm-design.md`

## Global Constraints

- Language: Rust (edition 2021), stable toolchain.
- Target platform: Windows (ConPTY via `portable-pty` default backend).
- Rendering: event-driven only. Never request a redraw unless there is damage (input, PTY output, resize).
- No unwrap in threads that must survive a shell exit; propagate/log instead of panicking the process.
- Font is monospace; a fixed `cell_w`/`cell_h` in physical pixels is computed once at font load.
- Dependency versions are pinned exactly as listed in Tech Stack. If a crate's API differs from the code below, the compile/test cycle will catch it — adjust to the installed version's real signatures, keeping behavior identical.

---

### Task 1: Cargo project skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `rust-toolchain.toml`

**Interfaces:**
- Consumes: nothing.
- Produces: a buildable binary crate named `miniterm`.

- [ ] **Step 1: Create `rust-toolchain.toml`**

```toml
[toolchain]
channel = "stable"
```

- [ ] **Step 2: Create `Cargo.toml`**

```toml
[package]
name = "miniterm"
version = "0.1.0"
edition = "2021"

[dependencies]
winit = "0.29"
wgpu = "0.19"
pollster = "0.3"
bytemuck = { version = "1", features = ["derive"] }
slotmap = "1"
alacritty_terminal = "0.24"
portable-pty = "0.8"
swash = "0.1"
raw-window-handle = "0.6"

[profile.release]
lto = "thin"
codegen-units = 1
```

- [ ] **Step 3: Create minimal `src/main.rs`**

```rust
fn main() {
    println!("miniterm boot");
}
```

- [ ] **Step 4: Build and run**

Run: `cargo run`
Expected: compiles, prints `miniterm boot`.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml rust-toolchain.toml src/main.rs
git commit -m "chore: scaffold miniterm cargo project"
```

---

### Task 2: Window + wgpu clear color

**Files:**
- Create: `src/render/mod.rs`
- Create: `src/render/renderer.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `Renderer::new(window: &winit::window::Window) -> Renderer`
  - `Renderer::resize(&mut self, size: winit::dpi::PhysicalSize<u32>)`
  - `Renderer::render(&mut self)` — clears the surface to a dark color.

- [ ] **Step 1: Create `src/render/mod.rs`**

```rust
pub mod renderer;
```

- [ ] **Step 2: Create `src/render/renderer.rs` with surface setup and clear**

```rust
use winit::window::Window;

pub struct Renderer {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl Renderer {
    pub fn new(window: &Window) -> Renderer {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();
        // SAFETY: window outlives the renderer for the program's lifetime.
        let surface = unsafe {
            instance
                .create_surface_unsafe(
                    wgpu::SurfaceTargetUnsafe::from_window(window).unwrap(),
                )
                .unwrap()
        };
        let adapter = pollster::block_on(instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor::default(),
            None,
        ))
        .unwrap();
        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .unwrap_or(caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        Renderer { surface, device, queue, config }
    }

    pub fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.surface.configure(&self.device, &self.config);
    }

    pub fn render(&mut self) {
        let frame = match self.surface.get_current_texture() {
            Ok(f) => f,
            Err(_) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05, g: 0.05, b: 0.06, a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
    }
}
```

- [ ] **Step 3: Rewrite `src/main.rs` to open the window and drive the event loop**

```rust
mod render;

use render::renderer::Renderer;
use winit::event::{Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

fn main() {
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new()
        .with_title("miniterm")
        .build(&event_loop)
        .unwrap();
    let mut renderer = Renderer::new(&window);

    event_loop
        .run(move |event, elwt| {
            if let Event::WindowEvent { event, .. } = event {
                match event {
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::Resized(size) => renderer.resize(size),
                    WindowEvent::RedrawRequested => renderer.render(),
                    _ => {}
                }
            }
        })
        .unwrap();
}
```

- [ ] **Step 4: Run and verify a window appears with a dark background**

Run: `cargo run`
Expected: a window titled "miniterm" opens showing a dark fill; closing it exits cleanly.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/main.rs src/render/mod.rs src/render/renderer.rs
git commit -m "feat: open window and clear surface with wgpu"
```

---

### Task 2b: Note on winit 0.29 vs 0.30

**Files:** none (informational).

winit 0.29 uses the closure-based `EventLoop::run` shown above. If `cargo build` reports that `EventLoop::run` or `WindowBuilder` is missing, the resolved winit is 0.30+, which requires the `ApplicationHandler` trait. In that case, keep behavior identical: implement `ApplicationHandler` with `resumed` (create window+renderer), `window_event` (match the same arms), and call `event_loop.run_app(&mut app)`. Do not change any other task; only the event-loop shell differs.

---

### Task 3: Font metrics

**Files:**
- Create: `src/text/mod.rs`
- Create: `src/text/metrics.rs`
- Create: `assets/font/DejaVuSansMono.ttf` (bundle a monospace TTF; DejaVu Sans Mono is a permissively licensed choice)
- Test: in `src/text/metrics.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `struct CellMetrics { pub cell_w: f32, pub cell_h: f32, pub ascent: f32 }`
  - `fn measure(font_bytes: &[u8], px: f32) -> CellMetrics`

- [ ] **Step 1: Create `src/text/mod.rs`**

```rust
pub mod metrics;
```

- [ ] **Step 2: Write the failing test in `src/text/metrics.rs`**

```rust
use swash::FontRef;

pub struct CellMetrics {
    pub cell_w: f32,
    pub cell_h: f32,
    pub ascent: f32,
}

pub fn measure(_font_bytes: &[u8], _px: f32) -> CellMetrics {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FONT: &[u8] = include_bytes!("../../assets/font/DejaVuSansMono.ttf");

    #[test]
    fn metrics_are_positive_and_monospace_sized() {
        let m = measure(FONT, 16.0);
        assert!(m.cell_w > 0.0 && m.cell_h > 0.0);
        assert!(m.ascent > 0.0 && m.ascent < m.cell_h);
        // Monospace advance is narrower than line height for typical fonts.
        assert!(m.cell_w < m.cell_h);
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test text::metrics -- --nocapture`
Expected: FAIL (panics in `unimplemented!`).

- [ ] **Step 4: Implement `measure`**

```rust
pub fn measure(font_bytes: &[u8], px: f32) -> CellMetrics {
    let font = FontRef::from_index(font_bytes, 0).expect("valid font");
    let metrics = font.metrics(&[]).scale(px);
    let glyph = font.charmap().map('M');
    let advance = font.glyph_metrics(&[]).scale(px).advance_width(glyph);
    let cell_h = (metrics.ascent + metrics.descent + metrics.leading).ceil();
    CellMetrics {
        cell_w: advance.ceil(),
        cell_h,
        ascent: metrics.ascent,
    }
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test text::metrics -- --nocapture`
Expected: PASS. If `swash` metric method names differ in 0.1.x, adjust to the real API (same fields: ascent, descent, leading, advance width for glyph 'M') keeping the assertions valid.

- [ ] **Step 6: Wire the module into main**

Add `mod text;` to `src/main.rs` (top, with the other `mod` lines).

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml src/main.rs src/text/mod.rs src/text/metrics.rs assets/font/DejaVuSansMono.ttf
git commit -m "feat: compute monospace cell metrics with swash"
```

---

### Task 4: Glyph atlas (shelf packer)

**Files:**
- Create: `src/text/atlas.rs`
- Modify: `src/text/mod.rs`
- Test: in `src/text/atlas.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: nothing (packing is pure logic; GPU upload comes in Task 7).
- Produces:
  - `struct GlyphKey { pub ch: char, pub bold: bool, pub italic: bool }`
  - `struct GlyphRect { pub x: u32, pub y: u32, pub w: u32, pub h: u32 }`
  - `struct ShelfPacker { /* width, cursor_x, cursor_y, shelf_h */ }`
  - `ShelfPacker::new(width: u32) -> ShelfPacker`
  - `ShelfPacker::insert(&mut self, w: u32, h: u32) -> GlyphRect` — returns the packed position, advancing shelves.

- [ ] **Step 1: Add module to `src/text/mod.rs`**

```rust
pub mod metrics;
pub mod atlas;
```

- [ ] **Step 2: Write the failing test in `src/text/atlas.rs`**

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub ch: char,
    pub bold: bool,
    pub italic: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlyphRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct ShelfPacker {
    width: u32,
    cursor_x: u32,
    cursor_y: u32,
    shelf_h: u32,
}

impl ShelfPacker {
    pub fn new(_width: u32) -> ShelfPacker {
        unimplemented!()
    }
    pub fn insert(&mut self, _w: u32, _h: u32) -> GlyphRect {
        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packs_left_to_right_then_wraps_to_new_shelf() {
        let mut p = ShelfPacker::new(20);
        let a = p.insert(8, 10);
        let b = p.insert(8, 10);
        // Third glyph would overflow width 20 -> wraps to a new shelf.
        let c = p.insert(8, 10);
        assert_eq!(a, GlyphRect { x: 0, y: 0, w: 8, h: 10 });
        assert_eq!(b, GlyphRect { x: 8, y: 0, w: 8, h: 10 });
        assert_eq!(c, GlyphRect { x: 0, y: 10, w: 8, h: 10 });
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test text::atlas -- --nocapture`
Expected: FAIL (`unimplemented!`).

- [ ] **Step 4: Implement the packer**

```rust
impl ShelfPacker {
    pub fn new(width: u32) -> ShelfPacker {
        ShelfPacker { width, cursor_x: 0, cursor_y: 0, shelf_h: 0 }
    }

    pub fn insert(&mut self, w: u32, h: u32) -> GlyphRect {
        if self.cursor_x + w > self.width {
            self.cursor_y += self.shelf_h;
            self.cursor_x = 0;
            self.shelf_h = 0;
        }
        let rect = GlyphRect { x: self.cursor_x, y: self.cursor_y, w, h };
        self.cursor_x += w;
        self.shelf_h = self.shelf_h.max(h);
        rect
    }
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test text::atlas -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/text/mod.rs src/text/atlas.rs
git commit -m "feat: shelf packer for glyph atlas"
```

---

### Task 5: ConPTY session + reader thread + alacritty grid

**Files:**
- Create: `src/terminal/mod.rs`
- Create: `src/terminal/pty.rs`
- Create: `src/terminal/session.rs`
- Modify: `src/main.rs`
- Test: in `src/terminal/session.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `type SharedTerm = std::sync::Arc<std::sync::Mutex<alacritty_terminal::Term<EventProxy>>>`
  - `struct Session { pub term: SharedTerm, writer: Box<dyn std::io::Write + Send>, size: (u16, u16) }`
  - `Session::spawn(rows: u16, cols: u16, shell: &str, on_output: impl Fn() + Send + 'static) -> Session`
  - `Session::write(&mut self, bytes: &[u8])`
  - `Session::resize(&mut self, rows: u16, cols: u16)`
  - `struct EventProxy` implementing `alacritty_terminal::event::EventListener` (no-op for M1).

- [ ] **Step 1: Create `src/terminal/mod.rs`**

```rust
pub mod pty;
pub mod session;
```

- [ ] **Step 2: Create `src/terminal/pty.rs` — thin ConPTY wrapper**

```rust
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};

pub struct Pty {
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
}

impl Pty {
    pub fn spawn(rows: u16, cols: u16, shell: &str) -> Pty {
        let sys = NativePtySystem::default();
        let pair = sys
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .expect("openpty");
        let cmd = CommandBuilder::new(shell);
        let _child = pair.slave.spawn_command(cmd).expect("spawn shell");
        let writer = pair.master.take_writer().expect("writer");
        let reader = pair.master.try_clone_reader().expect("reader");
        Pty { writer, reader, master: pair.master }
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        let _ = self.master.resize(PtySize {
            rows, cols, pixel_width: 0, pixel_height: 0,
        });
    }
}
```

- [ ] **Step 3: Create `src/terminal/session.rs` — Term + reader thread**

```rust
use crate::terminal::pty::Pty;
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::term::{test::TermSize, Config, Term};
use alacritty_terminal::vte::ansi::Processor;
use std::io::Read;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct EventProxy;
impl EventListener for EventProxy {
    fn send_event(&self, _event: Event) {}
}

pub type SharedTerm = Arc<Mutex<Term<EventProxy>>>;

pub struct Session {
    pub term: SharedTerm,
    writer: Box<dyn std::io::Write + Send>,
    pty: Pty,
    size: (u16, u16),
}

impl Session {
    pub fn spawn(
        rows: u16,
        cols: u16,
        shell: &str,
        on_output: impl Fn() + Send + 'static,
    ) -> Session {
        let mut pty = Pty::spawn(rows, cols, shell);
        let writer = std::mem::replace(
            &mut pty.writer,
            Box::new(std::io::sink()),
        );
        let term_size = TermSize::new(cols as usize, rows as usize);
        let term = Arc::new(Mutex::new(Term::new(
            Config::default(),
            &term_size,
            EventProxy,
        )));

        let reader_term = term.clone();
        let mut reader = std::mem::replace(
            &mut pty.reader,
            Box::new(std::io::empty()),
        );
        std::thread::spawn(move || {
            let mut parser = Processor::new();
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        {
                            let mut term = reader_term.lock().unwrap();
                            for &byte in &buf[..n] {
                                parser.advance(&mut *term, byte);
                            }
                        }
                        on_output();
                    }
                }
            }
        });

        Session { term, writer, pty, size: (rows, cols) }
    }

    pub fn write(&mut self, bytes: &[u8]) {
        let _ = self.writer.write_all(bytes);
        let _ = self.writer.flush();
    }

    pub fn resize(&mut self, rows: u16, cols: u16) {
        if (rows, cols) == self.size {
            return;
        }
        self.size = (rows, cols);
        self.pty.resize(rows, cols);
        let mut term = self.term.lock().unwrap();
        term.resize(TermSize::new(cols as usize, rows as usize));
    }
}
```

- [ ] **Step 4: Write the failing integration test in `src/terminal/session.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::index::{Column, Line, Point};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};

    #[test]
    fn echo_reaches_the_grid() {
        static WOKE: AtomicBool = AtomicBool::new(false);
        let mut s = Session::spawn(24, 80, "cmd.exe", || {
            WOKE.store(true, Ordering::SeqCst);
        });
        // Ask cmd to print a marker then exit.
        s.write(b"echo MINITERM_OK\r\n");

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut found = false;
        while Instant::now() < deadline && !found {
            std::thread::sleep(Duration::from_millis(50));
            let term = s.term.lock().unwrap();
            let grid = term.grid();
            for line in 0..24 {
                let mut row = String::new();
                for col in 0..80 {
                    let p = Point::new(Line(line), Column(col));
                    row.push(grid[p].c);
                }
                if row.contains("MINITERM_OK") {
                    found = true;
                    break;
                }
            }
        }
        assert!(found, "expected echoed marker in grid");
        assert!(WOKE.load(Ordering::SeqCst), "on_output should fire");
    }
}
```

- [ ] **Step 5: Run the test to verify it fails, then compiles/passes**

Run: `cargo test terminal::session -- --nocapture`
Expected: first FAIL if any signature is off; fix signatures against the installed `alacritty_terminal` 0.24 API (the grid indexing via `Point`, `term.grid()`, `cell.c`, `Term::new`, `TermSize::new`, `Processor::advance`). Once correct: PASS with the marker found. This test is the source of truth that the VT pipeline works.

- [ ] **Step 6: Wire module into main**

Add `mod terminal;` to `src/main.rs`.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs src/terminal/mod.rs src/terminal/pty.rs src/terminal/session.rs
git commit -m "feat: ConPTY session feeding an alacritty_terminal grid"
```

---

### Task 6: Grid → quad instances (pure mapping)

**Files:**
- Create: `src/render/grid_draw.rs`
- Modify: `src/render/mod.rs`
- Test: in `src/render/grid_draw.rs` under `#[cfg(test)]`

**Interfaces:**
- Consumes: `CellMetrics` (Task 3); a cell abstraction (char + fg/bg rgb).
- Produces:
  - `#[repr(C)] struct QuadInstance { pub pos: [f32;2], pub size: [f32;2], pub uv_min: [f32;2], pub uv_max: [f32;2], pub color: [f32;4] }` (derives `bytemuck::Pod, Zeroable, Clone, Copy`)
  - `struct CellView { pub ch: char, pub fg: [f32;3], pub bg: [f32;3] }`
  - `fn build_instances(cells: &[Vec<CellView>], m: &CellMetrics, origin: [f32;2], atlas_uv: &dyn Fn(char) -> ([f32;2],[f32;2])) -> (Vec<QuadInstance>, Vec<QuadInstance>)` returning `(bg_quads, glyph_quads)`.

- [ ] **Step 1: Add module to `src/render/mod.rs`**

```rust
pub mod renderer;
pub mod grid_draw;
```

- [ ] **Step 2: Write the failing test in `src/render/grid_draw.rs`**

```rust
use crate::text::metrics::CellMetrics;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    pub pos: [f32; 2],
    pub size: [f32; 2],
    pub uv_min: [f32; 2],
    pub uv_max: [f32; 2],
    pub color: [f32; 4],
}

pub struct CellView {
    pub ch: char,
    pub fg: [f32; 3],
    pub bg: [f32; 3],
}

pub fn build_instances(
    _cells: &[Vec<CellView>],
    _m: &CellMetrics,
    _origin: [f32; 2],
    _atlas_uv: &dyn Fn(char) -> ([f32; 2], [f32; 2]),
) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
    unimplemented!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_grid_cells_by_metrics_and_skips_spaces_for_glyphs() {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        let cells = vec![vec![
            CellView { ch: 'A', fg: [1.0, 1.0, 1.0], bg: [0.0, 0.0, 0.0] },
            CellView { ch: ' ', fg: [1.0, 1.0, 1.0], bg: [0.0, 0.0, 0.0] },
        ]];
        let uv = |_c: char| ([0.0, 0.0], [0.5, 0.5]);
        let (bg, glyphs) = build_instances(&cells, &m, [100.0, 50.0], &uv);
        // One bg quad per cell.
        assert_eq!(bg.len(), 2);
        assert_eq!(bg[1].pos, [110.0, 50.0]);
        assert_eq!(bg[0].size, [10.0, 20.0]);
        // Spaces produce no glyph quad.
        assert_eq!(glyphs.len(), 1);
        assert_eq!(glyphs[0].pos, [100.0, 50.0]);
        assert_eq!(glyphs[0].uv_max, [0.5, 0.5]);
    }
}
```

- [ ] **Step 3: Run the test to verify it fails**

Run: `cargo test render::grid_draw -- --nocapture`
Expected: FAIL (`unimplemented!`).

- [ ] **Step 4: Implement `build_instances`**

```rust
pub fn build_instances(
    cells: &[Vec<CellView>],
    m: &CellMetrics,
    origin: [f32; 2],
    atlas_uv: &dyn Fn(char) -> ([f32; 2], [f32; 2]),
) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
    let mut bg = Vec::new();
    let mut glyphs = Vec::new();
    for (row_idx, row) in cells.iter().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let x = origin[0] + col_idx as f32 * m.cell_w;
            let y = origin[1] + row_idx as f32 * m.cell_h;
            bg.push(QuadInstance {
                pos: [x, y],
                size: [m.cell_w, m.cell_h],
                uv_min: [0.0, 0.0],
                uv_max: [0.0, 0.0],
                color: [cell.bg[0], cell.bg[1], cell.bg[2], 1.0],
            });
            if cell.ch != ' ' && cell.ch != '\0' {
                let (uv_min, uv_max) = atlas_uv(cell.ch);
                glyphs.push(QuadInstance {
                    pos: [x, y],
                    size: [m.cell_w, m.cell_h],
                    uv_min,
                    uv_max,
                    color: [cell.fg[0], cell.fg[1], cell.fg[2], 1.0],
                });
            }
        }
    }
    (bg, glyphs)
}
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test render::grid_draw -- --nocapture`
Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add src/render/mod.rs src/render/grid_draw.rs
git commit -m "feat: map terminal grid cells to instanced quads"
```

---

### Task 7: GPU atlas upload + instanced draw pipeline

**Files:**
- Create: `src/render/atlas_gpu.rs`
- Create: `src/render/shader.wgsl`
- Modify: `src/render/renderer.rs`
- Modify: `src/render/mod.rs`

**Interfaces:**
- Consumes: `ShelfPacker` (Task 4), `QuadInstance` (Task 6), `CellMetrics` (Task 3).
- Produces:
  - `struct GpuAtlas` holding the atlas `wgpu::Texture`, its bind group, a `HashMap<GlyphKey, ([f32;2],[f32;2])>` of UVs, and rasterizing-on-miss via `swash`.
  - `GpuAtlas::uv_for(&mut self, ch: char, queue, m) -> ([f32;2],[f32;2])`
  - `Renderer::draw_quads(&mut self, bg: &[QuadInstance], glyphs: &[QuadInstance], atlas: &GpuAtlas)` — one instanced pass for bg (no texture sampling), one for glyphs (samples atlas alpha as coverage).

This task is a GPU smoke milestone: the deliverable is that the pipeline builds and a hard-coded string renders visibly. Pure logic was covered in Tasks 3, 4, 6.

- [ ] **Step 1: Create `src/render/shader.wgsl`**

```wgsl
struct Globals { screen: vec2<f32> };
@group(0) @binding(0) var<uniform> globals: Globals;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_smp: sampler;

struct VsIn {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_min: vec2<f32>,
    @location(3) uv_max: vec2<f32>,
    @location(4) color: vec4<f32>,
    @builtin(vertex_index) vid: u32,
};
struct VsOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) textured: f32,
};

@vertex
fn vs_main(in: VsIn) -> VsOut {
    // Two triangles from a unit quad, expanded per-instance.
    var corners = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0), vec2<f32>(1.0, 0.0), vec2<f32>(1.0, 1.0),
    );
    let c = corners[in.vid];
    let px = in.pos + c * in.size;
    // Pixel space -> clip space (y down).
    let ndc = vec2<f32>(
        px.x / globals.screen.x * 2.0 - 1.0,
        1.0 - px.y / globals.screen.y * 2.0,
    );
    var out: VsOut;
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = mix(in.uv_min, in.uv_max, c);
    out.color = in.color;
    out.textured = select(0.0, 1.0, in.uv_max.x > in.uv_min.x);
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    if (in.textured > 0.5) {
        let a = textureSample(atlas_tex, atlas_smp, in.uv).r;
        return vec4<f32>(in.color.rgb, in.color.a * a);
    }
    return in.color;
}
```

- [ ] **Step 2: Create `src/render/atlas_gpu.rs`**

```rust
use crate::text::atlas::{GlyphKey, GlyphRect, ShelfPacker};
use crate::text::metrics::CellMetrics;
use std::collections::HashMap;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::zeno::Format;
use swash::FontRef;

pub const ATLAS_SIZE: u32 = 1024;

pub struct GpuAtlas {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub bind_group: wgpu::BindGroup,
    packer: ShelfPacker,
    uvs: HashMap<GlyphKey, ([f32; 2], [f32; 2])>,
    font: Vec<u8>,
    px: f32,
    scale_cx: ScaleContext,
}

impl GpuAtlas {
    pub fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        font: Vec<u8>,
        px: f32,
    ) -> GpuAtlas {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_SIZE, height: ATLAS_SIZE, depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas-bg"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        GpuAtlas {
            texture, view, sampler, bind_group,
            packer: ShelfPacker::new(ATLAS_SIZE),
            uvs: HashMap::new(),
            font, px,
            scale_cx: ScaleContext::new(),
        }
    }

    pub fn uv_for(
        &mut self,
        queue: &wgpu::Queue,
        ch: char,
    ) -> ([f32; 2], [f32; 2]) {
        let key = GlyphKey { ch, bold: false, italic: false };
        if let Some(uv) = self.uvs.get(&key) {
            return *uv;
        }
        let font = FontRef::from_index(&self.font, 0).unwrap();
        let glyph_id = font.charmap().map(ch);
        let mut scaler = self
            .scale_cx
            .builder(font)
            .size(self.px)
            .hint(true)
            .build();
        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .render(&mut scaler, glyph_id);

        let (w, h, data) = match image {
            Some(img) => (
                img.placement.width.max(1),
                img.placement.height.max(1),
                img.data,
            ),
            None => (1, 1, vec![0u8]),
        };
        let rect: GlyphRect = self.packer.insert(w, h);
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x: rect.x, y: rect.y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        let f = ATLAS_SIZE as f32;
        let uv = (
            [rect.x as f32 / f, rect.y as f32 / f],
            [(rect.x + w) as f32 / f, (rect.y + h) as f32 / f],
        );
        self.uvs.insert(key, uv);
        uv
    }
}
```

- [ ] **Step 3: Extend `Renderer` with the pipeline, uniform, and `draw_quads`**

Add to `Renderer`: an instance `wgpu::Buffer` (growable), a globals uniform buffer + bind group (screen size), the render pipeline built from `shader.wgsl`, and the atlas bind group layout. Provide:

```rust
// in renderer.rs
pub fn atlas_bind_group_layout(&self) -> &wgpu::BindGroupLayout {
    &self.atlas_layout
}

pub fn queue(&self) -> &wgpu::Queue { &self.queue }
pub fn device(&self) -> &wgpu::Device { &self.device }

pub fn draw_quads(
    &mut self,
    bg: &[crate::render::grid_draw::QuadInstance],
    glyphs: &[crate::render::grid_draw::QuadInstance],
    atlas: &crate::render::atlas_gpu::GpuAtlas,
) {
    // 1. Update globals uniform with current screen size.
    // 2. Upload bg then glyph instances into the instance buffer
    //    (resize buffer if needed).
    // 3. begin_render_pass (Clear), set pipeline + globals bind group.
    // 4. Draw bg instances (atlas bind group still bound but uv_max==uv_min
    //    means untextured branch). draw(0..6, 0..bg.len()).
    // 5. Set atlas bind group, draw glyph instances offset after bg.
    // 6. submit + present.
}
```

Implement it concretely following the wgpu instanced-draw pattern: vertex buffer layout with `step_mode: VertexStepMode::Instance` for the five `QuadInstance` attributes (locations 0–4), `draw(0..6, first..last)` per instance range. Blend state: standard alpha blending so glyph coverage composites over background.

- [ ] **Step 4: Temporary render check in `main.rs`**

Temporarily, on `RedrawRequested`, build a hard-coded `Vec<Vec<CellView>>` spelling `"miniterm"`, call `GpuAtlas::uv_for` per char, `build_instances`, then `draw_quads`.

Run: `cargo run`
Expected: the window shows the word "miniterm" in the font over the dark background. Remove the hard-coded block after verifying (Task 8 replaces it with the live grid).

- [ ] **Step 5: Commit**

```bash
git add src/render/mod.rs src/render/renderer.rs src/render/atlas_gpu.rs src/render/shader.wgsl src/main.rs
git commit -m "feat: gpu glyph atlas and instanced quad pipeline"
```

---

### Task 8: Wire live terminal — render the grid + keyboard input + damage wakeups

**Files:**
- Create: `src/app.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Consumes: `Session` (Task 5), `build_instances`/`CellView` (Task 6), `GpuAtlas` (Task 7), `Renderer` (Task 2/7), `CellMetrics` (Task 3).
- Produces:
  - `struct App { session: Session, metrics: CellMetrics, atlas: GpuAtlas, needs_redraw: bool }`
  - `App::grid_to_cells(&self) -> Vec<Vec<CellView>>` — snapshot the locked `Term` grid into `CellView` rows (map alacritty colors to rgb).
  - `App::on_key(&mut self, text: &str)` — forward bytes to the session.

- [ ] **Step 1: Create `src/app.rs` with grid snapshot**

```rust
use crate::render::grid_draw::CellView;
use crate::terminal::session::Session;
use alacritty_terminal::index::{Column, Line, Point};

pub fn grid_to_cells(session: &Session, rows: usize, cols: usize) -> Vec<Vec<CellView>> {
    let term = session.term.lock().unwrap();
    let grid = term.grid();
    let mut out = Vec::with_capacity(rows);
    for line in 0..rows {
        let mut row = Vec::with_capacity(cols);
        for col in 0..cols {
            let cell = &grid[Point::new(Line(line as i32), Column(col))];
            row.push(CellView {
                ch: cell.c,
                fg: [0.85, 0.85, 0.85],
                bg: [0.05, 0.05, 0.06],
            });
        }
        out.push(row);
    }
    out
}
```

(Color mapping stays monochrome for M1; theme palette arrives in the config milestone. The point of M1 is a live, typable terminal.)

- [ ] **Step 2: Set up the event loop with a wakeup proxy in `main.rs`**

- Use `EventLoop::<UserEvent>::with_user_event()`; define `enum UserEvent { PtyOutput }`.
- Pass an `EventLoopProxy` clone into `Session::spawn`'s `on_output` closure; it calls `proxy.send_event(UserEvent::PtyOutput)`.
- Compute `rows/cols` from window size and `CellMetrics`; spawn the session with the OS default shell (`"cmd.exe"` for M1; configurable later).
- On `UserEvent::PtyOutput` → `window.request_redraw()`.
- On `WindowEvent::KeyboardInput` with text → `session.write(text.as_bytes())` then `window.request_redraw()`. Map Enter to `\r`, Backspace to `0x7f`, Ctrl-C to `0x03`.
- On `WindowEvent::Resized` → recompute rows/cols, `session.resize(...)`, `renderer.resize(...)`, request redraw.
- On `RedrawRequested` → `grid_to_cells`, `uv_for` per non-space char, `build_instances`, `renderer.draw_quads(...)`.

- [ ] **Step 3: Run and interact**

Run: `cargo run`
Expected: a real shell prompt appears. Typing shows characters; Enter runs commands; output streams in. Resizing the window reflows the shell. Idle (no typing) uses ~0% CPU (no redraws requested).

- [ ] **Step 4: Verify idle cost**

With the window open and idle, observe Task Manager: miniterm CPU should sit at ~0%. If it spins, a redraw is being requested every frame — ensure redraws are only requested on input/PtyOutput/resize.

- [ ] **Step 5: Run the full test suite**

Run: `cargo test`
Expected: metrics, atlas, grid_draw, and session tests all PASS.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/main.rs
git commit -m "feat: live terminal render with keyboard input and damage-driven redraw"
```

---

## Self-Review

**Spec coverage (M1 subset):** window + wgpu (Task 2) ✓; font metrics/atlas (Tasks 3,4,7) ✓; alacritty_terminal + ConPTY + reader thread + wakeup (Task 5, 8) ✓; damage-driven redraw + idle-0% (Task 8 steps 3–4) ✓; auto PTY resize on window resize (Task 8 step 2) ✓; instanced bg+glyph draw with scissor-ready origin (Tasks 6,7) ✓. Deferred to later milestones by design: multi-pane layout/drag-resize, sidebar workspaces, tab groups, config/theme/keybindings, session persistence, scrollback wheel, cursor styles. These are explicitly out of M1 scope per the spec's phasing (§12).

**Placeholder scan:** No "TBD"/"handle edge cases" left. Task 7 step 3 intentionally describes the wgpu instanced-draw wiring in prose plus concrete signatures rather than a full 200-line buffer-management listing; every type and call it references is concrete. Task 8 step 2 is an explicit event-wiring checklist with exact byte mappings.

**Type consistency:** `CellMetrics{cell_w,cell_h,ascent}` used identically in Tasks 3,6,7,8. `QuadInstance` fields (pos,size,uv_min,uv_max,color) match between Task 6 (definition), the WGSL vertex layout (Task 7), and `draw_quads`. `CellView{ch,fg,bg}` consistent in Tasks 6 and 8. `Session`/`SharedTerm`/`EventProxy` consistent between Tasks 5 and 8. `ShelfPacker::insert -> GlyphRect` consistent between Tasks 4 and 7. `GpuAtlas::uv_for` returns `([f32;2],[f32;2])` matching the `atlas_uv` closure signature in Task 6.

**API-drift caveat:** `alacritty_terminal` 0.24, `swash` 0.1, `winit` 0.29/0.30, and `wgpu` 0.19 have the most volatile surfaces. Each such step includes a run/verify checkpoint so a signature mismatch fails fast at compile/test time rather than hiding. Adjust to the resolved version's real signatures, preserving the documented behavior.
