use crate::layout::tree::{Dir, LayoutTree, Node, Rect, Side, split_rect};

pub struct SplitHit {
    pub path: Vec<Side>,
    pub dir: Dir,
}

pub fn hit_test(
    tree: &LayoutTree,
    root: Rect,
    gutter: f32,
    cursor: (f32, f32),
    tol: f32,
) -> Option<SplitHit> {
    let mut path = Vec::new();
    walk(&tree.root, root, gutter, cursor, tol, &mut path)
}

fn walk(
    node: &Node,
    rect: Rect,
    gutter: f32,
    cursor: (f32, f32),
    tol: f32,
    path: &mut Vec<Side>,
) -> Option<SplitHit> {
    if let Node::Split { dir, ratio, a, b } = node {
        let (ra, rb) = split_rect(rect, *dir, *ratio, gutter);

        // Recurse first so the DEEPEST matching split wins.
        path.push(Side::A);
        if let Some(hit) = walk(a, ra, gutter, cursor, tol, path) {
            return Some(hit);
        }
        path.pop();

        path.push(Side::B);
        if let Some(hit) = walk(b, rb, gutter, cursor, tol, path) {
            return Some(hit);
        }
        path.pop();

        // Divider band lies between ra and rb.
        let (cx, cy) = cursor;
        let on_divider = match dir {
            Dir::Horizontal => {
                let band_min = ra.x + ra.w - tol;
                let band_max = rb.x + tol;
                cx >= band_min && cx <= band_max && cy >= rect.y && cy <= rect.y + rect.h
            }
            Dir::Vertical => {
                let band_min = ra.y + ra.h - tol;
                let band_max = rb.y + tol;
                cy >= band_min && cy <= band_max && cx >= rect.x && cx <= rect.x + rect.w
            }
        };
        if on_divider {
            return Some(SplitHit { path: path.clone(), dir: *dir });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;
    use crate::layout::tree::PaneId;

    #[test]
    fn cursor_on_vertical_divider_hits_horizontal_split() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        let root = Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 };
        // Divider band sits at x in [400, 404].
        let hit = hit_test(&tree, root, 4.0, (402.0, 300.0), 3.0).expect("expected a hit");
        assert_eq!(hit.dir, Dir::Horizontal);
        assert!(hit.path.is_empty());
    }

    #[test]
    fn cursor_in_pane_interior_misses() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        let root = Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 };
        assert!(hit_test(&tree, root, 4.0, (100.0, 300.0), 3.0).is_none());
    }

    #[test]
    fn nested_split_divider_is_found_with_path() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5); // root: a | b
        tree.split(b, c, Dir::Vertical, 0.5);   // b becomes b / c at path [B]
        let root = Rect { x: 0.0, y: 0.0, w: 804.0, h: 604.0 };
        // b's subtree occupies x in [404, 804], full height 604.
        // Its vertical split divider band sits at y in [300, 304].
        let hit = hit_test(&tree, root, 4.0, (600.0, 302.0), 3.0).expect("expected nested hit");
        assert_eq!(hit.dir, Dir::Vertical);
        assert_eq!(hit.path, vec![Side::B]);
    }
}