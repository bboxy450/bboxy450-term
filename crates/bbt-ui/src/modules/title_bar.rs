//! Window chrome composed from retained L6 elements. The app owns all actions.
use super::{seat_row::named, strip::plain};
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, layout::Axis};

#[derive(Debug, Clone)]
pub struct TitleBar {
    pub title: String,
    pub crumb: String,
    pub sidebar_open: bool,
    pub blurred: bool,
}
impl Default for TitleBar {
    fn default() -> Self {
        Self {
            title: "bboxy450-term".into(),
            crumb: String::new(),
            sidebar_open: true,
            blurred: false,
        }
    }
}
fn flexible(element: c::Element) -> c::Item {
    c::Item {
        element,
        extent: c::Extent::Content {
            grow: 0.,
            shrink: 1.,
            min: 0.,
            max: None,
            max_fraction: None,
        },
    }
}
fn label(key: &str, value: &str, weight: u16, muted: bool, wrap: bool) -> c::Element {
    let mut text = c::Text::new(
        value,
        ui::Text {
            weight,
            paint: ui::Paint::solid(if muted {
                ui::Color::Muted
            } else {
                ui::Color::Foreground
            }),
            // .titlebar.app-bar inherits the lab's 1.4 line height.
            line_height: 1.4,
            ..Default::default()
        },
    );
    text.wrap = wrap;
    text.ellipsis = !wrap;
    named(key, c::Element::Text(text))
}
fn toggle(key: &str, icon: &str, selected: bool) -> c::Element {
    let mut style = plain(
        None,
        [STYLE.spacing.s2 as f32; 4],
        STYLE.radius.badge as f32,
    );
    if selected {
        style.overrides.fill = Some(ui::Paint {
            color: ui::Color::Foreground,
            alpha: ui::Alpha::Selection,
        });
    }
    named(
        key,
        c::Element::Box(c::Box::new(
            style,
            vec![
                named(
                    format!("{key}/hicon"),
                    c::Element::Icon {
                        name: icon.into(),
                        paint: ui::Paint::solid(if selected {
                            ui::Color::Foreground
                        } else {
                            ui::Color::Muted
                        }),
                        // .tbtn --icon in lab.css; fixed independently of density.
                        size: ui::Length::Px(18.),
                    },
                )
                .into(),
            ],
        )),
    )
}
impl TitleBar {
    pub fn element(&self, ctx: ui::Context) -> c::Element {
        let dots = (0..3)
            .map(|i| {
                // .win i: 10px circles, with 6px gaps; no OS-native chrome.
                let mut dot = plain(Some(10.), [0.; 4], 5.);
                dot.overrides.width = Some(ui::Length::Px(10.));
                dot.overrides.fill = Some(ui::Paint {
                    color: ui::Color::Line,
                    alpha: ui::Alpha::Value(if self.blurred { 0.6 } else { 1. }),
                });
                named(
                    if i == 0 {
                        "bar/win/i".into()
                    } else {
                        format!("bar/win/i{i}")
                    },
                    c::Element::Box(c::Box::new(dot, vec![])),
                )
                .into()
            })
            .collect();
        let mut logo = c::Element::Image {
            asset: "bboxy".into(),
            size: ui::Length::Px(18.),
            radius: ui::Length::Px(9.),
        };
        if self.blurred {
            logo = c::Element::Filter {
                content: Box::new(logo),
                saturation: 0.4,
                opacity: 0.8,
            };
        }
        // Reuse the frosted panel's Style (including its inset highlight),
        // replacing its geometry with the app-bar's tokenised dimensions.
        let mut style = ui::Style::new(ui::Preset::Modal);
        style.overrides.height = Some(ui::Length::Px(STYLE.spacing.title_bar as f32));
        style.overrides.padding = Some([
            ui::Length::Px(0.),
            ui::Length::Spacing(7),
            ui::Length::Px(0.),
            ui::Length::Spacing(7),
        ]);
        style.overrides.radii = Some([ui::Length::Px(0.); 4]);
        style.overrides.shadow = Some(ui::Shadow::None);
        style.overrides.border_sides = Some([false, false, true, false]);
        style.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: false,
        });
        let mut bar = c::Box::new(
            style,
            vec![
                named(
                    "bar/win",
                    c::Element::Stack(c::Stack {
                        direction: Axis::Row,
                        gap: ui::Length::Spacing(3),
                        align: c::Alignment::Center,
                        children: dots,
                    }),
                )
                .into(),
                toggle(
                    "bar/tbtn",
                    if self.sidebar_open {
                        "sidebarfoxopen"
                    } else {
                        "sidebarfox"
                    },
                    self.sidebar_open,
                )
                .into(),
                named("bar/logo", logo).into(),
                flexible(label(
                    "bar/ttl",
                    &self.title,
                    STYLE.r#type.weight_title as u16,
                    false,
                    true,
                )),
                flexible(label("bar/crumb", &self.crumb, 400, true, false)),
                c::Item {
                    element: named("bar/spacer", c::Element::Spacer),
                    extent: c::Extent::Fill,
                },
                toggle(
                    "bar/tbtn1",
                    if ctx.theme == ui::Theme::Dark {
                        "sun"
                    } else {
                        "moon"
                    },
                    false,
                )
                .into(),
            ],
        );
        bar.gap = ui::Length::Spacing(5);
        bar.cross_overflow = true;
        named("bar", c::Element::Box(bar))
    }
}
