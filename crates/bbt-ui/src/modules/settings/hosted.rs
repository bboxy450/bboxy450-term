// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Hosted Settings reuses the reference cards and adds caller-owned interaction.
//! Adapted from bboxy450-term's web Settings. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;
use controller::{Field, Presentation};

/// Availability changes launcher membership; observed connection state is separate.
pub(super) fn availability(view: &Settings, b: &Build, key: &str) -> Option<c::Element> {
    let id = view.values.destination_id(key)?;
    Some(row(
        &format!("destination/{id}/available"),
        plain(Some(28.), [0.; 4], 0.),
        8.,
        vec![
            fill(b.text("", "Available in launcher", 11., 400, false, false)),
            b.toggle("", !view.values.disabled_destinations.contains(id))
                .into(),
        ],
    ))
}

fn card_build<'a>(b: &Build<'a>, width: f32) -> Build<'a> {
    let columns = (((width + 10.) / 240.).floor() as usize).max(1);
    Build {
        width: (width - 10. * (columns - 1) as f32) / columns as f32 - 24.,
        ..*b
    }
}
fn alias(element: &mut c::Element, key: &str, name: &str) {
    match element {
        c::Element::Named {
            name: found,
            content,
        } if found == key => {
            let inner = std::mem::replace(content, Box::new(c::Element::Spacer));
            **content = named(name, *inner);
        }
        c::Element::Named { content, .. } | c::Element::Baseline { content, .. } => {
            alias(content, key, name)
        }
        c::Element::Box(b) => {
            for item in &mut b.children {
                alias(&mut item.element, key, name);
            }
        }
        c::Element::Layers { base, overlays } => {
            alias(base, key, name);
            for placed in overlays {
                alias(&mut placed.content, key, name);
            }
        }
        _ => {}
    }
}
fn append_card(element: &mut c::Element, item: c::Element) {
    match element {
        c::Element::Named { content, .. } => append_card(content, item),
        c::Element::Box(b) => b.children.push(item.into()),
        _ => unreachable!("Settings card root"),
    }
}
fn class_caption(element: &mut c::Element, key: &str, caption: &c::Element) {
    match element {
        c::Element::Named { name, content } if name == key => {
            if let c::Element::Box(b) = content.as_mut()
                && let Some(last) = b.children.last_mut()
            {
                last.element = caption.clone();
            }
        }
        c::Element::Named { content, .. } | c::Element::Baseline { content, .. } => {
            class_caption(content, key, caption)
        }
        c::Element::Box(b) => {
            for item in &mut b.children {
                class_caption(&mut item.element, key, caption);
            }
        }
        _ => {}
    }
}
fn text_input(
    b: &Build,
    name: &str,
    field: Field,
    value: &str,
    placeholder: &str,
    state: &Presentation,
    area: bool,
) -> c::Element {
    let active = state
        .focused_text
        .as_ref()
        .filter(|text| text.field == field);
    let value = active.map_or(value, |text| text.value.as_str());
    let style = font(
        if area { 11. } else { 12.5 },
        400,
        area,
        ui::Color::Foreground,
    );
    let prompt_height = if area {
        let sample = wrap(
            "",
            value,
            ui::Text {
                line_height: 1.45,
                ..style
            },
        );
        (sample
            .natural((b.width - 18.).max(0.), b.ctx, b.m)
            .expect("prompt geometry")
            .0[1]
            + 10.)
            .clamp(88., 240.)
    } else {
        0.
    };
    let content = if let Some(text) = active {
        let edit = c::Element::EditingText(c::EditingText {
            value: value.into(),
            placeholder: placeholder.into(),
            style,
            placeholder_paint: ui::Paint::solid(ui::Color::Muted),
            edit: text.edit.clone(),
            name: format!("{name}/edit"),
        });
        let viewport = [
            (b.width - 18.).max(1.),
            if area { prompt_height - 10. } else { 17. },
        ];
        let caret = c::utf16_byte_index(value, text.edit.caret.unwrap_or(0)).unwrap_or(value.len());
        let caret_width = measured(b.m, &value[..caret], style);
        let text_width = edit
            .natural(f32::INFINITY, b.ctx, b.m)
            .expect("editor geometry")
            .0[0];
        c::Element::ScrollView(c::ScrollView {
            viewport,
            offset: [(caret_width - viewport[0] + 8.).max(0.), 0.],
            content_width: Some(text_width.max(viewport[0]) + 8.),
            content: Box::new(if area {
                col("", plain(None, [0.; 4], 0.), 0., vec![fixed(edit, 17.)])
            } else {
                edit
            }),
        })
    } else {
        wrap(
            "",
            if value.is_empty() { placeholder } else { value },
            style,
        )
    };
    let mut box_ = c::Box::new(
        border(
            surface(
                ui::Color::Background,
                0.6,
                6.,
                [5., 9., 5., 9.],
                area.then_some(prompt_height),
            ),
            hair(b.ctx),
            true,
        ),
        vec![fill(content)],
    );
    box_.clip = true;
    named(name, c::Element::Box(box_))
}
fn role_card(
    b: &Build,
    key: &str,
    name: &str,
    role: &Role,
    default: Option<&Role>,
    state: &Presentation,
) -> c::Element {
    let mut card = b.role_card(key, name, role, default);
    class_caption(
        &mut card,
        &format!("{key}/f2"),
        &b.note("", "Change class before launching seats", true),
    );
    if state
        .focused_text
        .as_ref()
        .is_some_and(|text| text.field == Field::RolePrompt(name.into()))
    {
        replace_named(
            &mut card,
            &format!("{key}/f0/control"),
            &text_input(
                b,
                &controller::role_region(name, "prompt"),
                Field::RolePrompt(name.into()),
                &role.prompt,
                "You are …",
                state,
                true,
            ),
        );
    } else {
        alias(
            &mut card,
            &format!("{key}/f0/control"),
            &controller::role_region(name, "prompt"),
        );
    }
    replace_named(
        &mut card,
        &format!("{key}/f1/control"),
        &named(
            controller::role_region(name, "send"),
            b.text("", "Granted by role class", 12.5, 400, false, true),
        ),
    );
    replace_named(
        &mut card,
        &format!("{key}/f2/control"),
        &b.select(
            &controller::role_region(name, "class"),
            &["worker".into(), "oracle".into()],
            &role.cls,
            false,
            true,
        ),
    );
    if role.custom {
        alias(
            &mut card,
            &format!("{key}/delete"),
            &controller::role_region(name, "delete"),
        );
    }
    if default.is_some_and(|d| d.prompt != role.prompt) {
        let label = row(
            "",
            plain(None, [0.; 4], 0.),
            8.,
            vec![
                fill(b.text("", "Default system prompt", 12.5, 500, false, false)),
                b.diff(&controller::role_region(name, "reset")).into(),
            ],
        );
        replace_named(&mut card, &format!("{key}/f0/label"), &label);
    }
    card
}
fn draft_card(b: &Build, draft: &controller::DraftRole, state: &Presentation) -> c::Element {
    let key = "role/draft";
    let mut card = b.role_card(key, &draft.name, &draft.role, None);
    class_caption(
        &mut card,
        &format!("{key}/f2"),
        &b.note("", "Choose the priority class for this role", true),
    );
    let co = color(&draft.role.color);
    let swatch = named(
        "role/draft/color",
        row(
            "",
            surface(co, 1., 4., [0.; 4], Some(18.)),
            0.,
            vec![fixed(c::Element::Spacer, 18.)],
        ),
    );
    let chip = b.chip("", "new", true, Some(co), None);
    let chip_width = chip
        .natural(f32::INFINITY, b.ctx, b.m)
        .expect("draft chip geometry")
        .0[0];
    let name_build = Build {
        width: (b.width - 18. - 16. - chip_width).max(1.),
        ..*b
    };
    let band = row(
        "",
        surface(co, 0.18, 0., [0., 12., 0., 12.], Some(36.)),
        8.,
        vec![
            swatch.into(),
            fill(text_input(
                &name_build,
                "role/draft/name",
                Field::RoleDraftName,
                &draft.name,
                "role name",
                state,
                false,
            )),
            chip.into(),
        ],
    );
    replace_named(&mut card, "role/draft/band", &band);
    replace_named(
        &mut card,
        "role/draft/f0/control",
        &text_input(
            b,
            "role/draft/prompt",
            Field::RoleDraftPrompt,
            &draft.role.prompt,
            "You are …",
            state,
            true,
        ),
    );
    replace_named(
        &mut card,
        "role/draft/f1/control",
        &b.text("role/draft/send", &draft.role.send, 12.5, 400, false, true),
    );
    replace_named(
        &mut card,
        "role/draft/f2/control",
        &b.select(
            "role/draft/class",
            &["worker".into(), "oracle".into()],
            &draft.role.cls,
            false,
            true,
        ),
    );
    append_card(
        &mut card,
        col(
            "",
            plain(None, [0., 12., 10., 12.], 0.),
            8.,
            vec![
                b.field(
                    "role/draft/mark-field",
                    "Mark",
                    None,
                    b.select(
                        "role/draft/mark",
                        &[
                            "♜".into(),
                            "♔".into(),
                            "♟".into(),
                            "♝".into(),
                            "♖".into(),
                            "✦".into(),
                        ],
                        &draft.role.mark,
                        false,
                        true,
                    ),
                    false,
                    false,
                )
                .into(),
                row(
                    "",
                    plain(None, [0.; 4], 0.),
                    8.,
                    vec![
                        b.button("role/draft/add", "Add role", None, "primary")
                            .into(),
                        b.button("role/draft/cancel", "Cancel", None, "ghost")
                            .into(),
                    ],
                )
                .into(),
            ],
        ),
    );
    card
}
pub(super) fn roles(view: &Settings, b: &Build, width: f32, state: &Presentation) -> Vec<c::Item> {
    let inner = card_build(b, width);
    let mut cards: Vec<_> = view
        .values
        .roles
        .iter()
        .enumerate()
        .map(|(i, (name, role))| {
            role_card(
                &inner,
                &format!("card{i}"),
                name,
                role,
                view.defaults
                    .roles
                    .iter()
                    .find(|(id, _)| id == name)
                    .map(|(_, role)| role),
                state,
            )
        })
        .collect();
    cards.push(if let Some(draft) = &state.role_draft {
        draft_card(&inner, draft, state)
    } else {
        b.add_card(
            "role/new",
            "+ new role",
            "your own mark, colour and prompt",
            160.,
        )
    });
    vec![b.grid("roles", width, 230., 10., cards).into()]
}
pub(super) fn harnesses(view: &Settings, b: &Build, width: f32) -> Vec<c::Item> {
    let inner = card_build(b, width);
    let cards = view
        .values
        .harness
        .iter()
        .enumerate()
        .map(|(i, (name, h))| {
            let key = format!("card{i}");
            let mut card = inner.harness_card(
                &key,
                name,
                h,
                view.defaults
                    .harness
                    .iter()
                    .find(|(id, _)| id == name)
                    .map(|(_, h)| h),
                &view.launch,
                &h.available_roles,
            );
            alias(
                &mut card,
                &format!("{key}/r0/ctl"),
                &format!("harness/{name}/role"),
            );
            alias(
                &mut card,
                &format!("{key}/r0/diff"),
                &format!("harness/{name}/reset"),
            );
            for (row, value) in [
                (1, h.wake.as_str()),
                (2, h.state_from.as_str()),
                (3, if h.hooks { "on" } else { "off" }),
            ] {
                replace_named(
                    &mut card,
                    &format!("{key}/r{row}/ctl"),
                    &inner.text("", value, 11., 400, false, true),
                );
            }
            let available = h
                .eligible_roles
                .iter()
                .map(|role| {
                    let default = h.role == *role;
                    let on = h.available_roles.contains(role);
                    row(
                        &format!("harness/{name}/available/{role}"),
                        plain(Some(28.), [0.; 4], 0.),
                        6.,
                        vec![
                            fill(inner.text("", role, 11., 400, false, false)),
                            if default {
                                inner.chip("", "default", true, None, None)
                            } else {
                                inner.toggle("", on)
                            }
                            .into(),
                        ],
                    )
                    .into()
                })
                .collect();
            append_card(
                &mut card,
                inner.field(
                    &format!("harness/{name}/available"),
                    "Available roles",
                    Some("The default stays available"),
                    col("", plain(None, [0.; 4], 0.), 0., available),
                    false,
                    false,
                ),
            );
            card
        })
        .collect();
    vec![b.grid("harnesses", width, 230., 10., cards).into()]
}
pub(super) fn field_name(field: &Field) -> Option<String> {
    Some(match field {
        Field::RoleDraftColor => "role/draft/color".into(),
        Field::RoleDraftMark => "role/draft/mark".into(),
        Field::RoleDraftClass => "role/draft/class".into(),
        Field::RoleClass(name) => controller::role_region(name, "class"),
        Field::HarnessRole(name) => format!("harness/{name}/role"),
        _ => return None,
    })
}
pub(super) fn picker(
    root: c::Element,
    state: &Presentation,
    ctx: ui::Context,
    m: &dyn c::Measure,
    width: f32,
    height: f32,
) -> c::Element {
    let Some(picker) = &state.picker else {
        return root;
    };
    let Some(name) = field_name(&picker.field) else {
        return root;
    };
    let Ok(layout) = root.layout(
        bbt_ui_model::layout::Rect::new(0., 0., width, height),
        ctx,
        m,
    ) else {
        return root;
    };
    fn find<'a>(node: &'a c::Layout, name: &str) -> Option<&'a c::Layout> {
        if matches!(&node.kind, c::LayoutKind::Named(id) if id == name) {
            return Some(node);
        }
        node.children.iter().find_map(|node| find(node, name))
    }
    let Some(control) = find(&layout, &name) else {
        return root;
    };
    let width = control
        .rect
        .w
        .max(140.)
        .min((width - control.rect.x).max(1.));
    let b = Build { ctx, m, width };
    let menu_height = (picker.choices.len() as f32 * 28.)
        .min(224.)
        .min((height - 8.).max(1.));
    let y = if control.rect.bottom() + 4. + menu_height <= height {
        control.rect.bottom() + 4.
    } else {
        (control.rect.y - menu_height - 4.).max(0.)
    };
    let items = picker
        .choices
        .iter()
        .enumerate()
        .map(|(i, choice)| {
            row(
                &format!("settings/choice/{i}"),
                surface(
                    if i == picker.selected {
                        ui::Color::Accent
                    } else {
                        ui::Color::Surface
                    },
                    1.,
                    4.,
                    [0., 8., 0., 8.],
                    Some(28.),
                ),
                0.,
                vec![fill(b.text("", choice, 12.5, 400, false, false))],
            )
            .into()
        })
        .collect();
    let menu = named(
        "settings/choices",
        c::Element::ScrollView(c::ScrollView {
            viewport: [width, menu_height],
            offset: [
                0.,
                ((picker.selected + 1) as f32 * 28. - menu_height).max(0.),
            ],
            content_width: None,
            content: Box::new(col(
                "",
                border(
                    surface(ui::Color::Surface, 1., 6., [0.; 4], None),
                    hair(ctx),
                    true,
                ),
                0.,
                items,
            )),
        }),
    );
    c::Element::Layers {
        base: Box::new(root),
        overlays: vec![c::Placed {
            left: control.rect.x,
            bottom: height - y - menu_height,
            content: menu,
        }],
    }
}
