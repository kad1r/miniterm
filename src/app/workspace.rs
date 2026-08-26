#![allow(dead_code)]
use crate::app::Tab;
use crate::layout::tree::Rect;
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
        App { workspaces: vec![ws], active: 0, metrics, gutter, root_rect }
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

    fn relayout_active(&mut self) {
        let r = self.root_rect;
        self.active_tab_mut().relayout(r);
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
}
