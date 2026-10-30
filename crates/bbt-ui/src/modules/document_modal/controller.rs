// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
    if links.and_then(|links| links.get(target)) == Some(&None) {
        return (format!("{} · no record", numbered(target).map_or_else(|| display.to_owned(), |n| format!("#{n}"))), false);
    }
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
    ToggleProvenance,
    Feed,
    Copy,
    CopyPath,
    Close,
    Unavailable(&'static str),
}
#[derive(Debug, Clone)]
pub struct Horizontal {
    pub key: String,
    pub rect: Rect,
    pub max: f32,
    pub root: bool,
}
#[derive(Default)]
pub struct Regions {
    links: Vec<(Rect, Action)>,
    panel: Option<Rect>,
    pub viewport: Option<Rect>,
    pub extent: [f32; 2],
    horizontal: Vec<Horizontal>,
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
    /// A modal alone owns backdrop dismissal; embedded/pinned documents keep
    /// ordinary retained links without claiming clicks elsewhere in the window.
    /// Origin: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
    pub fn retained_modal(layout: &c::Layout) -> Self {
        Self {
            panel: Some(layout.rect),
            ..Self::retained(layout)
        }
    }
    pub fn retained(layout: &c::Layout) -> Self {
        fn visit<'a>(
            n: &'a c::Layout,
            clip: Option<Rect>,
            scope: Option<&'a str>,
            out: &mut Regions,
        ) {
            let scope = if let c::LayoutKind::Named(name) = &n.kind {
                Some(name.as_str())
            } else {
                scope
            };
            if let c::LayoutKind::Named(name) = &n.kind {
                let action = name
                    .split_once(PREFIX)
                    .map(|(_, target)| Action::Open(target.into()))
                    .or_else(|| {
                        (name == "metadata/provenance/toggle"
                            || name.ends_with("/metadata/provenance/toggle"))
                        .then_some(Action::ToggleProvenance)
                    })
                    .or_else(|| {
                        name.ends_with("/button-source")
                            .then_some(Action::Unavailable("Diagram source"))
                    })
                    .or_else(|| {
                        name.ends_with("/expand")
                            .then_some(Action::Unavailable("Expand diagram"))
                    })
                    .or_else(|| {
                        (name == "action6" || name.ends_with("/action6")).then_some(Action::Close)
                    })
                    .or_else(|| {
                        [("action0",Action::Feed),("action2",Action::Copy),("action3",Action::CopyPath)]
                            .into_iter().find(|(key,_)|name==key || name.ends_with(&format!("/{key}"))).map(|(_,action)|action)
                    })
                    .or_else(|| {
                        [

                            ("action1", "feed selection"),
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
            // The shell owns the first viewport; nested horizontal viewports
            // keep their own retained key, clip and extent.
            if let c::LayoutKind::ScrollView { content, .. } = &n.kind {
                let root = out.viewport.is_none();
                let extent = [
                    (content[0] - n.rect.w).max(0.),
                    (content[1] - n.rect.h).max(0.),
                ];
                if root {
                    out.viewport = Some(n.rect);
                    out.extent = extent;
                }
                let rect = clip.map_or(n.rect, |old| intersection(old, n.rect));
                if extent[0] > 0. && rect.w > 0. && rect.h > 0. {
                    out.horizontal.push(Horizontal {
                        key: scope.unwrap_or("scroller").into(),
                        rect,
                        max: extent[0],
                        root,
                    });
                }
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
                visit(child, clip, scope, out);
            }
        }
        let mut out = Self::default();
        visit(layout, None, None, &mut out);
        out
    }
    pub fn horizontal_at(&self, at: [f32; 2]) -> Option<&Horizontal> {
        self.horizontal.iter().rev().find(|r| {
            at[0] >= r.rect.x
                && at[0] < r.rect.right()
                && at[1] >= r.rect.y
                && at[1] < r.rect.bottom()
        })
    }
    pub fn horizontal(&self) -> &[Horizontal] {
        &self.horizontal
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<Action> {
        if !at.into_iter().all(f32::is_finite) {
            return None;
        }
        if self.panel.is_some_and(|r| {
            at[0] < r.x || at[0] >= r.right() || at[1] < r.y || at[1] >= r.bottom()
        }) {
            return Some(Action::Close);
        }
        self.links
            .iter()
            .rev()
            .find(|(r, _)| at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom())
            .map(|(_, a)| a.clone())
    }
    /// Missing links, unavailable commands and clipped content keep the arrow.
    pub fn cursor(&self, at: [f32; 2]) -> crate::cursor::Cursor {
        use crate::cursor::Cursor;
        match self.hit(at) {
            Some(Action::Open(_) | Action::ToggleProvenance | Action::Feed | Action::Copy | Action::CopyPath | Action::Close) => Cursor::Pointer,
            Some(Action::Unavailable(_)) | None => Cursor::Default,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Controller {
    pub offset: [f32; 2],
    /// Display state only; each newly opened document starts collapsed.
    pub provenance_expanded: bool,
    horizontal: BTreeMap<String, f32>,
}
impl Controller {
    pub fn horizontal(&self) -> &BTreeMap<String, f32> {
        &self.horizontal
    }
    pub fn pan(&mut self, key: &str, delta: f32, max: f32) -> bool {
        if !delta.is_finite() || !max.is_finite() {
            return false;
        }
        let old = self.horizontal.get(key).copied().unwrap_or(0.);
        let next = (old + delta).clamp(0., max.max(0.));
        if next == old {
            return false;
        }
        self.horizontal.insert(key.into(), next);
        true
    }
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
