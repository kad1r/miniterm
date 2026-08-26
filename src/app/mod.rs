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
