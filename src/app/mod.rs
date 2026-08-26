mod tab;
mod workspace;
pub use tab::Tab;
#[allow(unused_imports)]
pub use workspace::{App, Workspace};

use crate::layout::tree::Rect;

pub const SIDEBAR_W: f32 = 180.0;
pub const TAB_BAR_H: f32 = 28.0;
pub const ROW_H: f32 = 24.0;
pub const CHIP_W: f32 = 120.0;
pub const PAD: f32 = 6.0;

pub fn pane_area_rect(window: Rect) -> Rect {
    Rect {
        x: SIDEBAR_W,
        y: TAB_BAR_H,
        w: (window.w - SIDEBAR_W).max(0.0),
        h: (window.h - TAB_BAR_H).max(0.0),
    }
}

pub fn sidebar_row_rect(i: usize) -> Rect {
    Rect { x: 0.0, y: PAD + i as f32 * ROW_H, w: SIDEBAR_W, h: ROW_H }
}

pub fn tab_chip_rect(i: usize) -> Rect {
    Rect { x: SIDEBAR_W + PAD + i as f32 * CHIP_W, y: 0.0, w: CHIP_W - PAD, h: TAB_BAR_H }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeHit {
    Workspace(usize),
    NewWorkspace,
    Tab(usize),
    NewTab,
    PaneArea,
}

pub fn chrome_hit(window: Rect, ws_count: usize, tab_count: usize, p: (f32, f32)) -> ChromeHit {
    if p.0 < SIDEBAR_W {
        // Sidebar: which row?
        if p.1 >= PAD {
            let i = ((p.1 - PAD) / ROW_H).floor() as usize;
            if i < ws_count {
                return ChromeHit::Workspace(i);
            } else if i == ws_count {
                return ChromeHit::NewWorkspace;
            }
        }
        return ChromeHit::PaneArea; // empty sidebar space
    }
    if p.1 < TAB_BAR_H {
        for i in 0..=tab_count {
            let r = tab_chip_rect(i);
            if p.0 >= r.x && p.0 <= r.x + r.w {
                return if i < tab_count { ChromeHit::Tab(i) } else { ChromeHit::NewTab };
            }
        }
        return ChromeHit::PaneArea;
    }
    ChromeHit::PaneArea
}

#[cfg(test)]
mod chrome_tests {
    use super::*;
    use crate::layout::tree::Rect;

    fn win() -> Rect { Rect { x: 0.0, y: 0.0, w: 1000.0, h: 700.0 } }

    #[test]
    fn click_first_sidebar_row_is_workspace_0() {
        // row 0 spans y in [PAD, PAD+ROW_H).
        let hit = chrome_hit(win(), 2, 1, (20.0, PAD + 2.0));
        assert!(matches!(hit, ChromeHit::Workspace(0)));
    }

    #[test]
    fn click_new_workspace_row() {
        let hit = chrome_hit(win(), 2, 1, (20.0, PAD + 2.0 * ROW_H + 2.0));
        assert!(matches!(hit, ChromeHit::NewWorkspace));
    }

    #[test]
    fn click_first_tab_chip() {
        // tab bar y < TAB_BAR_H, chip 0 starts at SIDEBAR_W + PAD.
        let hit = chrome_hit(win(), 1, 2, (SIDEBAR_W + PAD + 5.0, 10.0));
        assert!(matches!(hit, ChromeHit::Tab(0)));
    }

    #[test]
    fn click_new_tab_chip() {
        let x = SIDEBAR_W + PAD + 2.0 * CHIP_W + 5.0;
        let hit = chrome_hit(win(), 1, 2, (x, 10.0));
        assert!(matches!(hit, ChromeHit::NewTab));
    }

    #[test]
    fn click_pane_area() {
        let hit = chrome_hit(win(), 1, 1, (400.0, 400.0));
        assert!(matches!(hit, ChromeHit::PaneArea));
    }
}
