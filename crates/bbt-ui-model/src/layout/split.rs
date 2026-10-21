use crate::model::{clamp_ratio, Axis, Branch, NodePath, SeatId, SplitNode};
use super::{length, Rect};

#[derive(Debug, Clone, PartialEq)]
pub struct Divider { pub axis: Axis, pub path: NodePath, pub rect: Rect, pub hit_rect: Rect }
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SplitLayout { pub panes: Vec<(SeatId, Rect)>, pub dividers: Vec<Divider> }

/// The same solver serves full panes, mini-grids and launcher previews.
pub fn solve_split(tree: &SplitNode, bounds: Rect, gutter: f32) -> SplitLayout {
    fn walk(tree: &SplitNode, rect: Rect, gutter: f32, path: &mut NodePath, out: &mut SplitLayout) {
        let SplitNode::Split { dir, ratio, a, b } = tree else {
            if let SplitNode::Leaf(id) = tree { out.panes.push((*id, rect)); } return;
        };
        let extent = if *dir == Axis::Row { rect.w } else { rect.h };
        let gap = length(gutter).min(extent);
        let usable = extent - gap;
        let first = usable * clamp_ratio(*ratio);
        let (ar, divider, br) = match dir {
            Axis::Row => (Rect::new(rect.x, rect.y, first, rect.h), Rect::new(rect.x + first, rect.y, gap, rect.h),
                Rect::new(rect.x + first + gap, rect.y, usable - first, rect.h)),
            Axis::Column => (Rect::new(rect.x, rect.y, rect.w, first), Rect::new(rect.x, rect.y + first, rect.w, gap),
                Rect::new(rect.x, rect.y + first + gap, rect.w, usable - first)),
        };
        let hit_rect = match dir {
            Axis::Row => Rect::new(divider.x - 4.0, divider.y, divider.w + 8.0, divider.h),
            Axis::Column => Rect::new(divider.x, divider.y - 4.0, divider.w, divider.h + 8.0),
        };
        out.dividers.push(Divider { axis: *dir, path: path.clone(), rect: divider, hit_rect });
        path.push(Branch::A); walk(a, ar, gutter, path, out); path.pop();
        path.push(Branch::B); walk(b, br, gutter, path, out); path.pop();
    }
    let mut layout = SplitLayout::default();
    walk(tree, bounds.normalized(), gutter, &mut Vec::new(), &mut layout); layout
}
