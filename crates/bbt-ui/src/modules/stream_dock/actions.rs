// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Actions are derived from the retained tree, with its actual clip chain.
use super::{Entry, StreamDock};
use crate::{components as c, widgets::text_area::TextArea};
use bbt_ui_model::layout::Rect;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Task(String),
    OpenRecord(String),
    Delegate(String),
    Accept(String),
    Reject(String),
    CancelReject(String),
    SubmitReject(String),
    RejectInput(String),
    Channel(String),
}
#[derive(Default)]
pub struct Regions {
    rects: BTreeMap<String, Rect>,
    actions: Vec<(Action, Rect)>,
    inputs: BTreeMap<String, String>,
}
fn intersect(a: Rect, b: Rect) -> Rect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    Rect::new(
        x,
        y,
        (a.right().min(b.right()) - x).max(0.),
        (a.bottom().min(b.bottom()) - y).max(0.),
    )
}
fn contains(r: Rect, at: [f32; 2]) -> bool {
    at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom()
}
fn find<'a>(n: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&n.kind, c::LayoutKind::Named(name) if name == key) {
        Some(n)
    } else {
        n.children.iter().find_map(|child| find(child, key))
    }
}
impl Regions {
    pub fn retained(layout: &c::Layout) -> Self {
        fn visit(n: &c::Layout, clip: Option<Rect>, out: &mut BTreeMap<String, Rect>) {
            if let c::LayoutKind::Named(key) = &n.kind {
                let r = clip.map_or(n.rect, |clip| intersect(n.rect, clip));
                if r.w > 0. && r.h > 0. {
                    out.insert(key.clone(), r);
                }
            }
            let clips = matches!(
                n.kind,
                c::LayoutKind::ScrollView { .. } | c::LayoutKind::Box { clip: true, .. }
            );
            let clip = if clips {
                Some(clip.map_or(n.rect, |old| intersect(old, n.rect)))
            } else {
                clip
            };
            for child in &n.children {
                visit(child, clip, out);
            }
        }
        let mut out = Self::default();
        visit(layout, None, &mut out.rects);
        out
    }
    fn add(&mut self, key: &str, action: Action) {
        if let Some(rect) = self.rects.get(key) {
            self.actions.push((action, *rect));
        }
    }
    pub fn dock(layout: &c::Layout, view: &StreamDock) -> Self {
        Self::dock_from(layout, view, 0)
    }
    pub fn dock_from(layout: &c::Layout, view: &StreamDock, start: usize) -> Self {
        let mut out = Self::retained(layout);
        for (i, entry) in view.entries.iter().enumerate() {
            let key = format!("e{}", i + start);
            match entry {
                Entry::Message { data, .. } => {
                    for (j, attachment) in data.attach.iter().enumerate() {
                        let target = if attachment.path.is_empty() {
                            &attachment.id
                        } else {
                            &attachment.path
                        };
                        if !target.is_empty() {
                            out.add(
                                &format!("{key}/body/attach/a{j}"),
                                Action::OpenRecord(target.clone()),
                            );
                        }
                    }
                }
                Entry::Task { data, delegate } => {
                    out.add(&key, Action::Task(data.id.clone()));
                    if *delegate && data.state == "open" && data.assignee.is_empty() {
                        out.add(&format!("{key}/edel"), Action::Delegate(data.id.clone()));
                    }
                }
                Entry::Epic { data } => {
                    for (j, item) in data.items.iter().enumerate() {
                        let k = format!(
                            "{key}/eitem{}",
                            if j == 0 { String::new() } else { j.to_string() }
                        );
                        out.add(&k, Action::Task(item.id.clone()));
                        if data.delegate && item.state == "open" && item.assignee.is_empty() {
                            out.add(&format!("{k}/edel"), Action::Delegate(item.id.clone()));
                        }
                    }
                }
                Entry::Report { id, data, review }
                    if (*review && data.status == "reported") || data.reject.is_some() =>
                {
                    if let Some(reject) = &data.reject {
                        out.add(
                            &format!("{key}/reject/cancel"),
                            Action::CancelReject(id.clone()),
                        );
                        if *review && data.status == "reported" && !reject.reason.trim().is_empty()
                        {
                            out.add(
                                &format!("{key}/reject/submit"),
                                Action::SubmitReject(id.clone()),
                            );
                        }
                        let input = format!("{key}/reject/input");
                        out.add(&input, Action::RejectInput(id.clone()));
                        out.inputs.insert(id.clone(), input);
                    } else {
                        out.add(&format!("{key}/reject"), Action::Reject(id.clone()));
                        if data.can_accept() {
                            out.add(&format!("{key}/accept"), Action::Accept(id.clone()));
                        }
                    }
                }
                Entry::Tabs { channels, .. } => {
                    for (j, name) in std::iter::once("all")
                        .chain(channels.iter().map(|ch| ch.name.as_str()))
                        .enumerate()
                    {
                        out.add(
                            &format!(
                                "{key}/ch-tab{}",
                                if j == 0 { String::new() } else { j.to_string() }
                            ),
                            Action::Channel(name.into()),
                        );
                    }
                }
                _ => {}
            }
        }
        out
    }
    /// Merge already clipped actions from disjoint stream entry ranges.
    pub fn extend(&mut self, other: Self) {
        self.rects.extend(other.rects);
        self.actions.extend(other.actions);
        self.inputs.extend(other.inputs);
    }
    pub fn attachments(
        &mut self,
        prefix: &str,
        attachments: &[crate::modules::composer::Attachment],
    ) {
        for (i, attachment) in attachments.iter().enumerate() {
            let target = if attachment.path.is_empty() {
                &attachment.id
            } else {
                &attachment.path
            };
            if !target.is_empty() {
                self.add(
                    &format!("{prefix}/a{i}"),
                    Action::OpenRecord(target.clone()),
                );
            }
        }
    }
    /// Plain stream text remains byte-identical. Link regions follow each
    /// positioned run and its clipping; line wraps never create a giant hit box.
    pub fn record_text_links(&mut self, layout: &c::Layout, measure: &dyn c::Measure) {
        fn visit(
            n: &c::Layout,
            clip: Option<Rect>,
            measure: &dyn c::Measure,
            out: &mut Vec<(Action, Rect)>,
        ) {
            if let c::LayoutKind::Text { runs, .. } = &n.kind {
                let value: String = runs.iter().map(|r| r.text.as_str()).collect();
                let mut rest = value.as_str();
                let mut base = 0;
                while let Some(start) = rest.find("[[") {
                    let tail = &rest[start + 2..];
                    let Some(end) = tail.find("]]") else {
                        break;
                    };
                    let label = &tail[..end];
                    let target = label.split_once('|').map_or(label, |(target, _)| {
                        target.strip_suffix('\\').unwrap_or(target)
                    });
                    if target.starts_with('#')
                        || label
                            .split_once('|')
                            .is_some_and(|(_, alias)| alias.starts_with('#'))
                    {
                        let first = base + start;
                        let last = first + end + 4;
                        let mut offset = 0;
                        for run in runs {
                            let a = first.max(offset);
                            let b = last.min(offset + run.text.len());
                            if a < b {
                                let style = bbt_ui_model::layout::TextStyle {
                                    size: run.style.size,
                                    weight: run.style.weight,
                                    mono: run.style.mono,
                                    italic: run.style.italic,
                                    tracking_em: run.style.tracking_em,
                                };
                                let x = measure.width(&run.text[..a - offset], &style);
                                let w = measure.width(&run.text[a - offset..b - offset], &style);
                                let rect = Rect::new(run.rect.x + x, run.rect.y, w, run.rect.h);
                                let rect = clip.map_or(rect, |old| intersect(old, rect));
                                if rect.w > 0. && rect.h > 0. {
                                    out.push((Action::OpenRecord(target.into()), rect));
                                }
                            }
                            offset += run.text.len();
                        }
                    }
                    let used = start + end + 4;
                    base += used;
                    rest = &rest[used..];
                }
            }
            let clip = if matches!(
                n.kind,
                c::LayoutKind::ScrollView { .. } | c::LayoutKind::Box { clip: true, .. }
            ) {
                Some(clip.map_or(n.rect, |old| intersect(old, n.rect)))
            } else {
                clip
            };
            for child in &n.children {
                visit(child, clip, measure, out);
            }
        }
        visit(layout, None, measure, &mut self.actions);
    }
    /// Shared TaskRow interaction, namespaced by its caller's stable task id.
    pub fn task_row(&mut self, key: &str, id: &str, can_accept: bool) {
        for (suffix, action) in [
            (key.to_owned(), Action::Task(id.into())),
            (format!("{key}/right/accept"), Action::Accept(id.into())),
        ] {
            if matches!(action, Action::Accept(_)) && !can_accept {
                continue;
            }
            for (name, rect) in &self.rects {
                if name == &suffix || name.ends_with(&format!("/{suffix}")) {
                    self.actions.push((action.clone(), *rect));
                }
            }
        }
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<Action> {
        self.actions
            .iter()
            .rev()
            .find(|(_, r)| contains(*r, at))
            .map(|(action, _)| action.clone())
    }
    pub fn cursor(&self, at: [f32; 2]) -> crate::cursor::Cursor {
        use crate::cursor::Cursor;
        match self.hit(at) {
            Some(Action::RejectInput(_)) => Cursor::Text,
            Some(_) => Cursor::Pointer,
            None => Cursor::Default,
        }
    }
    pub fn contains(&self, key: &str, at: [f32; 2]) -> bool {
        self.rects.get(key).is_some_and(|rect| contains(*rect, at))
    }
    pub fn scroll_extent(layout: &c::Layout, key: &str) -> Option<[f32; 2]> {
        fn scroll(n: &c::Layout) -> Option<[f32; 2]> {
            if let c::LayoutKind::ScrollView { content, .. } = &n.kind {
                Some([
                    (content[0] - n.rect.w).max(0.),
                    (content[1] - n.rect.h).max(0.),
                ])
            } else {
                n.children.iter().find_map(scroll)
            }
        }
        scroll(find(layout, key)?)
    }
    pub fn candidate_area(&self, id: &str) -> Option<Rect> {
        let key = self.inputs.get(id)?;
        self.rects
            .get(&format!("{key}/edit/caret"))
            .or_else(|| self.rects.get(key))
            .copied()
    }
    pub fn index_at(
        &self,
        layout: &c::Layout,
        id: &str,
        value: &str,
        at: [f32; 2],
        m: &dyn c::Measure,
    ) -> Option<usize> {
        let key = format!("{}/edit", self.inputs.get(id)?);
        Some(TextArea::index_at(value, find(layout, &key)?, at, m))
    }
}
