use crate::render::grid_draw::CellView;
use crate::terminal::session::Session;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};

/// Snapshot the alacritty Term grid into CellView rows.
/// Returns (cells, cursor_point) where cursor_point is (line, col) in grid coords.
/// Monochrome palette for M1; theme colours arrive in the config milestone.
///
/// Fix 2: uses the Term's ACTUAL dimensions (term.columns() / term.screen_lines())
/// instead of trusting the window-derived rows/cols, preventing out-of-bounds panics.
pub fn grid_to_cells(session: &Session) -> (Vec<Vec<CellView>>, (usize, usize)) {
    let term = session.term.lock().unwrap_or_else(|e| e.into_inner());
    let grid = term.grid();

    // Fix 2: use real grid dimensions to avoid indexing past the term's actual size.
    let actual_lines = term.screen_lines();
    let actual_cols = term.columns();

    // Fix 4: capture cursor position while we hold the lock.
    let cursor_pt = grid.cursor.point;
    let cursor_line = cursor_pt.line.0.max(0) as usize;
    let cursor_col = cursor_pt.column.0;

    let mut out = Vec::with_capacity(actual_lines);
    for line in 0..actual_lines {
        let mut row = Vec::with_capacity(actual_cols);
        for col in 0..actual_cols {
            let cell = &grid[Point::new(Line(line as i32), Column(col))];
            row.push(CellView {
                ch: cell.c,
                fg: [0.85, 0.85, 0.85],
                bg: [0.05, 0.05, 0.06],
            });
        }
        out.push(row);
    }
    (out, (cursor_line, cursor_col))
}
