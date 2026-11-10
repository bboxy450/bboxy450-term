// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Hit regions come from the solved sidebar, including clipped mini-grid cells.
use super::*;
impl Scene {
    pub fn sidebar_extent(&self, layouts: &[c::Layout]) -> Option<(Rect, f32)> {
        fn find(layout: &c::Layout) -> Option<(Rect, f32)> {
            if let c::LayoutKind::ScrollView { content, .. } = &layout.kind {
                return Some((layout.rect, (content[1] - layout.rect.h).max(0.)));
            }
            layout.children.iter().find_map(find)
        }
        layouts.get(1).and_then(find)
    }
    pub fn navigation_regions(&self, layouts: &[c::Layout]) -> Vec<(String, Rect)> {
        let Some(sidebar) = layouts.get(1) else {
            return vec![];
        };
        let mut result = vec![];
        let viewport = self
            .sidebar_extent(layouts)
            .map_or(sidebar.rect, |(r, _)| r);
        let mut add = |id: String, name: &str, clip: Rect| {
            if let Some(layout) = named(sidebar, name).or_else(|| named_suffix(sidebar, name)) {
                let r = layout.rect;
                let x = r.x.max(clip.x);
                let y = r.y.max(clip.y);
                let w = r.right().min(clip.right()) - x;
                let h = r.bottom().min(clip.bottom()) - y;
                if w > 0. && h > 0. {
                    result.push((id, Rect::new(x, y, w, h)));
                }
            }
        };
        for (i, swarm) in self.swarms.iter().enumerate() {
            let base = format!("swarm{i}");
            if let Some(rows) = self.canvas_rows.get(&swarm.id) {
                add(
                    format!("canvas-toggle:{}", swarm.id),
                    &format!("canvas/{}/toggle", swarm.id),
                    viewport,
                );
                if !self.canvas_closed.contains(&swarm.id) {
                    for row in rows {
                        add(
                            format!("canvas-open:{}/{}", swarm.id, row.id),
                            &format!("canvas/{}/open/{}", swarm.id, row.id),
                            viewport,
                        );
                    }
                }
            }
            for (j, tab) in swarm.tabs.iter().enumerate() {
                let path = format!("{base}/body/tab{j}");
                // Mini cells precede the encompassing header in pointer priority.
                fn cells<'a>(layout: &'a c::Layout, out: &mut Vec<&'a str>) {
                    if let c::LayoutKind::Named(name) = &layout.kind
                        && name
                            .rsplit('/')
                            .next()
                            .is_some_and(|s| s.starts_with("cell"))
                    {
                        out.push(name);
                    }
                    for child in &layout.children {
                        cells(child, out);
                    }
                }
                if let Some(grid) = named(sidebar, &format!("{path}/header/right/mg")) {
                    let mut names = vec![];
                    cells(grid, &mut names);
                    for (id, name) in tab.tree.leaves().into_iter().zip(names) {
                        add(format!("mini:{id}"), name, viewport);
                    }
                }
                for (k, id) in tab.seats.iter().enumerate() {
                    add(
                        format!("seat-mute:{id}"),
                        &format!("{path}/seat{k}/right/mute"),
                        viewport,
                    );
                    add(
                        format!("seat:{id}"),
                        &format!("{path}/seat{k}/row"),
                        viewport,
                    );
                }
                add(
                    format!("tab:{}", tab.id),
                    &format!("{path}/header"),
                    viewport,
                );
            }
            add(
                format!("toggle-swarm:{}", swarm.id),
                &format!("{base}/head"),
                viewport,
            );
            add(
                format!("new-seat:{}", swarm.id),
                &format!("{base}/body/newseat"),
                viewport,
            );
        }
        let offset = if !self.live || self.swarms.is_empty() {
            1
        } else {
            self.swarms.len()
        };
        for (i, card) in self.provider_cards().iter().enumerate() {
            add(
                format!("toggle-section:{}", card.key),
                &format!("swarm{}/head", offset + i),
                viewport,
            );
        }
        add("settings".into(), "top/span1", sidebar.rect);
        add("new-swarm".into(), "new-swarm", viewport);
        for (slot, layout) in self.pane_keys().zip(layouts.iter().skip(2)) {
            if let Slot::Seat(id) = slot {
                for action in ["view", "mute"] {
                    if let Some(control) = named(layout, &format!("pane/strip/{action}")) {
                        result.push((format!("pane-{action}:{id}"), control.rect));
                    }
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod view_switch_tests {
    use super::*;
    use ui::modules::strip::ViewTarget;

    fn labels(layout: &c::Layout) -> String {
        let mut text = String::new();
        if let c::LayoutKind::Text { runs, .. } = &layout.kind {
            for run in runs {
                text.push_str(&run.text);
            }
        }
        for child in &layout.children {
            text.push_str(&labels(child));
        }
        text
    }

    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "unattended geometry evidence IO, never a UI turn"
    )]
    fn view_switch_header_geometry_and_hits() {
        let ids = [0, 1, 2].map(SeatId::from_u128);
        let mut scene = Scene::new();
        scene.seats.truncate(3);
        scene.seat_index.retain(|_, index| *index < 3);
        for seat in &mut scene.seats {
            seat.may_mute = true;
            seat.shortcut = Some("8".into());
            seat.interrupt_sent = true;
            seat.name = "A very long operator supplied pane name".into();
            seat.title = "A long transcript title that must yield to header actions".into();
        }
        let mut evidence = vec![];
        for count in [1, 3] {
            scene.tree = Some(if count == 1 {
                SplitNode::Leaf(ids[0])
            } else {
                SplitNode::split(
                    Axis::Row,
                    1. / 3.,
                    SplitNode::Leaf(ids[0]),
                    SplitNode::split(
                        Axis::Row,
                        0.5,
                        SplitNode::Leaf(ids[1]),
                        SplitNode::Leaf(ids[2]),
                    ),
                )
            });
            for width in [200., 320., 800.] {
                for target in [ViewTarget::Terminal, ViewTarget::Harness] {
                    scene.set_pane_views(BTreeMap::from([(ids[0], target), (ids[1], target)]));
                    let size = [
                        scene.sidebar_width + (width + scene.rim) * count as f32,
                        700.,
                    ];
                    let layouts = scene.layout(size, false).unwrap();
                    let regions = scene.navigation_regions(&layouts);
                    for (index, pane) in layouts.iter().skip(2).enumerate() {
                        let id = ids[index];
                        let mute = named(pane, "pane/strip/mute").unwrap().rect;
                        let view = named(pane, "pane/strip/view");
                        if index == 2 {
                            assert!(view.is_none(), "ordinary terminal header changed");
                            assert!(
                                !regions
                                    .iter()
                                    .any(|(key, _)| key == &format!("pane-view:{id}"))
                            );
                            continue;
                        }
                        let view = view.expect("eligible header control");
                        assert_eq!(labels(view), target.label());
                        let identity = named(pane, "pane/strip/identity").unwrap().rect;
                        let close = named(pane, "pane/strip/close").unwrap().rect;
                        let body = pane_body(pane).unwrap();
                        assert!(close.right() <= identity.x && identity.right() <= view.rect.x);
                        assert!(view.rect.right() < mute.x, "view and mute overlap");
                        assert!(view.rect.x >= pane.rect.x && mute.right() <= pane.rect.right());
                        assert!(view.rect.bottom() <= body.y, "control overlays transcript");
                        assert!(
                            view.rect.h >= 20. && view.rect.w >= 48.,
                            "compact readable hit target"
                        );
                        assert!(
                            (view.rect.y + view.rect.h / 2. - mute.y - mute.h / 2.).abs() < 0.01
                        );
                        let hit = [
                            view.rect.x + view.rect.w / 2.,
                            view.rect.y + view.rect.h / 2.,
                        ];
                        assert_eq!(
                            regions
                                .iter()
                                .find(|(_, rect)| {
                                    hit[0] >= rect.x
                                        && hit[0] < rect.right()
                                        && hit[1] >= rect.y
                                        && hit[1] < rect.bottom()
                                })
                                .map(|(key, _)| key.as_str()),
                            Some(format!("pane-view:{id}").as_str()),
                        );
                        assert!(
                            regions
                                .iter()
                                .any(|(key, rect)| key == &format!("pane-mute:{id}")
                                    && *rect == mute)
                        );
                        evidence.push(serde_json::json!({"panes":count,"requested_width":width,"target":target.label(),"identity":format!("{identity:?}"),"control":format!("{:?}",view.rect),"mute":format!("{mute:?}")}));
                    }
                }
            }
        }
        if let Ok(path) = std::env::var("BBOXY_VIEW_SWITCH_GEOMETRY") {
            std::fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
        }
        assert_eq!(evidence.len(), 18);
    }
}

impl Scene {
    pub fn launcher_preview(
        &self,
        launcher: &Launcher,
        preset: Option<bbt_proto::seat_state::Side>,
    ) -> c::Element {
        use ui::modules::launcher_banner::{Banner, TabBanner};
        let id = launcher
            .params
            .as_ref()
            .map(|p| p.launch.id.as_str())
            .or_else(|| launcher.path.last().map(String::as_str))
            .unwrap_or("");
        let side = if id.ends_with(":right") {
            Some(Axis::Row)
        } else if id.ends_with(":bottom") {
            Some(Axis::Column)
        } else {
            preset.map(|s| {
                if s == bbt_proto::seat_state::Side::Right {
                    Axis::Row
                } else {
                    Axis::Column
                }
            })
        };
        let ctx = self.ui_context();
        if let (Some(side), Some(tree), Some(focused)) = (side, self.tree.as_ref(), self.focused) {
            fn insert(tree: Node, focused: &str, side: Axis) -> Node {
                match tree {
                    Node::Cell { ref key, .. } if key == focused => Node::Split {
                        axis: side,
                        ratio: 0.5,
                        a: Box::new(tree),
                        b: Box::new(Node::Cell {
                            key: "new".into(),
                            name: "new".into(),
                            activity: Activity::Idle,
                        }),
                    },
                    Node::Split { axis, ratio, a, b } => Node::Split {
                        axis,
                        ratio,
                        a: Box::new(insert(*a, focused, side)),
                        b: Box::new(insert(*b, focused, side)),
                    },
                    node => node,
                }
            }
            let anchor = &self.seats[self.seat_index[&focused]];
            return Banner {
                tree: insert(self.mini(tree), &focused.to_string(), side),
                focused_key: "new".into(),
                title: if side == Axis::Row {
                    "Split right"
                } else {
                    "Split down"
                }
                .into(),
                anchor: anchor.name.clone(),
                anchor_color: ui::Paint::solid(anchor.color(ctx))
                    .resolve(ctx)
                    .expect("seat color"),
            }
            .element(ctx);
        }
        let swarm = self
            .swarms
            .iter()
            .find(|s| {
                Some(s.id) == self.selected_swarm || {
                    s.tabs
                        .iter()
                        .any(|t| self.focused.is_some_and(|id| t.tree.contains(id)))
                }
            })
            .or_else(|| self.swarms.first());
        TabBanner {
            swarm: swarm.map_or("New swarm", |s| s.name.as_str()).into(),
            seats: swarm
                .into_iter()
                .flat_map(|s| &s.tabs)
                .flat_map(|t| &t.seats)
                .filter_map(|id| self.seat_index.get(id).map(|i| &self.seats[*i]))
                .take(5)
                .map(|s| {
                    (
                        s.name.clone(),
                        ui::Paint::solid(s.color(ctx))
                            .resolve(ctx)
                            .expect("seat color"),
                    )
                })
                .collect(),
        }
        .element(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sidebar_hits_follow_scroll_clipping_and_collapsed_tabs_keep_their_mini_grid() {
        let records: Vec<_> = (1..=20)
            .map(|n| {
                bbt_proto::SeatRecord::new(
                    bbt_mux::Ulid(n),
                    bbt_mux::Ulid(90),
                    format!("seat {n}"),
                    "zsh",
                )
            })
            .collect();
        let mut model = bboxy450_term::model::build_at(
            &records,
            &bboxy450_term::grids::Grids::new(),
            &Default::default(),
            Instant::now(),
            1000,
        );
        let mut scene = Scene::live();
        scene.set_model(&model, &model.ui.window.stage);
        let layouts = scene.layout([1000., 320.], false).unwrap();
        let (viewport, max) = scene.sidebar_extent(&layouts).unwrap();
        assert!(max > 0.);
        let first = format!("seat:{}", records[0].id);
        let last = format!("seat:{}", records[19].id);
        let hits = scene.navigation_regions(&layouts);
        assert!(hits.iter().any(|(id, _)| id == &first));
        assert!(!hits.iter().any(|(id, _)| id == &last));
        scene.sidebar_scroll = max;
        let layouts = scene.layout([1000., 320.], false).unwrap();
        let hits = scene.navigation_regions(&layouts);
        assert!(!hits.iter().any(|(id, _)| id == &first));
        assert!(hits.iter().any(|(id, _)| id == &last));
        assert!(
            hits.iter()
                .filter(|(id, _)| id.starts_with("seat:"))
                .all(|(_, r)| r.y >= viewport.y && r.bottom() <= viewport.bottom())
        );
        model.ui.window.sidebar.swarms[0].tabs[0].expanded = false;
        scene.set_model(&model, &model.ui.window.stage);
        scene.sidebar_scroll = 0.;
        let layouts = scene.layout([1000., 320.], false).unwrap();
        let hits = scene.navigation_regions(&layouts);
        assert!(!hits.iter().any(|(id, _)| id.starts_with("seat:")));
        assert_eq!(
            hits.iter()
                .filter(|(id, _)| id.starts_with("mini:"))
                .count(),
            20
        );
    }
}
