//! Actions are derived from the retained tree, with its actual clip chain.
use super::{Entry, StreamDock};
use crate::{components as c, widgets::text_area::TextArea};
use bbt_ui_model::layout::Rect;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Task(String),
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
        let mut out = Self::retained(layout);
        for (i, entry) in view.entries.iter().enumerate() {
            let key = format!("e{i}");
            match entry {
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
                Entry::Report { id, data, review } if *review && data.status == "reported" => {
                    if let Some(reject) = &data.reject {
                        out.add(
                            &format!("{key}/reject/cancel"),
                            Action::CancelReject(id.clone()),
                        );
                        if !reject.reason.trim().is_empty() {
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
