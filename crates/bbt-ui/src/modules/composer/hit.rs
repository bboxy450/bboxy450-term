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
}
pub struct Regions {
    pub panel: Rect,
    pub body: Rect,
    pub caret: Option<Rect>,
    pub address: Rect,
    picker: Option<Rect>,
    kind_menu: Option<Rect>,
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
        Ok(Self {
            panel: rect("composer")?,
            body: rect("composer/text")?,
            caret: find(layout, "composer/text/caret").map(|n| n.rect),
            address: rect("composer/address")?,
            picker: find(layout, "composer/pick").map(|n| n.rect),
            kind_menu: find(layout, "composer/kind-menu").map(|n| n.rect),
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
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
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
    pub fn candidate_area(&self) -> Rect {
        self.caret.unwrap_or(Rect::new(
            self.body.x,
            self.body.y,
            2.,
            self.body.h.min(20.),
        ))
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
        find(layout, "composer/text/text").map_or(0, |n| TextArea::index_at(value, n, at, m))
    }
}
