mod app;
mod layout;
mod render;
mod terminal;
mod text;

use app::App;
use layout::tree::{Dir, Rect};
use render::atlas_gpu::GpuAtlas;
use render::renderer::Renderer;
use terminal::session::Session;
use text::metrics::{measure, CellMetrics};
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::keyboard::{Key, ModifiersState, NamedKey};
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

    // Build root rect from current surface size.
    let (sw, sh) = renderer.surface_size();
    let root_rect = Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };

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
    let mut app = App::new(root_rect, metrics, spawn);

    // Track modifier state (updated by ModifiersChanged events).
    let mut mods = ModifiersState::empty();

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

                    // ── Modifier tracking ───────────────────────────────────
                    WindowEvent::ModifiersChanged(new_mods) => {
                        mods = new_mods.state();
                    }

                    // ── Resize ──────────────────────────────────────────────
                    WindowEvent::Resized(size) => {
                        renderer.resize(size);
                        let root_rect = Rect {
                            x: 0.0,
                            y: 0.0,
                            w: size.width as f32,
                            h: size.height as f32,
                        };
                        app.relayout(root_rect);
                        window.request_redraw();
                    }

                    // ── Keyboard ─────────────────────────────────────────────
                    WindowEvent::KeyboardInput { event, .. } => {
                        // Only act on key-press (and key-repeat), not key-release.
                        if event.state == ElementState::Released {
                            return;
                        }

                        // Check for Ctrl+Shift chord keybindings first.
                        if mods.control_key() && mods.shift_key() {
                            let (sw, sh) = renderer.surface_size();
                            let root_rect =
                                Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };

                            let handled = match &event.logical_key {
                                // Ctrl+Shift+D → split side-by-side (Horizontal)
                                Key::Character(s)
                                    if s.as_str().eq_ignore_ascii_case("d") =>
                                {
                                    let p = proxy.clone();
                                    let spawn_one = move |rows: u16, cols: u16| -> Session {
                                        let pp = p.clone();
                                        Session::spawn(rows, cols, "cmd.exe", move || {
                                            let _ = pp.send_event(UserEvent::PtyOutput);
                                        })
                                    };
                                    app.split_focused(Dir::Horizontal, root_rect, spawn_one);
                                    true
                                }
                                // Ctrl+Shift+S → split stacked (Vertical)
                                Key::Character(s)
                                    if s.as_str().eq_ignore_ascii_case("s") =>
                                {
                                    let p = proxy.clone();
                                    let spawn_one = move |rows: u16, cols: u16| -> Session {
                                        let pp = p.clone();
                                        Session::spawn(rows, cols, "cmd.exe", move || {
                                            let _ = pp.send_event(UserEvent::PtyOutput);
                                        })
                                    };
                                    app.split_focused(Dir::Vertical, root_rect, spawn_one);
                                    true
                                }
                                // Ctrl+Shift+W → close focused pane
                                Key::Character(s)
                                    if s.as_str().eq_ignore_ascii_case("w") =>
                                {
                                    app.close_focused(root_rect);
                                    true
                                }
                                // Ctrl+Shift+Tab → cycle focus
                                Key::Named(NamedKey::Tab) => {
                                    app.focus_next();
                                    true
                                }
                                // Ctrl+Shift+O → cycle focus (alternate)
                                Key::Character(s)
                                    if s.as_str().eq_ignore_ascii_case("o") =>
                                {
                                    app.focus_next();
                                    true
                                }
                                _ => false,
                            };

                            if handled {
                                window.request_redraw();
                                return;
                            }
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
                            if let Some(session) = app.sessions.get_mut(app.focus) {
                                session.write(b);
                            }
                            window.request_redraw();
                        } else if let Some(text) = &event.text {
                            let s = text.as_str();
                            if !s.is_empty() {
                                if let Some(session) = app.sessions.get_mut(app.focus) {
                                    session.write(s.as_bytes());
                                }
                                window.request_redraw();
                            }
                        }
                    }

                    // ── Redraw ──────────────────────────────────────────────
                    WindowEvent::RedrawRequested => {
                        // Clear every live session's redraw_pending gate before
                        // snapshotting so the next output chunk can queue a new wakeup.
                        for (_, s) in app.sessions.iter() {
                            s.redraw_pending.store(false, std::sync::atomic::Ordering::SeqCst);
                        }

                        // Build frame quads across all panes.
                        // queue() shared borrow ends before draw_quads's &mut self borrow.
                        let (bg, glyphs) = app.build_frame(renderer.queue(), &mut atlas);
                        renderer.draw_quads(&bg, &glyphs, &atlas);
                    }

                    _ => {}
                },

                _ => {}
            }
        })
        .unwrap();
}
