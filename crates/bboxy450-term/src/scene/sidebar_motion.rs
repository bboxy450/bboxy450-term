// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Sidebar drag-and-drop motion. The dragged swarm card or seat row follows the
//! pointer on top of its siblings, the rows it passes slide aside to show where
//! it lands, and after the drop every row eases from where it was drawn into its
//! new slot (FLIP). All of it offsets already solved layout each frame: no extra
//! layout pass, and the GPU paints the shifted geometry as usual.
use super::*;

/// Smoothing time constant for sliding rows, seconds.
const TAU: f32 = 0.07;

#[derive(Debug, Clone, PartialEq)]
pub struct SidebarDrag {
    /// `swarm:<id>` or `seat:<id>`.
    pub key: String,
    /// Pointer travel since the press, logical pixels.
    pub dy: f32,
}

/// One reorderable list: the swarm cards, or one swarm's seat rows.
#[derive(Debug, Clone, Default)]
pub(super) struct Group {
    /// (key, layout name prefix, top, height) in drawn order, before any offset.
    pub items: Vec<(String, String, f32, f32)>,
}

#[derive(Debug, Default)]
pub(crate) struct Motion {
    offsets: BTreeMap<String, f32>,
    visual: BTreeMap<String, f32>,
    groups: Vec<Group>,
    last: Option<Instant>,
}

/// Where the dragged item lands: the key whose slot it takes (`None` when it
/// stays put) and each other item's packed top in the previewed order.
fn preview(group: &Group, dragged: usize, dy: f32) -> (Option<String>, Vec<f32>) {
    let items = &group.items;
    let (_, _, y, h) = &items[dragged];
    let centre = y + h / 2. + dy;
    let others: Vec<usize> = (0..items.len()).filter(|i| *i != dragged).collect();
    let at = others
        .iter()
        .filter(|i| items[**i].2 + items[**i].3 / 2. < centre)
        .count();
    let gap = if items.len() > 1 {
        (items[1].2 - (items[0].2 + items[0].3)).max(0.)
    } else {
        0.
    };
    let mut order = others.clone();
    order.insert(at, dragged);
    let mut top = items[0].2;
    let mut packed = vec![0.; items.len()];
    for i in order {
        packed[i] = top;
        top += items[i].3 + gap;
    }
    let target = (at != dragged).then(|| items[at].0.clone());
    (target, packed)
}

fn shift(node: &mut c::Layout, dy: f32) {
    node.rect.y += dy;
    if let c::LayoutKind::Text { runs, .. } = &mut node.kind {
        for run in runs {
            run.rect.y += dy;
            run.baseline += dy;
        }
    }
    for child in &mut node.children {
        shift(child, dy);
    }
}
fn name(node: &c::Layout) -> Option<&str> {
    match &node.kind {
        c::LayoutKind::Named(name) => Some(name),
        _ => None,
    }
}
/// The bounds of every subtree whose name starts with `prefix`.
fn bounds(node: &c::Layout, prefix: &str, out: &mut Option<(f32, f32)>) {
    if name(node).is_some_and(|n| n.starts_with(prefix)) {
        let (top, bottom) = out.get_or_insert((node.rect.y, node.rect.bottom()));
        *top = top.min(node.rect.y);
        *bottom = bottom.max(node.rect.bottom());
        return;
    }
    for child in &node.children {
        bounds(child, prefix, out);
    }
}
fn shift_prefixed(node: &mut c::Layout, prefix: &str, dy: f32) {
    if name(node).is_some_and(|n| n.starts_with(prefix)) {
        shift(node, dy);
        return;
    }
    for child in &mut node.children {
        shift_prefixed(child, prefix, dy);
    }
}
/// Paint the dragged item over its siblings: its subtrees move to the end of
/// the container that holds them.
fn lift(node: &mut c::Layout, prefix: &str) -> bool {
    let matched: Vec<usize> = (0..node.children.len())
        .filter(|i| name(&node.children[*i]).is_some_and(|n| n.starts_with(prefix)))
        .collect();
    if !matched.is_empty() {
        let mut moved = vec![];
        for i in matched.into_iter().rev() {
            moved.push(node.children.remove(i));
        }
        node.children.extend(moved.into_iter().rev());
        return true;
    }
    node.children.iter_mut().any(|child| lift(child, prefix))
}

impl Scene {
    pub fn set_sidebar_drag(&mut self, drag: Option<SidebarDrag>) {
        self.sidebar_drag = drag;
    }
    pub(super) fn sidebar_motion_active(&self) -> bool {
        self.sidebar_drag.is_some() || !self.sidebar_motion.borrow().offsets.is_empty()
    }
    /// The key the current drag would drop onto, from the last drawn slots.
    pub fn sidebar_drop_target(&self) -> Option<String> {
        let drag = self.sidebar_drag.as_ref()?;
        let motion = self.sidebar_motion.borrow();
        motion.groups.iter().find_map(|group| {
            let index = group.items.iter().position(|item| item.0 == drag.key)?;
            preview(group, index, drag.dy).0
        })
    }
    fn sidebar_groups(&self, sidebar: &c::Layout) -> Vec<Group> {
        let measure = |prefix: String, key: String| {
            let mut out = None;
            bounds(sidebar, &prefix, &mut out);
            out.map(|(top, bottom)| (key, prefix, top, bottom - top))
        };
        let mut groups = vec![Group {
            items: self
                .swarms
                .iter()
                .enumerate()
                .filter_map(|(i, swarm)| {
                    measure(format!("swarm{i}/"), format!("swarm:{}", swarm.id))
                })
                .collect(),
        }];
        for (i, swarm) in self.swarms.iter().enumerate() {
            let mut items = vec![];
            for (j, tab) in swarm.tabs.iter().enumerate() {
                for (k, seat) in tab.seats.iter().enumerate() {
                    items.extend(measure(
                        format!("swarm{i}/body/tab{j}/seat{k}/"),
                        format!("seat:{seat}"),
                    ));
                }
            }
            groups.push(Group { items });
        }
        groups
    }
    /// Offset the solved sidebar for this frame's drag preview and drop easing.
    pub(super) fn animate_sidebar(&self, sidebar: &mut c::Layout, now: Instant) {
        if !self.live || self.swarms.is_empty() {
            return;
        }
        let groups = self.sidebar_groups(sidebar);
        let mut motion = self.sidebar_motion.borrow_mut();
        let dt = motion.last.map_or(0., |last| {
            now.saturating_duration_since(last).as_secs_f32().min(0.05)
        });
        motion.last = Some(now);
        // A changed order (a drop, or a move elsewhere) starts each row from where it
        // was last drawn: FLIP from the old visual top into the new slot.
        // Only a true reorder flips: the same items in another order. Expanding,
        // collapsing or new rows keep the accordion's own motion.
        let reordered: Vec<&Group> = groups
            .iter()
            .zip(&motion.groups)
            .filter(|(a, b)| {
                let keys = |g: &Group| g.items.iter().map(|i| i.0.clone()).collect::<Vec<_>>();
                let (mut x, mut y) = (keys(a), keys(b));
                let moved = x != y;
                x.sort();
                y.sort();
                moved && x == y
            })
            .map(|(a, _)| a)
            .collect();
        let mut flips = vec![];
        for (key, _, top, _) in reordered.iter().flat_map(|g| &g.items) {
            if let Some(was) = motion.visual.get(key) {
                let from = was - top;
                if from.abs() > 0.5 {
                    flips.push((key.clone(), from));
                }
            }
        }
        motion.offsets.extend(flips);
        let mut targets: BTreeMap<String, f32> = BTreeMap::new();
        let mut dragged = None;
        if let Some(drag) = &self.sidebar_drag {
            for group in &groups {
                if let Some(index) = group.items.iter().position(|item| item.0 == drag.key) {
                    let (_, packed) = preview(group, index, drag.dy);
                    for (i, (key, _, top, _)) in group.items.iter().enumerate() {
                        if i != index {
                            targets.insert(key.clone(), packed[i] - top);
                        }
                    }
                    dragged = Some((drag.key.clone(), group.items[index].1.clone(), drag.dy));
                }
            }
        }
        let ease = 1. - (-dt / TAU).exp();
        motion.visual.clear();
        for (key, prefix, top, _) in groups.iter().flat_map(|g| &g.items) {
            let offset = if dragged.as_ref().is_some_and(|d| &d.0 == key) {
                dragged.as_ref().unwrap().2
            } else {
                let target = targets.get(key).copied().unwrap_or(0.);
                let current = motion.offsets.get(key).copied().unwrap_or(0.);
                let next = current + (target - current) * ease;
                if (next - target).abs() < 0.25 && target == 0. {
                    motion.offsets.remove(key);
                    0.
                } else {
                    motion.offsets.insert(key.clone(), next);
                    next
                }
            };
            if offset != 0. {
                shift_prefixed(sidebar, prefix, offset);
            }
            motion.visual.insert(key.clone(), top + offset);
        }
        if let Some((_, prefix, _)) = &dragged {
            lift(sidebar, prefix);
        }
        motion.groups = groups;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn group(heights: &[f32]) -> Group {
        let mut top = 0.;
        Group {
            items: heights
                .iter()
                .enumerate()
                .map(|(i, h)| {
                    let item = (format!("k{i}"), format!("p{i}/"), top, *h);
                    top += h + 4.;
                    item
                })
                .collect(),
        }
    }
    #[test]
    fn preview_takes_the_slot_under_the_dragged_centre_and_packs_the_rest() {
        let g = group(&[20., 20., 20., 20.]);
        // Still: no target, everything where it was.
        assert_eq!(preview(&g, 1, 0.).0, None);
        // Down past the next centre: takes k2's slot, k2 moves up one pitch.
        let (target, packed) = preview(&g, 1, 30.);
        assert_eq!(target.as_deref(), Some("k2"));
        assert_eq!(packed[2], 24.);
        assert_eq!(packed[1], 48.);
        // Up to the top: takes k0's slot and k0 moves down.
        let (target, packed) = preview(&g, 3, -80.);
        assert_eq!(target.as_deref(), Some("k0"));
        assert_eq!((packed[3], packed[0]), (0., 24.));
    }
}
