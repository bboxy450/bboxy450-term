// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Semantic hit regions from the retained layout which is actually painted.
use crate::{components as c, widgets::text_area::TextArea};
use bbt_ui_model::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Body,
    Address,
    Choose(usize),
    Remove(usize),
    Send,
    Priority,
    Kind,
    KindChoice(usize),
    Attach,
    AttachQuery,
    AttachChoice(usize),
}
#[derive(Default)]
pub struct Regions {
    pub panel: Rect,
    pub body: Rect,
    pub body_offset: f32,
    pub body_max_scroll: f32,
    pub caret: Option<Rect>,
    pub address: Rect,
    picker: Option<Rect>,
    kind_menu: Option<Rect>,
    attachment_picker: Option<Rect>,
    pub attachment_query: Option<Rect>,
    attachment_caret: Option<Rect>,
    attachment_clip: Option<Rect>,
    attachment_choices: Vec<(usize, Rect)>,
    kinds: Vec<Rect>,
    choices: Vec<Rect>,
    removes: Vec<Rect>,
    send: Rect,
    priority: Rect,
    kind: Option<Rect>,
    attach: Rect,
}
fn contains(r: Rect, at: [f32; 2]) -> bool {
    at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
}
fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind,c::LayoutKind::Named(name) if name==key) {
        Some(n)
    } else {
        n.children.iter().find_map(|n| find(n, key))
    }
}
impl Regions {
    pub fn new(layout: &c::Layout) -> Result<Self, &'static str> {
        let rect = |key: &str| {
            find(layout, key)
                .map(|n| n.rect)
                .ok_or("composer region missing")
        };
        let choices = (0..)
            .map_while(|i| find(layout, &format!("composer/pick/o{i}")).map(|n| n.rect))
            .collect();
        let removes = (0..)
            .map_while(|i| find(layout, &format!("composer/address/a{i}/x")).map(|n| n.rect))
            .collect();
        fn clip(n: &c::Layout) -> Option<Rect> {
            if matches!(n.kind, c::LayoutKind::ScrollView { .. }) {
                Some(n.rect)
            } else {
                n.children.iter().find_map(clip)
            }
        }
        fn attachment_regions(n: &c::Layout, out: &mut Vec<(usize, Rect)>) {
            if let c::LayoutKind::Named(name) = &n.kind
                && let Some(i) = name
                    .strip_prefix("composer/attach-pick/o")
                    .and_then(|s| s.parse().ok())
            {
                out.push((i, n.rect));
            }
            for child in &n.children {
                attachment_regions(child, out);
            }
        }
        let mut attachment_choices = vec![];
        attachment_regions(layout, &mut attachment_choices);
        let body_viewport = find(layout, "composer/body-viewport");
        let (body_offset, body_max_scroll) = body_viewport
            .and_then(|n| n.children.first())
            .and_then(|n| if let c::LayoutKind::ScrollView { content, offset } = &n.kind {
                Some((offset[1], (content[1] - n.rect.h).max(0.)))
            } else { None }).unwrap_or((0., 0.));
        Ok(Self {
            body_offset,
            body_max_scroll,
            panel: rect("composer")?,
            body: find(layout, "composer/body-viewport").map_or(rect("composer/text")?, |n| n.rect),
            caret: find(layout, "composer/text/caret").map(|n| n.rect),
            address: rect("composer/address")?,
            picker: find(layout, "composer/pick").map(|n| n.rect),
            kind_menu: find(layout, "composer/kind-menu").map(|n| n.rect),
            attachment_picker: find(layout, "composer/attach-pick").map(|n| n.rect),
            attachment_query: find(layout, "composer/attach-pick/query").map(|n| n.rect),
            attachment_caret: find(layout, "composer/attach-pick/query/caret").map(|n| n.rect),
            attachment_clip: find(layout, "composer/attach-pick/list").and_then(clip),
            attachment_choices,
            kinds: (0..3)
                .filter_map(|i| find(layout, &format!("composer/kind-menu/k{i}")).map(|n| n.rect))
                .collect(),
            choices,
            removes,
            send: rect("composer/send")?,
            priority: rect("composer/priority")?,
            kind: find(layout, "composer/kind").map(|n| n.rect),
            attach: rect("composer/attach")?,
        })
    }
    pub fn contains(&self, at: [f32; 2]) -> bool {
        contains(self.panel, at)
            || self.picker.is_some_and(|r| contains(r, at))
            || self.kind_menu.is_some_and(|r| contains(r, at))
            || self.attachment_picker.is_some_and(|r| contains(r, at))
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        if self.attachment_query.is_some_and(|r| contains(r, at)) {
            return Some(Hit::AttachQuery);
        }
        if self.attachment_clip.is_some_and(|r| contains(r, at))
            && let Some((i, _)) = self
                .attachment_choices
                .iter()
                .find(|(_, r)| contains(*r, at))
        {
            return Some(Hit::AttachChoice(*i));
        }
        if let Some(i) = self.kinds.iter().position(|r| contains(*r, at)) {
            return Some(Hit::KindChoice(i));
        }
        if let Some(i) = self.choices.iter().position(|r| contains(*r, at)) {
            return Some(Hit::Choose(i));
        }
        if contains(self.priority, at) {
            return Some(Hit::Priority);
        }
        if contains(self.send, at) {
            return Some(Hit::Send);
        }
        if contains(self.attach, at) {
            return Some(Hit::Attach);
        }
        if self.kind.is_some_and(|r| contains(r, at)) {
            return Some(Hit::Kind);
        }
        if let Some(i) = self.removes.iter().position(|r| contains(*r, at)) {
            return Some(Hit::Remove(i));
        }
        if contains(self.address, at) {
            return Some(Hit::Address);
        }
        contains(self.body, at).then_some(Hit::Body)
    }
    pub fn cursor(&self, at: [f32; 2], controller: &super::Controller) -> crate::cursor::Cursor {
        use crate::cursor::Cursor;
        match self.hit(at) {
            Some(Hit::Body | Hit::Address | Hit::AttachQuery) => Cursor::Text,
            Some(Hit::Send) if !controller.send_enabled() => Cursor::Default,
            Some(Hit::Choose(index)) => {
                let available = !controller.composing()
                    && controller
                        .view
                        .pick
                        .iter()
                        .flat_map(|g| &g.items)
                        .nth(index)
                        .is_some_and(|a| {
                            !a.destination.is_empty() || a.kind == "role" && !a.role.is_empty()
                        });
                if available {
                    Cursor::Pointer
                } else {
                    Cursor::Default
                }
            }
            Some(Hit::AttachChoice(index)) => {
                let available =
                    !controller.composing()
                        && controller.view.attach_pick.as_ref().is_some_and(|p| {
                            p.rows().get(index).is_some_and(|a| !a.path.is_empty())
                        });
                if available {
                    Cursor::Pointer
                } else {
                    Cursor::Default
                }
            }
            Some(_) => Cursor::Pointer,
            None => Cursor::Default,
        }
    }
    pub fn candidate_area(&self) -> Rect {
        if let Some(query) = self.attachment_query {
            return self
                .attachment_caret
                .unwrap_or(Rect::new(query.x, query.y, 2., query.h));
        }
        let caret = self.caret.unwrap_or(Rect::new(self.body.x, self.body.y, 2., self.body.h.min(20.)));
        Rect::new(caret.x.clamp(self.body.x, (self.body.right() - caret.w).max(self.body.x)),
            caret.y.clamp(self.body.y, (self.body.bottom() - caret.h).max(self.body.y)),
            caret.w.min(self.body.w), caret.h.min(self.body.h))
    }
    pub fn attachment_index_at(
        &self,
        layout: &c::Layout,
        value: &str,
        at: [f32; 2],
        m: &dyn c::Measure,
    ) -> usize {
        find(layout, "composer/attach-pick/query/text")
            .map_or(0, |n| TextArea::index_at(value, n, at, m))
    }
    pub fn position_at(
        &self,
        layout: &c::Layout,
        value: &str,
        index: usize,
        m: &dyn c::Measure,
    ) -> [f32; 2] {
        find(layout, "composer/text/text").map_or([self.body.x, self.body.y], |n| {
            TextArea::position_at(value, n, index, m)
        })
    }
    pub fn index_at(
        &self,
        layout: &c::Layout,
        value: &str,
        at: [f32; 2],
        m: &dyn c::Measure,
    ) -> usize {
        let at = [at[0].clamp(self.body.x, self.body.right()),
            at[1].clamp(self.body.y, (self.body.bottom() - 0.01).max(self.body.y))];
        find(layout, "composer/text/text").map_or(0, |n| TextArea::index_at(value, n, at, m))
    }
}
