use crate::model::{Swarm, SwarmId};
use crate::tokens::{Density, TOKENS};
use super::{length, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind { Tab, Seat, Closed, NewSeat, Shadow }
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowRun { pub kind: RowKind, pub count: usize }
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarCard { pub swarm: SwarmId, pub open: bool, pub rows: Vec<RowRun> }
impl SidebarCard {
    pub fn from_swarm(swarm: &Swarm, open: bool) -> Self {
        let mut rows = Vec::new();
        if open {
            for tab in &swarm.tabs {
                if tab.seats.len() == 1 { rows.push(RowRun { kind: RowKind::Seat, count: 1 }); }
                else {
                    rows.push(RowRun { kind: RowKind::Tab, count: 1 });
                    if tab.expanded { rows.push(RowRun { kind: RowKind::Seat, count: tab.seats.len() }); }
                }
            }
            rows.extend([
                RowRun { kind: RowKind::Closed, count: swarm.closed.len() },
                RowRun { kind: RowKind::NewSeat, count: 1 },
                RowRun { kind: RowKind::Shadow, count: swarm.shadows.len() },
            ]);
        }
        Self { swarm: swarm.id, open, rows }
    }
}

/// The source design lacks band/bumper/padding tokens; the caller must supply
/// those chrome metrics. The solver does not invent replacements in TOKENS.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardChrome { pub band: f32, pub pad_top: f32, pub pad_bottom: f32, pub bumper_open: f32, pub bumper_closed: f32 }
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleRow { pub kind: RowKind, pub run: usize, pub index: usize, pub rect: Rect }
#[derive(Debug, Clone, PartialEq)]
pub struct VisibleCard { pub swarm: SwarmId, pub rect: Rect, pub rows: Vec<VisibleRow> }
#[derive(Debug, Clone, PartialEq)]
pub struct SidebarLayout { pub cards: Vec<VisibleCard>, pub total_height: f32, pub scroll: f32 }

fn row_height(kind: RowKind, density: Density) -> f32 {
    let d = density.tokens();
    if matches!(kind, RowKind::Tab | RowKind::Closed) { d.tab_h as f32 } else { d.row_h as f32 }
}
fn card_height(card: &SidebarCard, density: Density, chrome: CardChrome) -> f32 {
    length(chrome.band) + if card.open {
        length(chrome.pad_top) + length(chrome.pad_bottom) + length(chrome.bumper_open)
            + card.rows.iter().map(|r| row_height(r.kind, density) * r.count as f32).sum::<f32>()
    } else { length(chrome.bumper_closed) }
}

/// O(cards + row runs + visible rows), not O(all seats). Partial rows are kept
/// and clipped by the renderer; hidden rows have no allocated rectangle.
pub fn solve_sidebar(cards: &[SidebarCard], bounds: Rect, scroll: f32, density: Density, chrome: CardChrome) -> SidebarLayout {
    let bounds = bounds.normalized();
    let gap = TOKENS.layout.cohort_gap as f32;
    let total_height = cards.iter().map(|c| card_height(c, density, chrome)).sum::<f32>() + gap * cards.len().saturating_sub(1) as f32;
    let scroll = length(scroll).min(length(total_height - bounds.h));
    let mut layout = SidebarLayout { cards: Vec::new(), total_height, scroll };
    let mut top = bounds.y - scroll;
    for card in cards {
        let height = card_height(card, density, chrome);
        let rect = Rect::new(bounds.x, top, bounds.w, height);
        if rect.intersects(bounds) {
            let mut rows = Vec::new();
            let mut y = top + length(chrome.band) + length(chrome.pad_top);
            if card.open { for (run_index, run) in card.rows.iter().enumerate() {
                let h = row_height(run.kind, density);
                let first = ((bounds.y - y) / h).floor().max(0.0).min(run.count as f32) as usize;
                let end = ((bounds.bottom() - y) / h).ceil().max(0.0).min(run.count as f32) as usize;
                for index in first..end { rows.push(VisibleRow { kind: run.kind, run: run_index, index, rect: Rect::new(bounds.x, y + index as f32 * h, bounds.w, h) }); }
                y += run.count as f32 * h;
            }}
            layout.cards.push(VisibleCard { swarm: card.swarm, rect, rows });
        }
        top += height + gap;
    }
    layout
}

pub fn scroll_into_view(scroll: f32, viewport: f32, row_top: f32, row_height: f32, total: f32) -> f32 {
    let scroll = length(scroll);
    let next = if row_top < scroll { row_top } else if row_top + row_height > scroll + viewport { row_top + row_height - viewport } else { scroll };
    length(next).min(length(total - viewport))
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SheetState { #[default] Closed, Open, Dragging(f32) }
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhoneRules { pub breakpoint: f32, pub height_fraction: f32, pub handle: f32, pub transition_ms: u32 }
impl Default for PhoneRules {
    fn default() -> Self { Self { breakpoint: 600.0, height_fraction: 0.62, handle: 44.0, transition_ms: 320 } }
}
impl SheetState {
    pub fn rect(self, window: Rect, rules: PhoneRules) -> Rect {
        let window = window.normalized();
        let height = window.h * length(rules.height_fraction).min(1.0);
        let max_offset = length(height - length(rules.handle));
        let offset = match self { Self::Closed => max_offset, Self::Open => 0.0, Self::Dragging(offset) => length(offset).min(max_offset) };
        Rect::new(window.x, window.bottom() - height + offset, window.w, height)
    }
    pub fn snap(self, window: Rect, rules: PhoneRules) -> Self {
        let Self::Dragging(offset) = self else { return self; };
        let max_offset = length(length(window.h) * length(rules.height_fraction).min(1.0) - length(rules.handle));
        if length(offset) > max_offset / 2.0 { Self::Closed } else { Self::Open }
    }
    pub fn density(window_width: f32, desktop: Density, rules: PhoneRules) -> Density {
        if window_width < rules.breakpoint { Density::Touch } else { desktop }
    }
}
