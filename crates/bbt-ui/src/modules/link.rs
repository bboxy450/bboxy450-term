//! Provider link presentation shared by launcher rows, settings and seat chips.
use crate::{self as ui, components as c};
use bbt_ui_model::{STYLE, TOKENS, layout::Axis};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkState {
    Offline,
    Connecting,
    Connected { rtt_ms: u32 },
    Degraded { rtt_ms: u32 },
    Disconnected { reason: String },
}
impl LinkState {
    pub fn down(&self) -> bool {
        matches!(self, Self::Offline | Self::Disconnected { .. })
    }
    pub fn label(&self) -> String {
        match self {
            Self::Offline => "offline".into(),
            Self::Connecting => "connecting…".into(),
            Self::Connected { rtt_ms } => format!("{rtt_ms} ms"),
            Self::Degraded { rtt_ms } => format!("slow · {rtt_ms} ms"),
            Self::Disconnected { reason } => reason.clone(),
        }
    }
    pub fn color(&self) -> ui::Color {
        let value = match self {
            Self::Connected { .. } => TOKENS.color.ui.ok,
            Self::Degraded { .. } => TOKENS.color.state.waiting,
            Self::Disconnected { .. } => TOKENS.color.state.errored,
            Self::Offline | Self::Connecting => return ui::Color::Muted,
        };
        ui::Color::Rgba(bbt_ui_model::draw::Color::hex(value).0)
    }
    pub fn chip_color(&self, default: ui::Color) -> ui::Color {
        match self {
            Self::Offline | Self::Disconnected { .. } => {
                ui::Color::Rgba(bbt_ui_model::draw::Color::hex(TOKENS.color.state.errored).0)
            }
            Self::Degraded { .. } => self.color(),
            _ => default,
        }
    }
    pub fn dot(&self) -> c::Element {
        let mut style = super::strip::plain(Some(8.), [0.; 4], 4.);
        style.overrides.width = Some(ui::Length::Px(8.));
        style.overrides.fill = Some(ui::Paint::solid(if matches!(self, Self::Offline) {
            ui::Color::Transparent
        } else {
            self.color()
        }));
        if matches!(self, Self::Offline) {
            style.overrides.border = Some(ui::Border {
                width: ui::Length::Px(1.5),
                paint: ui::Paint::solid(ui::Color::Muted),
                outside: false,
            });
        }
        c::Element::Box(c::Box::new(style, vec![]))
    }
    pub fn element(&self, size: f32, text: bool) -> c::Element {
        self.named_element(size, text, "")
    }
    /// Retain the lab's inline-flex anatomy for module geometry diagnostics.
    pub fn named_element(&self, size: f32, text: bool, key: &str) -> c::Element {
        let named = |suffix: &str, element: c::Element| {
            if key.is_empty() {
                element
            } else {
                c::Element::Named {
                    name: format!("{key}{suffix}"),
                    content: Box::new(element),
                }
            }
        };
        let mut children = vec![named("/i", self.dot()).into()];
        if text {
            let mut label = c::Text::new(
                self.label(),
                ui::Text {
                    size: Some(size),
                    paint: ui::Paint::solid(ui::Color::Muted),
                    line_height: 1.4,
                    ..Default::default()
                },
            );
            label.wrap = false;
            children.push(named("/span", c::Element::Text(label)).into());
        }
        // CSS inline-flex takes its baseline from the first flex child (the
        // dot), including its vertical centring in the label's line box.
        let height = if text {
            (size * 1.4 * 64.).floor() / 64.
        } else {
            8.
        };
        c::Element::Baseline {
            content: Box::new(named(
                "",
                c::Element::Stack(c::Stack {
                    direction: Axis::Row,
                    align: c::Alignment::Center,
                    gap: ui::Length::Px(STYLE.spacing.s3 as f32),
                    children,
                }),
            )),
            offset: (height - 8.) * 0.5,
        }
    }
}

/// Accept old boolean machine fixtures while preserving new link records.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MachineState {
    Legacy(bool),
    Link(LinkState),
}

/// The optional fifth field keeps older four-field provider records readable.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cloud(
    pub String,
    pub String,
    pub bool,
    pub String,
    #[serde(default)] pub Option<LinkState>,
);
