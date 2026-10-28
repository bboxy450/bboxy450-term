// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AttachmentPicker {
    pub query: String,
    pub edit: Option<Edit>,
    pub items: Vec<Attachment>,
    pub sel: usize,
}
impl AttachmentPicker {
    pub fn rows(&self) -> Vec<&Attachment> {
        let query = self.query.to_lowercase();
        self.items
            .iter()
            .filter(|it| it.title.to_lowercase().contains(&query))
            .collect()
    }
}
impl Controller {
    pub fn replace_attachments(&mut self, items: Vec<Attachment>) {
        self.attachment_catalog = items;
        if let Some(p) = &mut self.view.attach_pick {
            let selected = p.rows().get(p.sel).map(|a| a.path.clone());
            p.items = self.attachment_catalog.clone();
            let rows = p.rows();
            p.sel = selected
                .and_then(|id| rows.iter().position(|a| a.path == id))
                .unwrap_or(p.sel.min(rows.len().saturating_sub(1)));
        }
    }
    pub(super) fn open_attachments(&mut self) {
        if self.view.attach_pick.is_some() {
            return;
        }
        self.body
            .handle(&mut self.view.text, EditCommand::CancelPreedit);
        self.address
            .handle(&mut self.view.typed, EditCommand::CancelPreedit);
        self.attachment_editor.reset("");
        self.view.attach_pick = Some(AttachmentPicker {
            items: self.attachment_catalog.clone(),
            edit: Some(self.attachment_editor.edit()),
            ..Default::default()
        });
        self.view.field = Field::Body;
        self.view.focus = true;
        self.view.pick.clear();
        self.view.kind_menu = false;
    }
    pub(super) fn attachment_command(&mut self, command: &Command) -> Option<Outcome> {
        self.view.attach_pick.as_ref()?;
        match command {
            Command::Edit(command) => {
                let p = self.view.attach_pick.as_mut().unwrap();
                let before = p.query.clone();
                let changed = self.attachment_editor.handle(&mut p.query, command.clone());
                if p.query != before {
                    p.sel = 0;
                }
                p.edit = Some(self.attachment_editor.edit());
                Some(if changed {
                    Outcome::Changed
                } else {
                    Outcome::Local
                })
            }
            Command::Next { backwards } => {
                let p = self.view.attach_pick.as_mut().unwrap();
                let count = p.rows().len();
                if count == 0 {
                    return Some(Outcome::Local);
                }
                p.sel = if *backwards {
                    (p.sel + count - 1) % count
                } else {
                    (p.sel + 1) % count
                };
                Some(Outcome::Changed)
            }
            Command::ChooseSelected | Command::ChooseAttachment(_) => {
                if self.attachment_editor.composing() {
                    return Some(Outcome::Local);
                }
                let p = self.view.attach_pick.as_ref().unwrap();
                let index = match command {
                    Command::ChooseAttachment(i) => *i,
                    _ => p.sel,
                };
                let Some(attachment) = p.rows().get(index).cloned().cloned() else {
                    return Some(Outcome::Local);
                };
                if attachment.path.is_empty() {
                    return Some(Outcome::Refused("attachment_unresolved"));
                }
                self.view.attach_pick = None;
                self.view.field = Field::Body;
                Some(self.handle(Command::Attach(attachment)))
            }
            Command::Escape => {
                self.view.attach_pick = None;
                self.view.field = Field::Body;
                self.sync();
                Some(Outcome::Changed)
            }
            Command::Send => Some(Outcome::Local),
            Command::Focus(_) | Command::Blur | Command::ToggleKindMenu | Command::Delegate(_) => {
                self.view.attach_pick = None;
                None
            }
            _ => None,
        }
    }
}

use crate::modules::{
    seat_row::named,
    stream_dock::{column, fill, label, row, text_style, tint},
    strip::plain,
};
use crate::{self as ui, components as c, widgets::text_area::TextArea};
use bbt_ui_model::{
    STYLE,
    layout::{Axis, TextStyle},
};
fn rule(mut s: ui::Style, top: bool) -> ui::Style {
    s.overrides.border = Some(ui::Border {
        width: ui::Length::Hairline,
        paint: ui::Paint {
            color: ui::Color::Line,
            alpha: ui::Alpha::Hairline,
        },
        outside: false,
    });
    s.overrides.border_sides = Some(if top {
        [true, false, false, false]
    } else {
        [false, false, true, false]
    });
    s
}
impl AttachmentPicker {
    pub(super) fn element(
        &self,
        key: &str,
        ctx: ui::Context,
        width: f32,
        m: &dyn c::Measure,
    ) -> ui::style::Result<c::Element> {
        let sp = &STYLE.spacing;
        let t = &STYLE.r#type;
        let head_style = text_style(t.row as f32, 400, false, ui::Color::Foreground);
        let mark_font = TextStyle {
            size: t.row as f32,
            weight: 400,
            mono: true,
            italic: false,
            tracking_em: 0.,
        };
        let query_width =
            (width - 2. * sp.s5 as f32 - sp.s3 as f32 - m.width("[[", &mark_font)).max(1.);
        let query = TextArea {
            value: self.query.clone(),
            placeholder: "a task or artifact".into(),
            style: head_style,
            edit: Some(self.edit.clone().unwrap_or_default()),
            min_lines: 1,
            wrap: false,
            name: format!("{key}/query"),
        }
        .element(query_width, ctx, m)?;
        let mut clip = c::Box::new(plain(None, [0.; 4], 0.), vec![query.into()]);
        clip.clip = true;
        clip.gap = ui::Length::Px(0.);
        let head = row(
            format!("{key}/head"),
            rule(
                plain(
                    None,
                    [sp.s3 as f32, sp.s5 as f32, sp.s3 as f32, sp.s5 as f32],
                    0.,
                ),
                false,
            ),
            sp.s3 as f32,
            c::Alignment::Center,
            vec![
                label(
                    format!("{key}/mark"),
                    "[[",
                    text_style(t.row as f32, 400, true, ui::Color::Accent),
                    false,
                )
                .into(),
                fill(c::Element::Box(clip)),
            ],
        );
        let rows = self.rows();
        let count = rows.len();
        let h = (t.small as f32 * 1.4 * 64.).floor() / 64. + 2. * sp.s3 as f32;
        let mut items = vec![];
        for (i, a) in rows.into_iter().enumerate() {
            let k = format!("{key}/o{i}");
            let mut s = plain(
                Some(h),
                [sp.s3 as f32, sp.s5 as f32, sp.s3 as f32, sp.s5 as f32],
                STYLE.radius.row as f32,
            );
            if i == self.sel.min(count.saturating_sub(1)) {
                s.overrides.fill = Some(tint(ui::Color::Accent, 0.28));
            }
            let glyph = if a.kind == "task" {
                super::view::task_mark(&format!("{k}/glyph"), &a.status, 14., ui::Color::Muted)
            } else {
                let mut glyph = c::Text::new(
                    super::view::artifact_glyph(&a.akind),
                    text_style(t.small as f32, 400, false, ui::Color::Muted),
                );
                glyph.wrap = false;
                glyph.align = c::Alignment::Center;
                named(format!("{k}/glyph"), c::Element::Text(glyph))
            };
            let mut kind = text_style(t.micro as f32, 400, true, ui::Color::Muted);
            kind.uppercase = true;
            kind.tracking_em = 0.06;
            let row = row(
                &k,
                s,
                sp.s4 as f32,
                c::Alignment::Center,
                vec![
                    c::Item {
                        element: glyph,
                        extent: c::Extent::Fixed(14.),
                    },
                    fill(label(
                        format!("{k}/title"),
                        &a.title,
                        text_style(t.small as f32, 400, false, ui::Color::Foreground),
                        false,
                    )),
                    label(
                        format!("{k}/meta"),
                        if a.kind == "task" { &a.status } else { &a.meta },
                        text_style(t.small as f32, 400, false, ui::Color::Muted),
                        false,
                    )
                    .into(),
                    label(format!("{k}/kind"), &a.kind, kind, false).into(),
                ],
            );
            items.push(
                column(
                    "",
                    plain(None, [0., sp.s2 as f32, 0., sp.s2 as f32], 0.),
                    vec![row.into()],
                )
                .fixed_height(h, &k, 0.)
                .into(),
            );
        }
        let total = if count == 0 {
            items.push(
                column(
                    format!("{key}/empty"),
                    plain(
                        None,
                        [sp.s4 as f32, sp.s5 as f32, sp.s4 as f32, sp.s5 as f32],
                        0.,
                    ),
                    vec![
                        label(
                            format!("{key}/empty/text"),
                            format!("nothing matches \"{}\"", self.query),
                            text_style(t.small as f32, 400, false, ui::Color::Muted),
                            false,
                        )
                        .into(),
                    ],
                )
                .into(),
            );
            (t.small as f32 * 1.4 * 64.).floor() / 64. + 2. * sp.s4 as f32
        } else {
            count as f32 * h
        };
        let viewport = total.min(220. - 2. * sp.s2 as f32);
        let scroll = if count > 0 {
            (self.sel.min(count - 1) as f32 * h + h - viewport).max(0.)
        } else {
            0.
        };
        let list = column(
            format!("{key}/list"),
            plain(None, [sp.s2 as f32, 0., sp.s2 as f32, 0.], 0.),
            vec![
                c::Element::ScrollView(c::ScrollView {
                    viewport: [width, viewport],
                    offset: [0., scroll],
                    content_width: Some(width),
                    content: Box::new(column("", plain(None, [0.; 4], 0.), items)),
                })
                .into(),
            ],
        );
        let foot = column(
            format!("{key}/foot"),
            rule(
                plain(
                    None,
                    [sp.s2 as f32, sp.s5 as f32, sp.s2 as f32, sp.s5 as f32],
                    0.,
                ),
                true,
            ),
            vec![
                label(
                    format!("{key}/foot/text"),
                    "↵ attach · ↑↓ · esc",
                    text_style(t.small as f32, 400, false, ui::Color::Muted),
                    false,
                )
                .into(),
            ],
        );
        let mut s = plain(None, [0.; 4], STYLE.radius.card as f32);
        s.overrides.width = Some(ui::Length::Px(width));
        s.overrides.fill = Some(ui::Paint::solid(ui::Color::Surface));
        s.overrides.shadow = Some(ui::Shadow::Floating);
        s.overrides.border = Some(ui::Border {
            width: ui::Length::Hairline,
            paint: ui::Paint {
                color: ui::Color::Line,
                alpha: ui::Alpha::Hairline,
            },
            outside: true,
        });
        let mut b = c::Box::new(s, vec![head.into(), list.into(), foot.into()]);
        b.direction = Axis::Column;
        b.align = c::Alignment::Stretch;
        b.gap = ui::Length::Px(0.);
        b.clip = true;
        Ok(named(key, c::Element::Box(b)))
    }
}
