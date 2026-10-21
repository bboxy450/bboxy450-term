use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::time::{Duration, Instant};

use super::*;
use crate::event::Update;
use crate::layout::sidebar::SheetState;
use crate::tokens::{Density, TOKENS, Theme};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TitleBar {
    pub sidebar_shown: bool,
    pub crumb: Option<(SwarmId, SeatId)>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Sidebar {
    pub width: f32,
    pub swarms: Vec<Swarm>,
    pub open: BTreeSet<SwarmId>,
    pub stream_filter: Option<ShadowId>,
    pub scroll: f32,
    pub sheet: SheetState,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stage {
    pub swarm: Option<SwarmId>,
    pub tab: Option<TabId>,
    pub focused: Option<SeatId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    Account,
    Cloud,
    Machines,
    Roles,
    Harnesses,
    Appearance,
    Shortcuts,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTab {
    pub section: Section,
    pub search: TextField,
    pub drafts: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Launcher {
    pub swarm: SwarmId,
    pub preset: Option<Side>,
    pub anchor: Option<SeatId>,
    pub query: TextField,
    pub stack: Vec<RowId>,
    pub selected: usize,
    pub name_seed: u64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PurgeModal {
    pub seat: SeatId,
    pub typed: TextField,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Strip {
    pub seat: SeatId,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaneBody {
    Terminal,
    Settings(SettingsTab),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pane {
    pub seat: SeatId,
    pub strip: Strip,
    pub body: PaneBody,
}
#[derive(Debug, Clone, PartialEq)]
pub struct MiniGrid {
    pub tree: SplitNode,
    pub focused: Option<SeatId>,
    pub big: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub enum Banner {
    Split(SplitNode),
    NewTab(Vec<TabId>),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamDock {
    pub items: Vec<StreamItem>,
    pub filter: Option<ShadowId>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchTier {
    Prefix,
    Facets,
    Subtitle,
    Fuzzy,
    Semantic,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    Launcher(Launcher),
    Purge(PurgeModal),
}
#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub title_bar: TitleBar,
    pub sidebar: Sidebar,
    pub stage: Stage,
    pub overlay: Option<Overlay>,
    pub theme: Theme,
    pub density: Density,
    pub gutter: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub window: Window,
    pub seats: BTreeMap<SeatId, Seat>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditError {
    MissingSeat,
    MissingSwarm,
    MissingTab,
    DuplicateId,
    InvalidTree,
    NotOpen,
    NotClosed,
    CrossSwarm,
    Expired,
    ProcessReplaced,
    ProcessExited,
    LayoutConflict,
    InvalidDeadline,
}
impl fmt::Display for EditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for EditError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    Tab { swarm: SwarmId, tab: TabId },
    Split { target: SeatId, side: Side },
}

impl Model {
    pub fn new(swarms: Vec<Swarm>, seats: Vec<Seat>) -> Result<Self, EditError> {
        let count = seats.len();
        let seats: BTreeMap<_, _> = seats.into_iter().map(|s| (s.id, s)).collect();
        if seats.len() != count {
            return Err(EditError::DuplicateId);
        }
        let mut swarm_ids = BTreeSet::new();
        let mut tab_ids = BTreeSet::new();
        let mut placed = BTreeSet::new();
        for swarm in &swarms {
            if !swarm_ids.insert(swarm.id) {
                return Err(EditError::DuplicateId);
            }
            for tab in &swarm.tabs {
                if !tab_ids.insert(tab.id) {
                    return Err(EditError::DuplicateId);
                }
                let leaves = tab.tree.leaves();
                let leaf_set: BTreeSet<_> = leaves.iter().copied().collect();
                if leaves.len() != leaf_set.len()
                    || leaf_set != tab.seats.iter().copied().collect()
                    || tab.seats.len() != leaves.len()
                {
                    return Err(EditError::InvalidTree);
                }
                for id in &tab.seats {
                    let seat = seats.get(id).ok_or(EditError::MissingSeat)?;
                    if seat.swarm != swarm.id {
                        return Err(EditError::CrossSwarm);
                    }
                    if !placed.insert(*id) {
                        return Err(EditError::DuplicateId);
                    }
                }
            }
        }
        let first = swarms.first();
        let stage = Stage {
            swarm: first.map(|s| s.id),
            tab: first.and_then(|s| s.tabs.first().map(|t| t.id)),
            focused: first
                .and_then(|s| s.tabs.first())
                .and_then(|t| t.seats.first().copied()),
        };
        Ok(Self {
            window: Window {
                title_bar: TitleBar {
                    sidebar_shown: true,
                    crumb: None,
                },
                sidebar: Sidebar {
                    width: TOKENS.sidebar.width as f32,
                    open: first.map(|s| BTreeSet::from([s.id])).unwrap_or_default(),
                    swarms,
                    stream_filter: None,
                    scroll: 0.0,
                    sheet: SheetState::Closed,
                },
                stage,
                overlay: None,
                theme: Theme::Dark,
                density: Density::Cozy,
                gutter: TOKENS.layout.gutter as f32,
            },
            seats,
        })
    }

    pub fn tab_of(&self, seat: SeatId) -> Option<(usize, usize)> {
        self.window
            .sidebar
            .swarms
            .iter()
            .enumerate()
            .find_map(|(si, s)| {
                s.tabs
                    .iter()
                    .position(|t| t.seats.contains(&seat))
                    .map(|ti| (si, ti))
            })
    }

    fn focus(&mut self, seat: SeatId) -> Result<(), EditError> {
        let (si, ti) = self.tab_of(seat).ok_or(EditError::MissingTab)?;
        let swarm = &self.window.sidebar.swarms[si];
        self.window.stage = Stage {
            swarm: Some(swarm.id),
            tab: Some(swarm.tabs[ti].id),
            focused: Some(seat),
        };
        Ok(())
    }

    /// Mux data never emits Locate. Activity/messages preserve focus; removing
    /// the focused seat chooses a surviving fallback without a locate signal.
    pub fn apply(&mut self, update: Update) -> Result<(), EditError> {
        match update {
            Update::SeatActivity {
                seat,
                activity,
                grade,
            } => {
                let seat = self.seats.get_mut(&seat).ok_or(EditError::MissingSeat)?;
                seat.activity = activity;
                seat.grade = grade;
            }
            Update::SeatGeneration { seat, generation } => {
                let seat = self.seats.get_mut(&seat).ok_or(EditError::MissingSeat)?;
                if generation < seat.process_generation {
                    return Err(EditError::ProcessReplaced);
                }
                seat.process_generation = generation;
            }
            Update::MessageArrived { seat, unread, role } => {
                self.seats
                    .get_mut(&seat)
                    .ok_or(EditError::MissingSeat)?
                    .unread = (unread > 0).then_some((unread, role));
            }
            Update::SeatClosedByDaemon { seat } => {
                if !self.seats.contains_key(&seat) {
                    return Err(EditError::MissingSeat);
                }
                detach_inner(self, seat);
                for swarm in &mut self.window.sidebar.swarms {
                    swarm.closed.retain(|c| c.seat != seat);
                }
                self.seats.get_mut(&seat).unwrap().lifecycle = Lifecycle::ClosedRetained;
            }
        }
        Ok(())
    }
}

fn detach_inner(model: &mut Model, seat: SeatId) {
    if let Some((si, ti)) = model.tab_of(seat) {
        let swarm = &mut model.window.sidebar.swarms[si];
        let tab = &mut swarm.tabs[ti];
        if let Some(tree) = prune(&tab.tree, seat) {
            tab.tree = tree;
            tab.seats.retain(|id| *id != seat);
        } else {
            swarm.tabs.remove(ti);
        }
        if model.window.stage.focused == Some(seat) {
            model.window.stage = Stage {
                swarm: Some(swarm.id),
                tab: swarm.tabs.first().map(|t| t.id),
                focused: swarm.tabs.first().and_then(|t| t.seats.first().copied()),
            };
        }
    }
}

pub fn detach(model: &Model, seat: SeatId) -> Result<Model, EditError> {
    if !model.seats.contains_key(&seat) {
        return Err(EditError::MissingSeat);
    }
    let mut next = model.clone();
    detach_inner(&mut next, seat);
    Ok(next)
}

pub fn place(model: &Model, seat: SeatId, placement: Placement) -> Result<Model, EditError> {
    let record = model.seats.get(&seat).ok_or(EditError::MissingSeat)?;
    if record.lifecycle != Lifecycle::Open {
        return Err(EditError::NotOpen);
    }
    let mut next = model.clone();
    match placement {
        Placement::Tab { swarm, tab } => {
            if record.swarm != swarm {
                return Err(EditError::CrossSwarm);
            }
            if model
                .window
                .sidebar
                .swarms
                .iter()
                .any(|s| s.tabs.iter().any(|t| t.id == tab))
            {
                return Err(EditError::DuplicateId);
            }
            detach_inner(&mut next, seat);
            let swarm = next
                .window
                .sidebar
                .swarms
                .iter_mut()
                .find(|s| s.id == swarm)
                .ok_or(EditError::MissingSwarm)?;
            let name = if record.harness.0 == "zsh" {
                "shell".into()
            } else {
                record
                    .name
                    .split_whitespace()
                    .next()
                    .unwrap_or("seat")
                    .to_lowercase()
            };
            let mut new_tab = Tab::new(tab, name, SplitNode::Leaf(seat));
            new_tab.shortcut = (swarm.tabs.len() < 9).then_some((swarm.tabs.len() + 1) as u8);
            swarm.tabs.push(new_tab);
            next.window.sidebar.open.insert(swarm.id);
        }
        Placement::Split { target, side } => {
            if target == seat {
                return Ok(next);
            }
            let (si, _) = model.tab_of(target).ok_or(EditError::MissingTab)?;
            if record.swarm != model.window.sidebar.swarms[si].id {
                return Err(EditError::CrossSwarm);
            }
            detach_inner(&mut next, seat);
            let (si, ti) = next.tab_of(target).ok_or(EditError::MissingTab)?;
            let swarm = &mut next.window.sidebar.swarms[si];
            let tab = &mut swarm.tabs[ti];
            let path = path_to(&tab.tree, target).ok_or(EditError::InvalidTree)?;
            let leaf = SplitNode::Leaf(seat);
            let target = SplitNode::Leaf(target);
            let (a, b) = if side.branch() == Branch::A {
                (leaf, target)
            } else {
                (target, leaf)
            };
            tab.tree = replace_node(&tab.tree, &path, SplitNode::split(side.axis(), 0.5, a, b))
                .ok_or(EditError::InvalidTree)?;
            tab.seats.push(seat);
            next.window.sidebar.open.insert(swarm.id);
        }
    }
    next.focus(seat)?;
    Ok(next)
}

pub fn close(
    model: &Model,
    seat: SeatId,
    now: Instant,
    grace: Duration,
) -> Result<Model, EditError> {
    let record = model.seats.get(&seat).ok_or(EditError::MissingSeat)?;
    if record.lifecycle != Lifecycle::Open {
        return Err(EditError::NotOpen);
    }
    // Settings is a local view, not a live harness: the lab closes it directly.
    if record.settings {
        let mut next = model.clone();
        detach_inner(&mut next, seat);
        next.seats.remove(&seat);
        return Ok(next);
    }
    let (si, ti) = model.tab_of(seat).ok_or(EditError::MissingTab)?;
    let tab = &model.window.sidebar.swarms[si].tabs[ti];
    let until = now.checked_add(grace).ok_or(EditError::InvalidDeadline)?;
    let closed = ClosedStrip {
        seat,
        until,
        undo: UndoRecord {
            tab: tab.id,
            tab_index: ti,
            tab_name: tab.name.clone(),
            shortcut: tab.shortcut,
            expanded: tab.expanded,
            parent: parent_of(&tab.tree, seat),
            process_generation: record.process_generation,
        },
    };
    let mut next = model.clone();
    detach_inner(&mut next, seat);
    next.window.sidebar.swarms[si].closed.push(closed);
    next.seats.get_mut(&seat).unwrap().lifecycle = Lifecycle::ClosedGrace { until };
    Ok(next)
}

pub fn undo(model: &Model, seat: SeatId, now: Instant) -> Result<Model, EditError> {
    let (si, ci) = model
        .window
        .sidebar
        .swarms
        .iter()
        .enumerate()
        .find_map(|(si, s)| {
            s.closed
                .iter()
                .position(|c| c.seat == seat)
                .map(|ci| (si, ci))
        })
        .ok_or_else(|| {
            if model.seats.get(&seat).is_some_and(
                |s| matches!(s.lifecycle, Lifecycle::ClosedGrace { until } if now >= until),
            ) {
                EditError::Expired
            } else {
                EditError::NotClosed
            }
        })?;
    let closed = &model.window.sidebar.swarms[si].closed[ci];
    if now >= closed.until {
        return Err(EditError::Expired);
    }
    let record = model.seats.get(&seat).ok_or(EditError::MissingSeat)?;
    if record.process_generation != closed.undo.process_generation {
        return Err(EditError::ProcessReplaced);
    }
    if !matches!(record.lifecycle, Lifecycle::ClosedGrace { .. }) {
        return Err(EditError::ProcessExited);
    }
    let mut next = model.clone();
    let swarm = &mut next.window.sidebar.swarms[si];
    if let Some(ti) = swarm.tabs.iter().position(|t| t.id == closed.undo.tab) {
        let parent = closed
            .undo
            .parent
            .as_ref()
            .ok_or(EditError::LayoutConflict)?;
        let tab = &mut swarm.tabs[ti];
        let path = sibling_path(&tab.tree, &parent.sibling).ok_or(EditError::LayoutConflict)?;
        let sibling = node_at(&tab.tree, &path)
            .ok_or(EditError::LayoutConflict)?
            .clone();
        let leaf = SplitNode::Leaf(seat);
        let (a, b) = if parent.side == Branch::A {
            (leaf, sibling)
        } else {
            (sibling, leaf)
        };
        tab.tree = replace_node(
            &tab.tree,
            &path,
            SplitNode::split(parent.dir, parent.ratio, a, b),
        )
        .ok_or(EditError::LayoutConflict)?;
        tab.seats.push(seat);
    } else {
        let mut tab = Tab::new(
            closed.undo.tab,
            closed.undo.tab_name.clone(),
            SplitNode::Leaf(seat),
        );
        tab.shortcut = closed.undo.shortcut;
        tab.expanded = closed.undo.expanded;
        swarm
            .tabs
            .insert(closed.undo.tab_index.min(swarm.tabs.len()), tab);
    }
    swarm.closed.remove(ci);
    next.seats.get_mut(&seat).unwrap().lifecycle = Lifecycle::Open;
    next.focus(seat)?;
    Ok(next)
}

pub fn expire_close(model: &Model, seat: SeatId, now: Instant) -> Result<Model, EditError> {
    let mut next = model.clone();
    let closed = next
        .window
        .sidebar
        .swarms
        .iter_mut()
        .find_map(|s| {
            s.closed
                .iter()
                .position(|c| c.seat == seat)
                .map(|ci| (s, ci))
        })
        .ok_or(EditError::NotClosed)?;
    if now < closed.0.closed[closed.1].until {
        return Err(EditError::NotClosed);
    }
    closed.0.closed.remove(closed.1);
    // Expiring undo cannot prove the child terminated. Wait for the mux's
    // SeatClosedByDaemon update before claiming ClosedRetained.
    Ok(next)
}
