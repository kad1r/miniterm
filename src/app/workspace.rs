use crate::app::{pane_area_rect, sidebar_row_rect, tab_chip_rect, PAD, SIDEBAR_W, TAB_BAR_H, Tab};
use crate::layout::tree::Rect;
use crate::render::atlas_gpu::GpuAtlas;
use crate::render::grid_draw::{GlyphInfo, QuadInstance};
use crate::render::text_draw::build_text;
use crate::terminal::session::Session;
use crate::text::metrics::CellMetrics;

pub struct Workspace {
    pub name: String,
    pub tabs: Vec<Tab>,
    pub active_tab: usize,
}

pub struct App {
    pub workspaces: Vec<Workspace>,
    pub active: usize,
    pub metrics: CellMetrics,
    pub gutter: f32,
    root_rect: Rect,
    pub editing: Option<usize>,
    pub edit_buf: String,
}

impl App {
    pub fn new(
        root_rect: Rect,
        metrics: CellMetrics,
        spawn: impl FnMut(u16, u16) -> Session,
    ) -> App {
        let gutter = 4.0;
        let tab = Tab::new(root_rect, metrics, gutter, spawn);
        let ws = Workspace { name: "1".to_string(), tabs: vec![tab], active_tab: 0 };
        App { workspaces: vec![ws], active: 0, metrics, gutter, root_rect, editing: None, edit_buf: String::new() }
    }

    pub fn set_root_rect(&mut self, r: Rect) { self.root_rect = r; }

    pub fn active_ws(&self) -> &Workspace { &self.workspaces[self.active] }
    pub fn active_ws_mut(&mut self) -> &mut Workspace { &mut self.workspaces[self.active] }

    pub fn active_tab(&self) -> &Tab {
        let ws = self.active_ws();
        &ws.tabs[ws.active_tab]
    }
    pub fn active_tab_mut(&mut self) -> &mut Tab {
        let a = self.active;
        let ws = &mut self.workspaces[a];
        let t = ws.active_tab;
        &mut ws.tabs[t]
    }

    pub fn new_workspace(&mut self, spawn: impl FnMut(u16, u16) -> Session) {
        let tab = Tab::new(self.root_rect, self.metrics, self.gutter, spawn);
        let name = format!("{}", self.workspaces.len() + 1);
        self.workspaces.push(Workspace { name, tabs: vec![tab], active_tab: 0 });
        self.active = self.workspaces.len() - 1;
    }

    pub fn next_workspace(&mut self) {
        if self.workspaces.len() <= 1 { return; }
        self.active = (self.active + 1) % self.workspaces.len();
        self.relayout_active();
    }
    pub fn prev_workspace(&mut self) {
        if self.workspaces.len() <= 1 { return; }
        self.active = (self.active + self.workspaces.len() - 1) % self.workspaces.len();
        self.relayout_active();
    }
    pub fn switch_workspace(&mut self, idx: usize) {
        if idx < self.workspaces.len() {
            self.active = idx;
            self.relayout_active();
        }
    }

    pub fn new_tab(&mut self, spawn: impl FnMut(u16, u16) -> Session) {
        let tab = Tab::new(self.root_rect, self.metrics, self.gutter, spawn);
        let ws = self.active_ws_mut();
        ws.tabs.push(tab);
        ws.active_tab = ws.tabs.len() - 1;
    }
    pub fn next_tab(&mut self) {
        let n = self.active_ws().tabs.len();
        if n <= 1 { return; }
        let ws = self.active_ws_mut();
        ws.active_tab = (ws.active_tab + 1) % n;
        self.relayout_active();
    }
    pub fn prev_tab(&mut self) {
        let n = self.active_ws().tabs.len();
        if n <= 1 { return; }
        let ws = self.active_ws_mut();
        ws.active_tab = (ws.active_tab + n - 1) % n;
        self.relayout_active();
    }
    pub fn switch_tab(&mut self, idx: usize) {
        if idx < self.active_ws().tabs.len() {
            self.active_ws_mut().active_tab = idx;
            self.relayout_active();
        }
    }

    pub fn begin_rename(&mut self, ws: usize) {
        if ws < self.workspaces.len() {
            self.edit_buf = self.workspaces[ws].name.clone();
            self.editing = Some(ws);
        }
    }
    pub fn rename_push(&mut self, ch: char) {
        if self.editing.is_some() && !ch.is_control() {
            self.edit_buf.push(ch);
        }
    }
    pub fn rename_backspace(&mut self) {
        if self.editing.is_some() { self.edit_buf.pop(); }
    }
    pub fn rename_commit(&mut self) {
        if let Some(ws) = self.editing.take() {
            let name = self.edit_buf.trim().to_string();
            if !name.is_empty() {
                self.workspaces[ws].name = name;
            }
        }
        self.edit_buf.clear();
    }
    pub fn rename_cancel(&mut self) {
        self.editing = None;
        self.edit_buf.clear();
    }
    pub fn delete_workspace(&mut self, ws: usize) {
        if self.workspaces.len() <= 1 || ws >= self.workspaces.len() { return; }
        self.workspaces.remove(ws);
        if self.active >= self.workspaces.len() {
            self.active = self.workspaces.len() - 1;
        }
        self.editing = None;
        self.edit_buf.clear();
        self.relayout_active();
    }

    fn relayout_active(&mut self) {
        let r = self.root_rect;
        self.active_tab_mut().relayout(r);
    }

    pub fn build_frame(
        &mut self,
        queue: &wgpu::Queue,
        atlas: &mut GpuAtlas,
        window: Rect,
    ) -> (Vec<QuadInstance>, Vec<QuadInstance>) {
        const SIDEBAR_BG: [f32; 4] = [0.10, 0.10, 0.12, 1.0];
        const HILITE: [f32; 4] = [0.20, 0.22, 0.28, 1.0];
        const CHIP_BG: [f32; 4] = [0.13, 0.13, 0.16, 1.0];
        const LABEL: [f32; 3] = [0.85, 0.85, 0.85];

        // 1. Active tab's pane frame (borrows atlas mutably, returns owned Vecs).
        let (mut bg, mut glyphs) = self.active_tab_mut().build_frame(queue, atlas);

        let solid = |r: Rect, color: [f32; 4]| QuadInstance {
            pos: [r.x, r.y],
            size: [r.w, r.h],
            uv_min: [0.0, 0.0],
            uv_max: [0.0, 0.0],
            color,
        };

        // 2. Sidebar panel background (full height).
        bg.push(solid(Rect { x: 0.0, y: 0.0, w: SIDEBAR_W, h: window.h }, SIDEBAR_BG));

        // 3. Tab bar background (right of sidebar).
        bg.push(solid(
            Rect { x: SIDEBAR_W, y: 0.0, w: (window.w - SIDEBAR_W).max(0.0), h: TAB_BAR_H },
            SIDEBAR_BG,
        ));

        // Collect label strings and indices before borrowing atlas.
        let ws_labels: Vec<String> = self.workspaces.iter().map(|w| w.name.clone()).collect();
        let active_ws = self.active;
        let tab_count = self.active_ws().tabs.len();
        let active_tab_idx = self.active_ws().active_tab;
        let metrics = self.metrics;

        // Resolve UV map for all label characters via the atlas (fresh borrow).
        let mut chars: Vec<char> = Vec::new();
        for s in &ws_labels {
            chars.extend(s.chars());
        }
        if self.editing.is_some() {
            chars.extend(self.edit_buf.chars());
            chars.push('_');
        }
        chars.push('+');
        for i in 0..tab_count {
            chars.extend(format!("{}", i + 1).chars());
        }
        let mut uv_map: std::collections::HashMap<char, GlyphInfo> =
            std::collections::HashMap::new();
        for c in chars {
            if c != ' ' && !uv_map.contains_key(&c) {
                uv_map.insert(c, atlas.uv_for(queue, c));
            }
        }
        let dg = GlyphInfo {
            uv_min: [0.0, 0.0],
            uv_max: [0.0, 0.0],
            px_size: [0.0, 0.0],
            offset: [0.0, 0.0],
        };
        let lookup = |c: char| uv_map.get(&c).copied().unwrap_or(dg);

        // 4. Sidebar workspace rows.
        let editing_idx = self.editing;
        let edit_buf_snapshot = self.edit_buf.clone();
        for (i, name) in ws_labels.iter().enumerate() {
            let r = sidebar_row_rect(i);
            if i == active_ws {
                bg.push(solid(r, HILITE));
            }
            let ty = r.y + (r.h - metrics.cell_h) * 0.5;
            let shown = if editing_idx == Some(i) {
                format!("{}_", edit_buf_snapshot)
            } else {
                name.clone()
            };
            glyphs.extend(build_text(&shown, &metrics, [r.x + PAD, ty], LABEL, &lookup));
        }
        // "+ new workspace" row.
        let plus_r = sidebar_row_rect(ws_labels.len());
        let pty = plus_r.y + (plus_r.h - metrics.cell_h) * 0.5;
        glyphs.extend(build_text("+", &metrics, [plus_r.x + PAD, pty], LABEL, &lookup));

        // 5. Tab bar chips for the active workspace.
        for i in 0..tab_count {
            let r = tab_chip_rect(i);
            bg.push(solid(r, if i == active_tab_idx { HILITE } else { CHIP_BG }));
            let ty = r.y + (r.h - metrics.cell_h) * 0.5;
            let title = format!("{}", i + 1);
            glyphs.extend(build_text(&title, &metrics, [r.x + PAD, ty], LABEL, &lookup));
        }
        // "+ new tab" chip.
        let plus_chip = tab_chip_rect(tab_count);
        bg.push(solid(plus_chip, CHIP_BG));
        let cty = plus_chip.y + (plus_chip.h - metrics.cell_h) * 0.5;
        glyphs.extend(build_text("+", &metrics, [plus_chip.x + PAD, cty], LABEL, &lookup));

        (bg, glyphs)
    }

    // Public convenience wrapper mandated by the plan; callers use the free
    // `pane_area_rect` directly, so this method has no in-tree consumer.
    #[allow(dead_code)]
    pub fn pane_area(&self, window: Rect) -> Rect {
        pane_area_rect(window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::tree::Rect;
    use crate::text::metrics::CellMetrics;
    use crate::terminal::session::Session;

    fn spawn_stub(rows: u16, cols: u16) -> Session {
        Session::spawn(rows.max(1), cols.max(1), "cmd.exe", || {})
    }

    fn test_app() -> App {
        let m = CellMetrics { cell_w: 10.0, cell_h: 20.0, ascent: 15.0 };
        App::new(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, m, spawn_stub)
    }

    #[test]
    fn starts_with_one_workspace_one_tab() {
        let app = test_app();
        assert_eq!(app.workspaces.len(), 1);
        assert_eq!(app.active, 0);
        assert_eq!(app.active_ws().tabs.len(), 1);
        assert_eq!(app.active_ws().active_tab, 0);
    }

    #[test]
    fn new_workspace_appends_and_activates() {
        let mut app = test_app();
        app.new_workspace(spawn_stub);
        assert_eq!(app.workspaces.len(), 2);
        assert_eq!(app.active, 1);
    }

    #[test]
    fn next_prev_workspace_wraps() {
        let mut app = test_app();
        app.new_workspace(spawn_stub); // active=1, len=2
        app.next_workspace();
        assert_eq!(app.active, 0);
        app.prev_workspace();
        assert_eq!(app.active, 1);
    }

    #[test]
    fn new_tab_appends_within_active_workspace() {
        let mut app = test_app();
        app.new_tab(spawn_stub);
        assert_eq!(app.active_ws().tabs.len(), 2);
        assert_eq!(app.active_ws().active_tab, 1);
    }

    #[test]
    fn tabs_are_isolated_per_workspace() {
        let mut app = test_app();
        app.new_tab(spawn_stub); // ws0 has 2 tabs
        app.new_workspace(spawn_stub); // ws1 has 1 tab
        assert_eq!(app.workspaces[0].tabs.len(), 2);
        assert_eq!(app.workspaces[1].tabs.len(), 1);
    }

    #[test]
    fn switch_tab_bounds_checked() {
        let mut app = test_app();
        app.switch_tab(99); // out of range -> no-op
        assert_eq!(app.active_ws().active_tab, 0);
    }

    #[test]
    fn rename_commit_updates_name() {
        let mut app = test_app();
        app.begin_rename(0);
        app.rename_backspace(); // clear seeded "1"
        for c in "work".chars() { app.rename_push(c); }
        app.rename_commit();
        assert_eq!(app.workspaces[0].name, "work");
        assert!(app.editing.is_none());
    }

    #[test]
    fn rename_cancel_keeps_old_name() {
        let mut app = test_app();
        let old = app.workspaces[0].name.clone();
        app.begin_rename(0);
        app.rename_push('x');
        app.rename_cancel();
        assert_eq!(app.workspaces[0].name, old);
        assert!(app.editing.is_none());
    }

    #[test]
    fn delete_workspace_refuses_last() {
        let mut app = test_app();
        app.delete_workspace(0);
        assert_eq!(app.workspaces.len(), 1);
    }

    #[test]
    fn delete_workspace_removes_and_fixes_active() {
        let mut app = test_app();
        app.new_workspace(spawn_stub); // active=1, len=2
        app.delete_workspace(1);
        assert_eq!(app.workspaces.len(), 1);
        assert_eq!(app.active, 0);
    }
}
