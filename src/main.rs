mod app;
mod render;
mod terminal;
mod text;

use app::grid_to_cells;
use render::atlas_gpu::GpuAtlas;
use render::grid_draw::build_instances;
use render::renderer::Renderer;
use terminal::session::Session;
use text::metrics::{measure, CellMetrics};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::keyboard::{Key, NamedKey};
use winit::window::WindowBuilder;

const FONT_BYTES: &[u8] = include_bytes!("../assets/font/CascadiaMono.ttf");
const FONT_PX: f32 = 18.0;

#[derive(Debug, Clone)]
enum UserEvent {
    PtyOutput,
}

fn main() {
    let event_loop: EventLoop<UserEvent> = EventLoopBuilder::<UserEvent>::with_user_event()
        .build()
        .unwrap();
    let window = WindowBuilder::new()
        .with_title("miniterm")
        .build(&event_loop)
        .unwrap();
    let mut renderer = Renderer::new(&window);

    // Build the GpuAtlas once (stored alongside renderer).
    let mut atlas = GpuAtlas::new(
        renderer.device(),
        renderer.atlas_bind_group_layout(),
        FONT_BYTES.to_vec(),
        FONT_PX,
    );

    // Measure cell metrics using the same font bytes.
    let metrics: CellMetrics = measure(FONT_BYTES, FONT_PX);

    // Compute initial rows/cols from window size.
    let inner = window.inner_size();
    let mut rows = ((inner.height as f32) / metrics.cell_h).floor() as u16;
    let mut cols = ((inner.width as f32) / metrics.cell_w).floor() as u16;
    rows = rows.max(1);
    cols = cols.max(1);

    // Spawn session; wakeup proxy sends PtyOutput into the event loop.
    let proxy = event_loop.create_proxy();
    let mut session = Session::spawn(rows, cols, "cmd.exe", move || {
        let _ = proxy.send_event(UserEvent::PtyOutput);
    });

    // Request an initial draw so the window isn't blank at startup.
    window.request_redraw();

    event_loop
        .run(move |event, elwt| {
            match event {
                // ── PTY wakeup ──────────────────────────────────────────────
                Event::UserEvent(UserEvent::PtyOutput) => {
                    window.request_redraw();
                }

                Event::WindowEvent { event, .. } => match event {
                    // ── Close ───────────────────────────────────────────────
                    WindowEvent::CloseRequested => elwt.exit(),

                    // ── Resize ──────────────────────────────────────────────
                    WindowEvent::Resized(size) => {
                        renderer.resize(size);
                        let new_rows = ((size.height as f32) / metrics.cell_h).floor() as u16;
                        let new_cols = ((size.width as f32) / metrics.cell_w).floor() as u16;
                        rows = new_rows.max(1);
                        cols = new_cols.max(1);
                        session.resize(rows, cols);
                        window.request_redraw();
                    }

                    // ── Keyboard ─────────────────────────────────────────────
                    WindowEvent::KeyboardInput { event, .. } => {
                        // Only act on key-press (and key-repeat), not key-release.
                        if event.state == ElementState::Released {
                            return;
                        }

                        // Map special keys first, then fall through to text.
                        let bytes: Option<&[u8]> = match &event.logical_key {
                            // Ctrl-C: send ETX regardless of the text field.
                            Key::Character(s) if s.as_str() == "\x03" => Some(b"\x03"),
                            // Enter → CR (0x0d).
                            Key::Named(NamedKey::Enter) => Some(b"\r"),
                            // Backspace → DEL (0x7f).
                            Key::Named(NamedKey::Backspace) => Some(b"\x7f"),
                            _ => None,
                        };

                        if let Some(b) = bytes {
                            session.write(b);
                            window.request_redraw();
                        } else if let Some(text) = &event.text {
                            // text is a SmolStr; skip control chars that
                            // would otherwise double-fire (Enter, Backspace).
                            let s = text.as_str();
                            if !s.is_empty() {
                                session.write(s.as_bytes());
                                window.request_redraw();
                            }
                        }
                    }

                    // ── Redraw ──────────────────────────────────────────────
                    WindowEvent::RedrawRequested => {
                        // 1. Snapshot the terminal grid.
                        let cells =
                            grid_to_cells(&session, rows as usize, cols as usize);

                        // 2. Collect distinct non-space, non-NUL chars.
                        let mut distinct: std::collections::HashSet<char> =
                            std::collections::HashSet::new();
                        for row in &cells {
                            for cell in row {
                                if cell.ch != ' ' && cell.ch != '\0' {
                                    distinct.insert(cell.ch);
                                }
                            }
                        }

                        // 3. Pre-resolve UVs (mutates atlas + uploads glyphs).
                        //    Finish this loop before calling draw_quads so the
                        //    queue borrow is dropped.
                        let queue = renderer.queue();
                        let uv_map: std::collections::HashMap<
                            char,
                            ([f32; 2], [f32; 2]),
                        > = distinct
                            .iter()
                            .map(|&ch| (ch, atlas.uv_for(queue, ch)))
                            .collect();

                        // 4. Build quad instances.
                        let default_uv = ([0.0f32; 2], [0.0f32; 2]);
                        let (bg, glyphs) = build_instances(
                            &cells,
                            &metrics,
                            [0.0, 0.0],
                            &|ch| uv_map.get(&ch).copied().unwrap_or(default_uv),
                        );

                        // 5. Draw.
                        renderer.draw_quads(&bg, &glyphs, &atlas);
                    }

                    _ => {}
                },

                _ => {}
            }
        })
        .unwrap();
}
