//! Pure document navigation and clipped retained hit regions. Hosts own reads.
use crate::components as c;
use bbt_ui_model::layout::Rect;
use std::collections::BTreeMap;

/// An absent index preserves plain wiki chips. Within an index, None is a
/// resolved missing record, rather than a guessed path or a clickable fallback.
pub type RecordLinks = BTreeMap<String, Option<u64>>;
const PREFIX: &str = "record-link:";

pub fn numbered(target: &str) -> Option<u64> {
    let value = target.strip_prefix('#')?;
    if value.starts_with('0') || value.is_empty() || !value.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    value.parse().ok().filter(|n| *n > 0)
}

pub fn presentation(target: &str, display: &str, links: Option<&RecordLinks>) -> (String, bool) {
    if numbered(target).is_none() && numbered(display).is_none() {
        return (display.into(), true);
    }
    match links.and_then(|links| links.get(target)) {
        Some(Some(number)) => (format!("#{number}"), true),
        Some(None) => (
            format!(
                "{} · no record",
                numbered(target).map_or_else(|| display.to_owned(), |n| format!("#{n}"))
            ),
            false,
        ),
        None => (display.to_owned(), true),
    }
}

/// Metadata only: wrapping a link never adds paint, layout or a new authority.
pub fn link(element: c::Element, target: &str, enabled: bool) -> c::Element {
    if enabled {
        c::Element::Named {
            name: format!("{PREFIX}{target}"),
            content: Box::new(element),
        }
    } else {
        element
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Open(String),
    Close,
    Unavailable(&'static str),
}
#[derive(Default)]
pub struct Regions {
    links: Vec<(Rect, Action)>,
    pub viewport: Option<Rect>,
    pub extent: [f32; 2],
}

pub fn intersection(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect::new(
        x,
        y,
        (a.right().min(b.right()) - x).max(0.),
        (a.bottom().min(b.bottom()) - y).max(0.),
    )
}
impl Regions {
    pub fn retained(layout: &c::Layout) -> Self {
        fn visit(n: &c::Layout, clip: Option<Rect>, out: &mut Regions) {
            if let c::LayoutKind::Named(name) = &n.kind {
                let action = name
                    .split_once(PREFIX)
                    .map(|(_, target)| Action::Open(target.into()))
                    .or_else(|| {
                        (name == "action6" || name.ends_with("/action6")).then_some(Action::Close)
                    })
                    .or_else(|| {
                        [
                            ("action0", "feed"),
                            ("action1", "feed selection"),
                            ("action2", "copy"),
                            ("action3", "copy path"),
                            ("action4", "find"),
                            ("action5", "pin"),
                        ]
                        .into_iter()
                        .find(|(key, _)| name == key || name.ends_with(&format!("/{key}")))
                        .map(|(_, label)| Action::Unavailable(label))
                    });
                if let Some(action) = action {
                    let r = clip.map_or(n.rect, |old| intersection(old, n.rect));
                    if r.w > 0. && r.h > 0. {
                        out.links.push((r, action));
                    }
                }
            }
            // The shell owns the first viewport. A nested diagram's horizontal
            // scroller must not replace the document's vertical scroll extent.
            if let c::LayoutKind::ScrollView { content, .. } = &n.kind
                && out.viewport.is_none()
            {
                out.viewport = Some(n.rect);
                out.extent = [
                    (content[0] - n.rect.w).max(0.),
                    (content[1] - n.rect.h).max(0.),
                ];
            }
            let clip = if matches!(
                n.kind,
                c::LayoutKind::ScrollView { .. } | c::LayoutKind::Box { clip: true, .. }
            ) {
                Some(clip.map_or(n.rect, |old| intersection(old, n.rect)))
            } else {
                clip
            };
            for child in &n.children {
                visit(child, clip, out);
            }
        }
        let mut out = Self::default();
        visit(layout, None, &mut out);
        out
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<Action> {
        self.links
            .iter()
            .rev()
            .find(|(r, _)| at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom())
            .map(|(_, a)| a.clone())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Controller {
    pub offset: [f32; 2],
}
impl Controller {
    pub fn scroll(&mut self, delta: [f32; 2], extent: [f32; 2]) -> bool {
        let before = self.offset;
        for ((offset, delta), extent) in self.offset.iter_mut().zip(delta).zip(extent) {
            if delta.is_finite() && extent.is_finite() {
                *offset = (*offset + delta).clamp(0., extent.max(0.));
            }
        }
        self.offset != before
    }
}
