use slotmap::new_key_type;

new_key_type! {
    pub struct PaneId;
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dir {
    Horizontal, // a | b  (side by side, vertical divider)
    Vertical,   // a / b  (stacked,     horizontal divider)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    A,
    B,
}

pub enum Node {
    Leaf(PaneId),
    Split {
        dir: Dir,
        ratio: f32,
        a: Box<Node>,
        b: Box<Node>,
    },
}

pub struct LayoutTree {
    pub root: Node,
}

impl LayoutTree {
    pub fn new(first: PaneId) -> LayoutTree {
        LayoutTree { root: Node::Leaf(first) }
    }

    /// Split the leaf holding `target`: it becomes child A, `new_pane` becomes B.
    /// Returns false if `target` is not a leaf in the tree.
    pub fn split(&mut self, target: PaneId, new_pane: PaneId, dir: Dir, ratio: f32) -> bool {
        Self::split_node(&mut self.root, target, new_pane, dir, ratio)
    }

    fn split_node(node: &mut Node, target: PaneId, new_pane: PaneId, dir: Dir, ratio: f32) -> bool {
        match node {
            Node::Leaf(id) if *id == target => {
                let a = Node::Leaf(target);
                let b = Node::Leaf(new_pane);
                *node = Node::Split { dir, ratio, a: Box::new(a), b: Box::new(b) };
                true
            }
            Node::Leaf(_) => false,
            Node::Split { a, b, .. } => {
                Self::split_node(a, target, new_pane, dir, ratio)
                    || Self::split_node(b, target, new_pane, dir, ratio)
            }
        }
    }

    /// Remove the leaf holding `target`; its sibling collapses into the parent.
    /// Returns false if `target` is the sole remaining pane or not found.
    pub fn close(&mut self, target: PaneId) -> bool {
        if matches!(&self.root, Node::Leaf(_)) {
            return false; // sole pane — refuse
        }
        Self::close_node(&mut self.root, target)
    }

    fn close_node(node: &mut Node, target: PaneId) -> bool {
        if let Node::Split { a, b, .. } = node {
            // If either direct child is the target leaf, replace self with the sibling.
            let a_is = matches!(a.as_ref(), Node::Leaf(id) if *id == target);
            let b_is = matches!(b.as_ref(), Node::Leaf(id) if *id == target);
            if a_is {
                let sibling = std::mem::replace(b.as_mut(), Node::Leaf(target));
                *node = sibling;
                return true;
            }
            if b_is {
                let sibling = std::mem::replace(a.as_mut(), Node::Leaf(target));
                *node = sibling;
                return true;
            }
            // Recurse.
            let (a, b) = match node {
                Node::Split { a, b, .. } => (a, b),
                _ => unreachable!(),
            };
            return Self::close_node(a, target) || Self::close_node(b, target);
        }
        false
    }

    pub fn compute_rects(&self, root: Rect, gutter: f32) -> Vec<(PaneId, Rect)> {
        let mut out = Vec::new();
        Self::assign(&self.root, root, gutter, &mut out);
        out
    }

    fn assign(node: &Node, rect: Rect, gutter: f32, out: &mut Vec<(PaneId, Rect)>) {
        match node {
            Node::Leaf(id) => out.push((*id, rect)),
            Node::Split { dir, ratio, a, b } => {
                let (ra, rb) = split_rect(rect, *dir, *ratio, gutter);
                Self::assign(a, ra, gutter, out);
                Self::assign(b, rb, gutter, out);
            }
        }
    }

    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut out = Vec::new();
        Self::collect_ids(&self.root, &mut out);
        out
    }

    fn collect_ids(node: &Node, out: &mut Vec<PaneId>) {
        match node {
            Node::Leaf(id) => out.push(*id),
            Node::Split { a, b, .. } => {
                Self::collect_ids(a, out);
                Self::collect_ids(b, out);
            }
        }
    }

    pub fn set_split_ratio(&mut self, path: &[Side], ratio: f32) -> bool {
        let mut node = &mut self.root;
        for side in path {
            match node {
                Node::Split { a, b, .. } => {
                    node = match side {
                        Side::A => a.as_mut(),
                        Side::B => b.as_mut(),
                    };
                }
                Node::Leaf(_) => return false,
            }
        }
        match node {
            Node::Split { ratio: r, .. } => {
                *r = ratio.clamp(0.0, 1.0);
                true
            }
            Node::Leaf(_) => false,
        }
    }

    pub fn clamp_ratio_for(
        root: Rect,
        dir: Dir,
        gutter: f32,
        min_w: f32,
        min_h: f32,
        ratio: f32,
    ) -> f32 {
        let (avail, min_child) = match dir {
            Dir::Horizontal => ((root.w - gutter).max(1.0), min_w),
            Dir::Vertical => ((root.h - gutter).max(1.0), min_h),
        };
        let lo = (min_child / avail).clamp(0.0, 1.0);
        let hi = (1.0 - min_child / avail).clamp(0.0, 1.0);
        if lo > hi {
            0.5
        } else {
            ratio.clamp(lo, hi)
        }
    }
}

/// Divide `rect` along `dir` by `ratio`, reserving `gutter` px between children.
pub fn split_rect(rect: Rect, dir: Dir, ratio: f32, gutter: f32) -> (Rect, Rect) {
    match dir {
        Dir::Horizontal => {
            let avail = (rect.w - gutter).max(0.0);
            let wa = (avail * ratio).max(0.0);
            let wb = (avail - wa).max(0.0);
            (
                Rect { x: rect.x, y: rect.y, w: wa, h: rect.h },
                Rect { x: rect.x + wa + gutter, y: rect.y, w: wb, h: rect.h },
            )
        }
        Dir::Vertical => {
            let avail = (rect.h - gutter).max(0.0);
            let ha = (avail * ratio).max(0.0);
            let hb = (avail - ha).max(0.0);
            (
                Rect { x: rect.x, y: rect.y, w: rect.w, h: ha },
                Rect { x: rect.x, y: rect.y + ha + gutter, w: rect.w, h: hb },
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use slotmap::SlotMap;

    fn approx(a: Rect, b: Rect) -> bool {
        (a.x - b.x).abs() < 0.01
            && (a.y - b.y).abs() < 0.01
            && (a.w - b.w).abs() < 0.01
            && (a.h - b.h).abs() < 0.01
    }

    #[test]
    fn single_leaf_fills_root_rect() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let p = sm.insert(());
        let tree = LayoutTree::new(p);
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, 4.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, p);
        assert!(approx(rects[0].1, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }));
    }

    #[test]
    fn horizontal_split_halves_width_minus_gutter() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        assert!(tree.split(a, b, Dir::Horizontal, 0.5));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 }, 4.0);
        // total width 804, gutter 4 => each pane 400 wide.
        let ra = rects.iter().find(|(id, _)| *id == a).unwrap().1;
        let rb = rects.iter().find(|(id, _)| *id == b).unwrap().1;
        assert!(approx(ra, Rect { x: 0.0, y: 0.0, w: 400.0, h: 600.0 }));
        assert!(approx(rb, Rect { x: 404.0, y: 0.0, w: 400.0, h: 600.0 }));
    }

    #[test]
    fn vertical_split_halves_height_minus_gutter() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        assert!(tree.split(a, b, Dir::Vertical, 0.5));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 800.0, h: 604.0 }, 4.0);
        let ra = rects.iter().find(|(id, _)| *id == a).unwrap().1;
        let rb = rects.iter().find(|(id, _)| *id == b).unwrap().1;
        assert!(approx(ra, Rect { x: 0.0, y: 0.0, w: 800.0, h: 300.0 }));
        assert!(approx(rb, Rect { x: 0.0, y: 304.0, w: 800.0, h: 300.0 }));
    }

    #[test]
    fn close_collapses_sibling_into_parent_rect() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        assert!(tree.close(b));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }, 4.0);
        assert_eq!(rects.len(), 1);
        assert_eq!(rects[0].0, a);
        assert!(approx(rects[0].1, Rect { x: 0.0, y: 0.0, w: 800.0, h: 600.0 }));
    }

    #[test]
    fn cannot_close_last_pane() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let mut tree = LayoutTree::new(a);
        assert!(!tree.close(a));
    }

    #[test]
    fn pane_ids_lists_all_leaves() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        tree.split(b, c, Dir::Vertical, 0.5);
        let mut ids = tree.pane_ids();
        ids.sort_by_key(|k| format!("{:?}", k));
        let mut expected = vec![a, b, c];
        expected.sort_by_key(|k| format!("{:?}", k));
        assert_eq!(ids, expected);
    }
}

#[cfg(test)]
mod drag_tests {
    use super::*;
    use slotmap::SlotMap;

    #[test]
    fn set_split_ratio_updates_root_split() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5);
        // Root split is reached by an empty path.
        assert!(tree.set_split_ratio(&[], 0.25));
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 804.0, h: 600.0 }, 4.0);
        let ra = rects.iter().find(|(id, _)| *id == a).unwrap().1;
        // avail 800 * 0.25 = 200.
        assert!((ra.w - 200.0).abs() < 0.01);
    }

    #[test]
    fn set_split_ratio_follows_path_into_nested_split() {
        let mut sm: SlotMap<PaneId, ()> = SlotMap::with_key();
        let a = sm.insert(());
        let b = sm.insert(());
        let c = sm.insert(());
        let mut tree = LayoutTree::new(a);
        tree.split(a, b, Dir::Horizontal, 0.5); // root split; a=A, b=B
        tree.split(b, c, Dir::Vertical, 0.5);   // b's leaf becomes a nested split at path [B]
        assert!(tree.set_split_ratio(&[Side::B], 0.75));
        // The nested split now favours its A child (b) at 0.75 of the height.
        let rects = tree.compute_rects(Rect { x: 0.0, y: 0.0, w: 804.0, h: 604.0 }, 4.0);
        let rb = rects.iter().find(|(id, _)| *id == b).unwrap().1;
        // nested avail height 600 * 0.75 = 450.
        assert!((rb.h - 450.0).abs() < 0.01);
    }

    #[test]
    fn clamp_keeps_both_children_above_min_width() {
        // root 200 wide, gutter 4 => avail 196, min_w 40.
        // ratio 0.01 would give A=1.96px < 40 => clamp up to 40/196.
        let clamped = LayoutTree::clamp_ratio_for(
            Rect { x: 0.0, y: 0.0, w: 200.0, h: 100.0 },
            Dir::Horizontal,
            4.0,
            40.0,
            10.0,
            0.01,
        );
        let a_w = 196.0 * clamped;
        let b_w = 196.0 - a_w;
        assert!(a_w >= 40.0 - 0.01);
        assert!(b_w >= 40.0 - 0.01);
    }
}
