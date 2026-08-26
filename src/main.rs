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
use winit::event::{ElementState, Event, MouseButton, WindowEvent};
use winit::event_loop::{EventLoop, EventLoopBuilder};
use winit::keyboard::{Key, ModifiersState, NamedKey};
use winit::window::{CursorIcon, WindowBuilder};

/// Fallback font, bundled so the app always has a valid monospace face even if
/// the user's configured terminal font cannot be resolved.
const BUNDLED_FONT: &[u8] = include_bytes!("../assets/font/consola.ttf");
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

    // Resolve the user's configured terminal font (Windows Terminal / console),
    // falling back to the bundled Consolas.
    let (font_bytes, font_label) = text::font_source::resolve_font(BUNDLED_FONT);
    eprintln!("[miniterm] font: {font_label}");

    // Build the GpuAtlas once (stored alongside renderer).
    let mut atlas = GpuAtlas::new(
        renderer.device(),
        renderer.atlas_bind_group_layout(),
        font_bytes.clone(),
        FONT_PX,
    );

    // Measure cell metrics using the same font bytes.
    let metrics: CellMetrics = measure(&font_bytes, FONT_PX);

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
    app.set_root_rect(root_rect);

    // Track modifier state (updated by ModifiersChanged events).
    let mut mods = ModifiersState::empty();

    // Mouse drag state.
    let mut cursor_pos = (0.0f32, 0.0f32);
    let mut drag: Option<crate::layout::hit::SplitHit> = None;
    let mut last_resize = std::time::Instant::now();

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
                        app.set_root_rect(root_rect);
                        app.active_tab_mut().relayout(root_rect);
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
                                    app.active_tab_mut().split_focused(Dir::Horizontal, root_rect, spawn_one);
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
                                    app.active_tab_mut().split_focused(Dir::Vertical, root_rect, spawn_one);
                                    true
                                }
                                // Ctrl+Shift+W → close focused pane
                                Key::Character(s)
                                    if s.as_str().eq_ignore_ascii_case("w") =>
                                {
                                    app.active_tab_mut().close_focused(root_rect);
                                    true
                                }
                                // Ctrl+Shift+Tab → cycle focus
                                Key::Named(NamedKey::Tab) => {
                                    app.active_tab_mut().focus_next();
                                    true
                                }
                                // Ctrl+Shift+O → cycle focus (alternate)
                                Key::Character(s)
                                    if s.as_str().eq_ignore_ascii_case("o") =>
                                {
                                    app.active_tab_mut().focus_next();
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
                            let tab = app.active_tab_mut();
                            if let Some(session) = tab.sessions.get_mut(tab.focus) {
                                session.write(b);
                            }
                            window.request_redraw();
                        } else if let Some(text) = &event.text {
                            let s = text.as_str();
                            if !s.is_empty() {
                                let tab = app.active_tab_mut();
                                if let Some(session) = tab.sessions.get_mut(tab.focus) {
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
                        for (_, s) in app.active_tab().sessions.iter() {
                            s.redraw_pending.store(false, std::sync::atomic::Ordering::SeqCst);
                        }

                        // Build frame quads across all panes.
                        // queue() shared borrow ends before draw_quads's &mut self borrow.
                        let (bg, glyphs) = app.active_tab_mut().build_frame(renderer.queue(), &mut atlas);
                        renderer.draw_quads(&bg, &glyphs, &atlas);
                    }

                    // ── Cursor movement ─────────────────────────────────────
                    WindowEvent::CursorMoved { position, .. } => {
                        cursor_pos = (position.x as f32, position.y as f32);
                        let (sw, sh) = renderer.surface_size();
                        let root_rect = Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };
                        if let Some(hit) = &drag {
                            app.active_tab_mut().apply_drag(hit, cursor_pos, root_rect);
                            // Debounce ConPTY resize to ~16ms during a live drag.
                            if last_resize.elapsed().as_millis() >= 16 {
                                app.active_tab_mut().relayout(root_rect);
                                last_resize = std::time::Instant::now();
                            }
                            window.request_redraw();
                        } else {
                            // Set the resize cursor when hovering a divider.
                            let hovering = crate::layout::hit::hit_test(
                                &app.active_tab().tree, root_rect, app.active_tab().gutter, cursor_pos, 3.0,
                            );
                            let icon = match hovering.as_ref().map(|h| h.dir) {
                                Some(crate::layout::tree::Dir::Horizontal) => CursorIcon::EwResize,
                                Some(crate::layout::tree::Dir::Vertical) => CursorIcon::NsResize,
                                None => CursorIcon::Default,
                            };
                            window.set_cursor_icon(icon);
                        }
                    }

                    // ── Mouse buttons ────────────────────────────────────────
                    WindowEvent::MouseInput { state, button, .. }
                        if button == MouseButton::Left =>
                    {
                        match state {
                            ElementState::Pressed => {
                                let (sw, sh) = renderer.surface_size();
                                let root_rect =
                                    Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };
                                drag = crate::layout::hit::hit_test(
                                    &app.active_tab().tree, root_rect, app.active_tab().gutter, cursor_pos, 3.0,
                                );
                                if drag.is_none() {
                                    if let Some(id) = app.active_tab().pane_at_point(cursor_pos) {
                                        app.active_tab_mut().focus = id;
                                        window.request_redraw();
                                    }
                                }
                            }
                            ElementState::Released => {
                                if drag.is_some() {
                                    let (sw, sh) = renderer.surface_size();
                                    let root_rect =
                                        Rect { x: 0.0, y: 0.0, w: sw as f32, h: sh as f32 };
                                    app.active_tab_mut().relayout(root_rect);
                                    window.request_redraw();
                                }
                                drag = None;
                            }
                        }
                    }

                    _ => {}
                },

                _ => {}
            }
        })
        .unwrap();
}
