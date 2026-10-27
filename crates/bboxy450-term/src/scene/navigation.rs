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
            if let Some(layout) = named(sidebar, name) {
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
                format!("swarm:{}", swarm.id),
                &format!("{base}/head"),
                viewport,
            );
            add(
                format!("new-seat:{}", swarm.id),
                &format!("{base}/body/newseat"),
                viewport,
            );
        }
        add("settings".into(), "top/span1", sidebar.rect);
        add("new-swarm".into(), "new-swarm", viewport);
        result
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
