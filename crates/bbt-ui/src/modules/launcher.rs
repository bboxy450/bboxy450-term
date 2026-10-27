//! Launcher presentation and pure tree/filter projection. The app owns input,
//! launching and name generation. Its pure controller owns navigation and editing.
pub mod controller;
pub mod params;
use super::{seat_row::named, strip::plain};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, draw::Color as Rgba, layout::Axis};
pub use params::{FieldKey, Source, SpawnField, SpawnParams, ValueChoice};
use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Item {
    pub id: String,
    pub label: String,
    pub sub: String,
    pub harness: String,
    /// Declarative spawn preset; the backend resolves its authority class.
    pub role: Option<String>,
    /// A local launch leaf may open the caller-resolved parameter level.
    pub parameters: bool,
    pub kind: String,
    pub tile: Option<String>,
    pub section: Option<String>,
    pub aliases: Vec<String>,
    pub tags: Vec<String>,
    pub navigation: bool,
    pub link: Option<super::link::LinkState>,
    pub down: bool,
    /// Enter picks this parent's default action; Right still opens its children.
    #[serde(rename = "enterPicks")]
    pub enter_picks: bool,
    /// Some(empty) remains a parent, as in the lab's empty Go to fixture.
    pub children: Option<Vec<Item>>,
}
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct Cap {
    pub scope: String,
    pub name: String,
    pub used: usize,
    pub limit: usize,
    pub counts: String,
}
#[derive(Debug, Clone)]
pub struct Launcher {
    pub items: Vec<Item>,
    pub query: String,
    pub path: Vec<String>,
    pub selected: Option<usize>,
    pub placeholder: String,
    pub name: String,
    pub swatch: [f32; 4],
    pub actions: Vec<(String, String)>,
    pub cap: Option<Cap>,
    pub spotlight: bool,
    pub edit: Option<c::Edit>,
    pub params: Option<SpawnParams>,
}
pub struct View<'a> {
    pub crumbs: Vec<&'a Item>,
    pub rows: Vec<&'a Item>,
    pub selected: usize,
}
fn norm(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
fn aliases(item: &Item) -> Vec<String> {
    std::iter::once(&item.label)
        .chain(&item.aliases)
        .map(|s| norm(s))
        .collect()
}
fn subsequence(hay: &str, needle: &str) -> bool {
    let mut n = needle.chars().peekable();
    for h in hay.chars() {
        if n.peek() == Some(&h) {
            n.next();
        }
    }
    n.peek().is_none()
}
fn prefix(it: &Item, w: &str) -> bool {
    aliases(it).iter().any(|a| a.starts_with(w))
        || norm(&it.label).split(' ').any(|a| a.starts_with(w))
}
fn tier(it: &Item, w: &str) -> u8 {
    if prefix(it, w) {
        1
    } else if it.tags.iter().any(|t| norm(t).starts_with(w)) {
        2
    } else if norm(&it.sub).contains(w) {
        3
    } else {
        4
    }
}
fn hits(it: &Item, w: &str) -> bool {
    tier(it, w) < 4
        || it.id.starts_with(w)
        || (w.chars().count() >= 3 && subsequence(&norm(&it.label).replace(' ', ""), w))
}
impl Launcher {
    /// The app's Enter handler consumes this; unavailable hosts stay visible.
    pub fn activation(&self) -> Option<&Item> {
        let view = self.view();
        view.rows
            .get(view.selected)
            .copied()
            .filter(|item| !item.down && !item.link.as_ref().is_some_and(|link| link.down()))
    }
    pub fn view(&self) -> View<'_> {
        if let Some(params) = &self.params {
            return params.view(&self.query, self.selected);
        }
        let mut level = self.items.as_slice();
        let mut crumbs = vec![];
        for id in &self.path {
            let Some(it) = level.iter().find(|it| &it.id == id) else {
                break;
            };
            crumbs.push(it);
            level = it.children.as_deref().unwrap_or(&[]);
        }
        let mut q = norm(&self.query);
        for _ in 0..8 {
            let found = level
                .iter()
                .filter(|it| it.children.is_some())
                .find_map(|it| {
                    aliases(it)
                        .into_iter()
                        .find(|a| q.starts_with(&format!("{a} ")))
                        .map(|a| (it, a.len() + 1))
                });
            let Some((it, n)) = found else { break };
            crumbs.push(it);
            q = q[n..].to_owned();
            level = it.children.as_deref().unwrap();
        }
        let words = q.split(' ').filter(|w| !w.is_empty()).collect::<Vec<_>>();
        let mut rows = level
            .iter()
            .filter(|it| words.iter().all(|w| hits(it, w)))
            .collect::<Vec<_>>();
        if let Some(first) = words.first() {
            rows.sort_by_key(|it| tier(it, first));
        }
        let selected = self.selected.unwrap_or_else(|| {
            if self.cap.is_some() {
                rows.iter().position(|it| it.navigation).unwrap_or(0)
            } else {
                0
            }
        });
        View {
            crumbs,
            rows,
            selected,
        }
    }
}
fn paint(color: ui::Color, alpha: f32) -> ui::Paint {
    ui::Paint {
        color,
        alpha: ui::Alpha::Value(alpha),
    }
}
fn text(
    key: &str,
    value: impl Into<String>,
    size: f32,
    weight: u16,
    color: ui::Color,
    mono: bool,
    wrap: bool,
) -> c::Element {
    let mut t = c::Text::new(
        value,
        ui::Text {
            size: Some(size),
            weight,
            paint: ui::Paint::solid(color),
            mono,
            line_height: 1.4,
            ..Default::default()
        },
    );
    t.wrap = wrap;
    if key.is_empty() {
        c::Element::Text(t)
    } else {
        named(key, c::Element::Text(t))
    }
}
fn stack(children: Vec<c::Item>, axis: Axis, gap: f32) -> c::Element {
    c::Element::Stack(c::Stack {
        children,
        direction: axis,
        gap: ui::Length::Px(gap),
        align: if axis == Axis::Column {
            c::Alignment::Stretch
        } else {
            c::Alignment::Center
        },
    })
}
fn shrink(e: c::Element) -> c::Item {
    c::Item {
        element: e,
        extent: c::Extent::MinContent,
    }
}
fn boxed(key: &str, style: ui::Style, children: Vec<c::Item>, axis: Axis, gap: f32) -> c::Element {
    let mut b = c::Box::new(style, children);
    b.direction = axis;
    b.gap = ui::Length::Px(gap);
    if axis == Axis::Column {
        b.align = c::Alignment::Stretch;
    }
    if key.is_empty() {
        c::Element::Box(b)
    } else {
        named(key, c::Element::Box(b))
    }
}
fn leaf(key: &str, size: [f32; 2], color: ui::Paint, radius: f32) -> c::Element {
    let mut s = plain(Some(size[1]), [0.; 4], radius);
    s.overrides.width = Some(ui::Length::Px(size[0]));
    s.overrides.fill = Some(color);
    boxed(key, s, vec![], Axis::Row, 0.)
}
impl Launcher {
    fn header(&self, view: &View<'_>) -> c::Element {
        let spot = self.spotlight;
        let mut children = vec![];
        if !view.crumbs.is_empty() {
            let crumbs = view
                .crumbs
                .iter()
                .enumerate()
                .map(|(i, it)| {
                    let key = format!(
                        "picker/inrow/crumbs/crumb{}",
                        if i == 0 { String::new() } else { i.to_string() }
                    );
                    let mut s = plain(None, [4., 10., 4., 10.], STYLE.radius.badge as f32);
                    s.overrides.fill = Some(paint(
                        ui::Color::Foreground,
                        STYLE.tint.selection_alpha as f32,
                    ));
                    shrink(boxed(
                        &key,
                        s,
                        vec![
                            text(
                                &format!("{key}/text"),
                                format!("{} ›", it.label),
                                STYLE.r#type.kind as f32,
                                500,
                                ui::Color::Foreground,
                                false,
                                false,
                            )
                            .into(),
                        ],
                        Axis::Row,
                        0.,
                    ))
                })
                .collect();
            children.push(shrink(boxed(
                "picker/inrow/crumbs",
                plain(None, [0., 0., 0., 20.], 0.),
                crumbs,
                Axis::Row,
                6.,
            )));
        }
        let mut input = plain(
            None,
            if spot {
                [20., 24., 18., 24.]
            } else {
                [10., 12., 10., 12.]
            },
            0.,
        );
        if !spot {
            input.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint::solid(ui::Color::Line),
                outside: false,
            });
            input.overrides.border_sides = Some([false, false, true, false]);
        }
        let mut t = text(
            "picker/inrow/input/text",
            if self.query.is_empty() {
                &self.placeholder
            } else {
                &self.query
            },
            if spot {
                STYLE.r#type.input
            } else {
                STYLE.r#type.body
            } as f32,
            400,
            if self.query.is_empty() {
                ui::Color::Muted
            } else {
                ui::Color::Foreground
            },
            false,
            false,
        );
        if spot
            && self.query.is_empty()
            && let c::Element::Named { content, .. } = &mut t
            && let c::Element::Text(t) = content.as_mut()
        {
            t.runs[0].style.paint.alpha = ui::Alpha::Value(0.8);
        }
        if let Some(edit) = &self.edit {
            let c::Element::Named { content, .. } = t else {
                unreachable!()
            };
            let c::Element::Text(text) = *content else {
                unreachable!()
            };
            let mut style = text.runs[0].style;
            style.paint = ui::Paint::solid(ui::Color::Foreground);
            t = c::Element::EditingText(c::EditingText {
                value: self.query.clone(),
                placeholder: self.placeholder.clone(),
                style,
                placeholder_paint: paint(ui::Color::Muted, if spot { 0.8 } else { 1. }),
                edit: edit.clone(),
                name: "picker/inrow/input/edit".into(),
            });
        }
        let mut b = c::Box::new(input, vec![t.into()]);
        b.gap = ui::Length::Px(0.);
        b.clip = true;
        children.push(c::Item {
            element: named("picker/inrow/input", c::Element::Box(b)),
            extent: c::Extent::Fill,
        });
        let mut head = plain(None, if spot { [0., 20., 0., 0.] } else { [0.; 4] }, 0.);
        if spot {
            head.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: false,
            });
            head.overrides.border_sides = Some([false, false, true, false]);
        }
        boxed("picker/inrow", head, children, Axis::Row, 0.)
    }
    fn row(&self, it: &Item, index: usize, selected: bool, ctx: ui::Context) -> c::Element {
        let spot = self.spotlight;
        let parent =
            it.children.is_some() && !it.down && !it.link.as_ref().is_some_and(|link| link.down());
        let key = format!(
            "picker/div/opt{}",
            if index == 0 {
                String::new()
            } else {
                index.to_string()
            }
        );
        let gap = if spot { 12. } else { 10. };
        let mut tile = plain(
            Some(if spot { 30. } else { 22. }),
            [0.; 4],
            if spot {
                STYLE.radius.tile
            } else {
                STYLE.radius.badge
            } as f32,
        );
        tile.overrides.width = Some(ui::Length::Px(if spot { 30. } else { 22. }));
        tile.overrides.fill = Some(ui::Paint::solid(
            it.tile
                .as_ref()
                .map_or(ui::Color::Surface2, |s| ui::Color::Rgba(Rgba::hex(s).0)),
        ));
        if spot {
            tile.overrides.rings = Some(vec![ui::Border {
                width: ui::Length::Hairline,
                paint: paint(ui::Color::OnAccent, 0.12),
                outside: false,
            }]);
        }
        let icon = named(
            format!("{key}/tile/hicon"),
            c::Element::Icon {
                name: it.harness.clone(),
                size: ui::Length::Px(if spot { 17. } else { 13. }),
                paint: ui::Paint::solid(ui::Color::OnAccent),
            },
        );
        let tile = boxed(
            &format!("{key}/tile"),
            tile,
            vec![
                c::Element::Spacer.into(),
                icon.into(),
                c::Element::Spacer.into(),
            ],
            Axis::Row,
            0.,
        );
        let label = text(
            &format!("{key}/lbl"),
            &it.label,
            if spot {
                STYLE.r#type.title
            } else {
                STYLE.r#type.body
            } as f32,
            if spot { 500 } else { 400 },
            ui::Color::Foreground,
            false,
            true,
        );
        let mut sub = text(
            &format!("{key}/sub"),
            &it.sub,
            if spot {
                STYLE.r#type.subtitle
            } else {
                STYLE.r#type.caption
            } as f32,
            400,
            ui::Color::Muted,
            false,
            true,
        );
        let parameters = self.params.is_some();
        if parameters
            && let c::Element::Named { content, .. } = &mut sub
            && let c::Element::Text(text) = content.as_mut()
        {
            text.wrap = false;
            text.ellipsis = true;
        }
        let sub = if spot {
            boxed(
                "",
                plain(None, [0., 0., 0., 2.], 0.),
                vec![c::Item {
                    element: sub,
                    extent: if parameters {
                        c::Extent::Fill
                    } else {
                        c::Extent::MinContent
                    },
                }],
                Axis::Row,
                0.,
            )
        } else {
            sub
        };
        let left = boxed(
            "",
            plain(None, [0., gap, 0., 0.], 0.),
            vec![
                tile.into(),
                shrink(label),
                c::Item {
                    element: sub,
                    extent: if parameters {
                        c::Extent::Fill
                    } else {
                        c::Extent::MinContent
                    },
                },
            ],
            Axis::Row,
            gap,
        );
        let kind = text(
            &format!("{key}/kind"),
            &it.kind,
            if spot {
                STYLE.r#type.kind
            } else {
                STYLE.r#type.caption
            } as f32,
            400,
            ui::Color::Muted,
            false,
            !parameters,
        );
        let kind = it.link.as_ref().map_or(kind, |link| {
            let size = if spot {
                STYLE.r#type.kind
            } else {
                STYLE.r#type.caption
            } as f32;
            let mut style = plain(None, [0.; 4], 0.);
            style.overrides.text = Some(ui::Text {
                size: Some(size),
                line_height: 1.4,
                ..Default::default()
            });
            let mut kind = c::Box::new(
                style,
                vec![
                    link.named_element(size, true, &format!("{key}/kind/link"))
                        .into(),
                ],
            );
            kind.inline = true;
            kind.align = c::Alignment::Baseline;
            kind.gap = ui::Length::Px(0.);
            named(format!("{key}/kind"), c::Element::Box(kind))
        });
        let end = if parent {
            let mut s = plain(None, [0.; 4], 0.);
            if spot {
                s.overrides.width = Some(ui::Length::Px(24.));
            }
            boxed(
                &format!("{key}/chev"),
                s,
                vec![
                    c::Element::Spacer.into(),
                    text(
                        "",
                        "›",
                        if spot {
                            ctx.density.tokens().icon as f32
                        } else {
                            STYLE.r#type.body as f32
                        },
                        400,
                        ui::Color::Muted,
                        false,
                        false,
                    )
                    .into(),
                    c::Element::Spacer.into(),
                ],
                Axis::Row,
                0.,
            )
        } else if spot {
            let mut s = plain(Some(22.), [0.; 4], STYLE.radius.badge as f32);
            s.overrides.width = Some(ui::Length::Px(24.));
            s.overrides.fill = Some(paint(
                ui::Color::Muted,
                if selected {
                    STYLE.tint.badge_selected_alpha
                } else {
                    STYLE.tint.badge_alpha
                } as f32,
            ));
            boxed(
                &format!("{key}/num"),
                s,
                vec![
                    c::Element::Spacer.into(),
                    text(
                        "",
                        (index + 1).to_string(),
                        STYLE.r#type.badge as f32,
                        400,
                        ui::Color::Foreground,
                        true,
                        false,
                    )
                    .into(),
                    c::Element::Spacer.into(),
                ],
                Axis::Row,
                0.,
            )
        } else {
            text(
                &format!("{key}/num"),
                (index + 1).to_string(),
                STYLE.r#type.small as f32,
                400,
                ui::Color::Muted,
                true,
                false,
            )
        };
        let end = if parent {
            boxed(
                "",
                plain(None, [0., 0., 0., STYLE.spacing.s2 as f32], 0.),
                vec![end.into()],
                Axis::Row,
                0.,
            )
        } else {
            end
        };
        let mut end = end;
        if it.down || it.link.as_ref().is_some_and(|link| link.down()) {
            hide_number(&mut end);
        }
        if self.cap.is_some() && !it.navigation && it.children.is_none() {
            hide_number(&mut end);
        }
        let right = stack(vec![shrink(kind), end.into()], Axis::Row, gap);
        let mut s = plain(
            None,
            if spot {
                [8., 14., 8., 14.]
            } else {
                [7., 12., 7., 12.]
            },
            if spot { STYLE.radius.row as f32 } else { 8. },
        );
        if selected {
            s.overrides.fill = Some(if spot {
                paint(ui::Color::Foreground, STYLE.tint.selection_alpha as f32)
            } else {
                ui::Paint::solid(ui::Color::Rgba(
                    Rgba::hex(ctx.theme.palette().surface)
                        .mix(Rgba::hex(ctx.theme.palette().accent), 0.28)
                        .0,
                ))
            });
        }
        let children = if parameters {
            vec![
                c::Item {
                    element: left,
                    extent: c::Extent::Fill,
                },
                shrink(right),
            ]
        } else {
            vec![
                shrink(left),
                c::Item {
                    element: c::Element::Spacer,
                    extent: c::Extent::Fill,
                },
                shrink(right),
            ]
        };
        let row = boxed(&key, s, children, Axis::Row, 0.);
        let row = if it.down || it.link.as_ref().is_some_and(|link| link.down()) {
            c::Element::Filter {
                content: Box::new(row),
                saturation: 1.,
                opacity: 0.45,
            }
        } else if self.cap.is_some() && !it.navigation {
            c::Element::Filter {
                content: Box::new(row),
                saturation: 1.,
                opacity: 0.38,
            }
        } else {
            row
        };
        boxed(
            "",
            plain(
                None,
                [
                    0.,
                    if spot { 10. } else { 6. },
                    0.,
                    if spot { 10. } else { 6. },
                ],
                0.,
            ),
            vec![row.into()],
            Axis::Column,
            0.,
        )
    }
    fn foot(&self, ctx: ui::Context) -> c::Element {
        let spot = self.spotlight;
        let gap = if spot { 10. } else { 8. };
        let size = if spot { 12. } else { 10. };
        let mut sw = plain(Some(size), [0.; 4], size / 2.);
        sw.overrides.width = Some(ui::Length::Px(size));
        sw.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(self.swatch)));
        sw.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint::solid(ui::Color::Line),
            outside: false,
        });
        let sw = c::Item {
            element: boxed("picker/foot/sw", sw, vec![], Axis::Row, 0.),
            extent: c::Extent::Content {
                grow: 0.,
                shrink: 1.,
                min: 2.,
                max: None,
                max_fraction: None,
            },
        };
        let name = text(
            "picker/foot/nm",
            &self.name,
            if spot {
                STYLE.r#type.kind
            } else {
                STYLE.r#type.section
            } as f32,
            600,
            ui::Color::Foreground,
            false,
            true,
        );
        let mut button = plain(
            if spot { Some(28.) } else { None },
            if spot { [0.; 4] } else { [2., 10., 2., 10.] },
            if spot { 14. } else { TOKENS.radius.grid as f32 },
        );
        if spot {
            button.overrides.width = Some(ui::Length::Px(28.));
            button.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: false,
            });
        }
        button.overrides.shadow = Some(ui::Shadow::Positioned {
            offset: [0., 1.],
            blur: 0.,
            color: [0., 0., 0., 0.4],
        });
        if !spot {
            button.overrides.material = Some(ui::Material::Gloss);
            button.overrides.fill = Some(ui::Paint::solid(ui::Color::Rgba(Rgba::hex("#0f1226").0)));
        }
        let mut reroll = c::Text::new(
            "↻",
            ui::Text {
                size: Some(if spot {
                    STYLE.r#type.kind
                } else {
                    STYLE.r#type.micro
                } as f32),
                paint: ui::Paint::solid(if spot {
                    ui::Color::Foreground
                } else {
                    ui::Color::Rgba(Rgba::hex(ctx.theme.palette().navy).mix(Rgba::WHITE, 0.8).0)
                }),
                line_height: 1.4,
                tracking_em: 0.14,
                uppercase: true,
                ..Default::default()
            },
        );
        reroll.wrap = false;
        let button = boxed(
            "picker/foot/button",
            button,
            vec![
                c::Element::Spacer.into(),
                c::Element::Text(reroll).into(),
                c::Element::Spacer.into(),
            ],
            Axis::Row,
            0.,
        );
        let left = boxed(
            "",
            plain(None, [0., gap, 0., 0.], 0.),
            vec![sw, shrink(name), shrink(button)],
            Axis::Row,
            gap,
        );
        let mut acts = vec![];
        let parameter_actions: Vec<(String, String)> =
            if self.params.as_ref().is_some_and(|p| p.editing.is_some()) {
                vec![("Set".into(), "↵".into()), ("Keep".into(), "esc".into())]
            } else if self.params.as_ref().is_some_and(|p| p.choosing.is_none()) {
                vec![
                    ("Launch".into(), "↵".into()),
                    ("Change".into(), "→".into()),
                    ("Back".into(), "←".into()),
                ]
            } else {
                vec![]
            };
        let actions = if parameter_actions.is_empty() {
            &self.actions
        } else {
            &parameter_actions
        };
        for (i, (label, kbd)) in actions.iter().enumerate() {
            if i > 0 {
                let key = format!(
                    "picker/foot/pill/sep{}",
                    if i == 1 {
                        String::new()
                    } else {
                        (i - 1).to_string()
                    }
                );
                acts.push(c::Item {
                    element: leaf(
                        &key,
                        [1., if spot { 16. } else { 12. }],
                        ui::Paint {
                            color: ui::Color::Line,
                            alpha: if spot {
                                ui::Alpha::Hairline
                            } else {
                                ui::Alpha::Value(1.)
                            },
                        },
                        0.,
                    ),
                    extent: c::Extent::Fixed(1.),
                });
            }
            let key = format!(
                "picker/foot/pill/span{}",
                if i == 0 { String::new() } else { i.to_string() }
            );
            acts.push(shrink(text(
                &key,
                label,
                if spot {
                    STYLE.r#type.body
                } else {
                    STYLE.r#type.caption
                } as f32,
                400,
                if spot {
                    ui::Color::Foreground
                } else {
                    ui::Color::Muted
                },
                false,
                true,
            )));
            let key = format!(
                "picker/foot/pill/kbd{}",
                if i == 0 { String::new() } else { i.to_string() }
            );
            let mut s = plain(
                None,
                if spot {
                    [2., 6., 2., 6.]
                } else {
                    [1., 5., 1., 5.]
                },
                if spot {
                    STYLE.radius.tag as f32
                } else {
                    TOKENS.radius.chip as f32
                },
            );
            s.overrides.fill = Some(paint(
                ui::Color::Muted,
                if spot {
                    STYLE.tint.badge_alpha as f32
                } else {
                    0.14
                },
            ));
            acts.push(shrink(boxed(
                &key,
                s,
                vec![
                    text(
                        "",
                        kbd,
                        if spot {
                            STYLE.r#type.caption
                        } else {
                            STYLE.r#type.small
                        } as f32,
                        400,
                        ui::Color::Foreground,
                        true,
                        false,
                    )
                    .into(),
                ],
                Axis::Row,
                0.,
            )));
        }
        let mut s = if spot {
            ui::Style::new(ui::Preset::Pill)
        } else {
            plain(None, [0.; 4], 0.)
        };
        if spot {
            s.overrides.padding = Some([
                ui::Length::Px(6.),
                ui::Length::Px(12.),
                ui::Length::Px(6.),
                ui::Length::Px(12.),
            ]);
            s.overrides.fill = Some(paint(ui::Color::Surface, 0.85));
            s.overrides.shadow = Some(ui::Shadow::Pill);
            s.overrides.rings = Some(vec![ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: true,
            }]);
        }
        let pill = boxed(
            "picker/foot/pill",
            s,
            acts,
            Axis::Row,
            if spot { 8. } else { 6. },
        );
        let mut s = plain(
            None,
            if spot {
                [10., 14., 12., 20.]
            } else {
                [8., 12., 8., 12.]
            },
            0.,
        );
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: if spot {
                    ui::Alpha::Hairline
                } else {
                    ui::Alpha::Value(1.)
                },
            },
            outside: false,
        });
        s.overrides.border_sides = Some([true, false, false, false]);
        boxed(
            "picker/foot",
            s,
            vec![
                shrink(left),
                c::Item {
                    element: c::Element::Spacer,
                    extent: c::Extent::Fill,
                },
                shrink(pill),
            ],
            Axis::Row,
            0.,
        )
    }
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let view = self.view();
        let mut list = vec![];
        if let Some(cap) = &self.cap {
            let title = format!(
                "Spawn cap reached · {}{} · {} of {}",
                cap.scope,
                if cap.name.is_empty() {
                    String::new()
                } else {
                    format!(" {}", cap.name)
                },
                cap.used,
                cap.limit
            );
            let caption = format!(
                "{} · close a seat or raise the cap in Settings · Go to still works",
                if cap.counts.is_empty() {
                    "running, starting and grace processes count"
                } else {
                    &cap.counts
                }
            );
            let waiting = ui::Color::Rgba(Rgba::hex(TOKENS.color.state.waiting).0);
            let mut style = plain(None, [7., 10., 7., 10.], STYLE.radius.row as f32);
            style.overrides.fill = Some(paint(waiting, 0.12));
            style.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: paint(waiting, 0.35),
                outside: false,
            });
            let mut caption = c::Text::new(
                caption,
                ui::Text {
                    size: Some(STYLE.r#type.note as f32),
                    paint: ui::Paint::solid(ui::Color::Muted),
                    line_height: 1.35,
                    ..Default::default()
                },
            );
            caption.wrap = true;
            let cap = boxed(
                "picker/div/capnote",
                style,
                vec![
                    text(
                        "picker/div/capnote/b",
                        title,
                        STYLE.r#type.section as f32,
                        600,
                        ui::Color::Foreground,
                        false,
                        true,
                    )
                    .into(),
                    named("picker/div/capnote/small", c::Element::Text(caption)).into(),
                ],
                Axis::Column,
                2.,
            );
            list.push(
                boxed(
                    "",
                    plain(None, [0., 6., 4., 6.], 0.),
                    vec![cap.into()],
                    Axis::Column,
                    0.,
                )
                .into(),
            );
        }
        if view.rows.is_empty() {
            list.push(
                boxed(
                    "picker/div/empty",
                    plain(None, [10., 12., 10., 12.], 0.),
                    vec![
                        text(
                            "",
                            format!("nothing matches \"{}\"", self.query),
                            STYLE.r#type.body as f32,
                            400,
                            ui::Color::Muted,
                            false,
                            true,
                        )
                        .into(),
                    ],
                    Axis::Column,
                    0.,
                )
                .into(),
            );
        }
        let mut last = None;
        let mut sec = 0;
        for (i, it) in view.rows.iter().enumerate() {
            if it.section.is_some() && last != it.section.as_deref() {
                last = it.section.as_deref();
                let key = format!(
                    "picker/div/sec{}",
                    if sec == 0 {
                        String::new()
                    } else {
                        sec.to_string()
                    }
                );
                sec += 1;
                let mut t = c::Text::new(
                    last.unwrap(),
                    ui::Text {
                        size: Some(if self.spotlight {
                            STYLE.r#type.section
                        } else {
                            STYLE.r#type.small
                        } as f32),
                        weight: if self.spotlight { 500 } else { 600 },
                        tracking_em: if self.spotlight { 0. } else { 0.06 },
                        paint: ui::Paint::solid(ui::Color::Muted),
                        line_height: 1.4,
                        ..Default::default()
                    },
                );
                t.wrap = false;
                list.push(
                    boxed(
                        &key,
                        plain(
                            None,
                            if self.spotlight {
                                [12., 24., 6., 24.]
                            } else {
                                [8., 12., 2., 12.]
                            },
                            0.,
                        ),
                        vec![c::Element::Text(t).into()],
                        Axis::Column,
                        0.,
                    )
                    .into(),
                );
            }
            let row = self.row(it, i, i == view.selected, ctx);
            list.push(row.into());
        }
        if let Some(params) = &self.params
            && params.choosing.is_none()
        {
            let mut style = plain(
                None,
                [
                    STYLE.spacing.s4 as f32,
                    STYLE.spacing.s6 as f32,
                    STYLE.spacing.s4 as f32,
                    STYLE.spacing.s6 as f32,
                ],
                0.,
            );
            style.overrides.border = Some(ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint::solid(ui::Color::Line),
                outside: false,
            });
            style.overrides.border_sides = Some([true, false, false, false]);
            let mut row = c::Box::new(
                style,
                vec![
                    text(
                        "picker/div/cmdrow/k",
                        "Runs",
                        STYLE.r#type.caption as f32,
                        400,
                        ui::Color::Muted,
                        false,
                        false,
                    )
                    .into(),
                    text(
                        "picker/div/cmdrow/cmdline",
                        &params.runs,
                        STYLE.r#type.caption as f32,
                        400,
                        ui::Color::Accent,
                        true,
                        true,
                    )
                    .into(),
                ],
            );
            row.direction = Axis::Row;
            row.gap = ui::Length::Px(STYLE.spacing.s5 as f32);
            row.align = c::Alignment::Baseline;
            list.push(named("picker/div/cmdrow", c::Element::Box(row)).into());
        }
        let mut style = if self.spotlight {
            ui::Style::new(ui::Preset::Modal)
        } else {
            ui::Style::new(ui::Preset::Card)
        };
        style.overrides.padding = Some([ui::Length::Px(0.); 4]);
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint::solid(ui::Color::Line),
            outside: false,
        });
        if !self.spotlight {
            style.overrides.radii = Some([ui::Length::Px(STYLE.radius.card as f32); 4]);
            style.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface));
            style.overrides.shadow = Some(ui::Shadow::Positioned {
                offset: [0., 20.],
                blur: 60.,
                color: [0., 0., 0., 0.45],
            });
        } else {
            style.overrides.rings = Some(vec![ui::Border {
                width: ui::Length::Hairline,
                paint: ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Hairline,
                },
                outside: true,
            }]);
        }
        let mut panel = c::Box::new(
            style,
            vec![
                self.header(&view).into(),
                boxed(
                    "",
                    plain(
                        None,
                        [if self.cap.is_some() { 6. } else { 0. }, 0., 0., 0.],
                        0.,
                    ),
                    vec![named("picker/div", stack(list, Axis::Column, 0.)).into()],
                    Axis::Column,
                    0.,
                )
                .into(),
                self.foot(ctx).into(),
            ],
        );
        panel.direction = Axis::Column;
        panel.align = c::Alignment::Stretch;
        panel.gap = ui::Length::Px(0.);
        panel.clip = true;
        named("picker", c::Element::Box(panel))
    }
}

fn hide_number(e: &mut c::Element) {
    match e {
        c::Element::Named { content, .. } => hide_number(content),
        c::Element::Box(b) => {
            b.style.overrides.fill = Some(ui::Paint::solid(ui::Color::Transparent));
            for c in &mut b.children {
                hide_number(&mut c.element);
            }
        }
        c::Element::Text(t) => {
            for r in &mut t.runs {
                r.style.paint = ui::Paint::solid(ui::Color::Transparent)
            }
        }
        _ => {}
    }
}
