use super::SeatId;

/// Row = web `v` (left/right); Column = web `h` (top/bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Row,
    Column,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Branch {
    A,
    B,
}
pub type NodePath = Vec<Branch>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}
impl Side {
    pub fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Row,
            _ => Axis::Column,
        }
    }
    pub fn branch(self) -> Branch {
        match self {
            Self::Left | Self::Top => Branch::A,
            _ => Branch::B,
        }
    }
}

pub fn clamp_ratio(ratio: f32) -> f32 {
    if ratio.is_finite() {
        ratio.clamp(0.1, 0.9)
    } else {
        0.5
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SplitNode {
    Leaf(SeatId),
    Split {
        dir: Axis,
        ratio: f32,
        a: Box<Self>,
        b: Box<Self>,
    },
}
impl SplitNode {
    pub fn split(dir: Axis, ratio: f32, a: Self, b: Self) -> Self {
        Self::Split {
            dir,
            ratio: clamp_ratio(ratio),
            a: Box::new(a),
            b: Box::new(b),
        }
    }
    pub fn leaves(&self) -> Vec<SeatId> {
        match self {
            Self::Leaf(id) => vec![*id],
            Self::Split { a, b, .. } => [a.leaves(), b.leaves()].concat(),
        }
    }
    pub fn contains(&self, seat: SeatId) -> bool {
        match self {
            Self::Leaf(id) => *id == seat,
            Self::Split { a, b, .. } => a.contains(seat) || b.contains(seat),
        }
    }
}

/// Remove a leaf and collapse its empty ancestor. Surviving ratios stay intact.
pub fn prune(tree: &SplitNode, seat: SeatId) -> Option<SplitNode> {
    match tree {
        SplitNode::Leaf(id) => (*id != seat).then_some(tree.clone()),
        SplitNode::Split { dir, ratio, a, b } => match (prune(a, seat), prune(b, seat)) {
            (Some(a), Some(b)) => Some(SplitNode::split(*dir, *ratio, a, b)),
            (a, b) => a.or(b),
        },
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parent {
    pub path: NodePath,
    pub dir: Axis,
    pub ratio: f32,
    pub side: Branch,
    pub sibling: SplitNode,
}

pub fn parent_of(tree: &SplitNode, seat: SeatId) -> Option<Parent> {
    fn walk(tree: &SplitNode, seat: SeatId, path: &mut NodePath) -> Option<Parent> {
        let SplitNode::Split { dir, ratio, a, b } = tree else {
            return None;
        };
        for (side, child, sibling) in [(Branch::A, a, b), (Branch::B, b, a)] {
            if matches!(child.as_ref(), SplitNode::Leaf(id) if *id == seat) {
                return Some(Parent {
                    path: path.clone(),
                    dir: *dir,
                    ratio: *ratio,
                    side,
                    sibling: sibling.as_ref().clone(),
                });
            }
            path.push(side);
            if let Some(parent) = walk(child, seat, path) {
                return Some(parent);
            }
            path.pop();
        }
        None
    }
    walk(tree, seat, &mut Vec::new())
}

pub fn node_at<'a>(tree: &'a SplitNode, path: &[Branch]) -> Option<&'a SplitNode> {
    match path.split_first() {
        None => Some(tree),
        Some((branch, rest)) => match tree {
            SplitNode::Split { a, b, .. } => {
                node_at(if *branch == Branch::A { a } else { b }, rest)
            }
            SplitNode::Leaf(_) => None,
        },
    }
}

/// None means a stale path. The input tree is never partially modified.
pub fn replace_node(
    tree: &SplitNode,
    path: &[Branch],
    replacement: SplitNode,
) -> Option<SplitNode> {
    let Some((branch, rest)) = path.split_first() else {
        return Some(replacement);
    };
    let SplitNode::Split { dir, ratio, a, b } = tree else {
        return None;
    };
    Some(match branch {
        Branch::A => SplitNode::split(
            *dir,
            *ratio,
            replace_node(a, rest, replacement)?,
            b.as_ref().clone(),
        ),
        Branch::B => SplitNode::split(
            *dir,
            *ratio,
            a.as_ref().clone(),
            replace_node(b, rest, replacement)?,
        ),
    })
}

pub fn path_to(tree: &SplitNode, seat: SeatId) -> Option<NodePath> {
    match tree {
        SplitNode::Leaf(id) => (*id == seat).then(Vec::new),
        SplitNode::Split { a, b, .. } => path_to(a, seat)
            .map(|mut p| {
                p.insert(0, Branch::A);
                p
            })
            .or_else(|| {
                path_to(b, seat).map(|mut p| {
                    p.insert(0, Branch::B);
                    p
                })
            }),
    }
}

/// Find a surviving sibling by structure/IDs; preserve any intervening resize.
pub(crate) fn sibling_path(tree: &SplitNode, sibling: &SplitNode) -> Option<NodePath> {
    fn same(a: &SplitNode, b: &SplitNode) -> bool {
        match (a, b) {
            (SplitNode::Leaf(a), SplitNode::Leaf(b)) => a == b,
            (
                SplitNode::Split {
                    dir: ad,
                    a: aa,
                    b: ab,
                    ..
                },
                SplitNode::Split {
                    dir: bd,
                    a: ba,
                    b: bb,
                    ..
                },
            ) => ad == bd && same(aa, ba) && same(ab, bb),
            _ => false,
        }
    }
    if same(tree, sibling) {
        return Some(Vec::new());
    }
    let SplitNode::Split { a, b, .. } = tree else {
        return None;
    };
    sibling_path(a, sibling)
        .map(|mut p| {
            p.insert(0, Branch::A);
            p
        })
        .or_else(|| {
            sibling_path(b, sibling).map(|mut p| {
                p.insert(0, Branch::B);
                p
            })
        })
}
