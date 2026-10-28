// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use bbt_ui::{
    self as ui, components as c,
    cursor::Cursor,
    overlays::{self, Outcome, context_menu as menu, dialog},
};
use bbt_ui_model::layout::{Axis, Rect, TextMeasure, TextStyle};
struct Metrics;
impl TextMeasure for Metrics {
    fn width(&self, text: &str, style: &TextStyle) -> f32 {
        text.chars().count() as f32 * style.size * 0.5
    }
}
impl c::Measure for Metrics {
    fn font_metrics(&self, style: &TextStyle) -> [f32; 2] {
        [style.size * 0.8, style.size * 0.2]
    }
}
fn ctx() -> ui::Context {
    ui::Context::new(ui::Theme::Dark, ui::Density::Cozy)
}
fn label(value: &str) -> c::Element {
    c::Element::Text(c::Text::new(value, ui::Text::default()))
}
fn named(id: &str, body: c::Element) -> c::Element {
    c::Element::Named {
        name: id.into(),
        content: Box::new(body),
    }
}
fn menu() -> menu::Controller {
    let mut disabled = menu::Item::new("blocked", label("Disabled"));
    disabled.enabled = false;
    menu::Controller::new(menu::Menu::new(vec![
        disabled,
        menu::Item::new("rename", label("Rename")),
        menu::Item::new("close", label("Close")),
    ]))
}
#[test]
fn context_menu_skips_disabled_wraps_and_invokes_once() {
    let mut controller = menu();
    assert_eq!(controller.menu.selected, Some(1));
    assert_eq!(
        controller.handle(menu::Command::Next { backwards: true }),
        Outcome::Changed
    );
    assert_eq!(controller.menu.selected, Some(2));
    controller.handle(menu::Command::First);
    assert_eq!(controller.menu.selected, Some(1));
    controller.handle(menu::Command::Last);
    assert_eq!(controller.menu.selected, Some(2));
    assert_eq!(
        controller.handle(menu::Command::Activate(0)),
        Outcome::Local
    );
    assert_eq!(
        controller.handle(menu::Command::Submit { repeat: true }),
        Outcome::Local
    );
    assert_eq!(
        controller.handle(menu::Command::Submit { repeat: false }),
        Outcome::Invoke("close".into())
    );
    assert_eq!(
        controller.handle(menu::Command::Submit { repeat: false }),
        Outcome::Local
    );
}
#[test]
fn context_menu_clamps_all_edges_and_outside_click_dismisses() {
    for anchor in [[-100., -200.], [400., 800.], [30., 40.]] {
        let mut controller = menu();
        let viewport = Rect::new(10., 20., 180., 120.);
        let layout = controller
            .menu
            .layout(anchor, viewport, ctx(), &Metrics)
            .unwrap();
        assert!(layout.rect.x >= viewport.x && layout.rect.y >= viewport.y);
        assert!(
            layout.rect.right() <= viewport.right() + 0.01
                && layout.rect.bottom() <= viewport.bottom() + 0.01
        );
        let regions = menu::Regions::retained(&layout, &controller.menu);
        assert_eq!(controller.click([0., 0.], &regions), Outcome::Dismiss);
        assert!(!controller.is_open());
    }
    assert!(
        menu()
            .menu
            .layout(
                [f32::NAN, 0.],
                Rect::new(0., 0., 100., 100.),
                ctx(),
                &Metrics
            )
            .is_err()
    );
}
#[test]
fn tall_context_menu_reveals_selection_and_clipped_rows_are_not_actionable() {
    let mut controller = menu::Controller::new(menu::Menu::new(
        (0..20)
            .map(|i| menu::Item::new(i.to_string(), label("row")))
            .collect(),
    ));
    controller.handle(menu::Command::Last);
    let layout = controller
        .menu
        .layout([100., 90.], Rect::new(0., 0., 120., 70.), ctx(), &Metrics)
        .unwrap();
    let regions = menu::Regions::retained(&layout, &controller.menu);
    let mut found = false;
    for y in layout.rect.y.ceil() as usize..layout.rect.bottom() as usize {
        let at = [layout.rect.x + 10., y as f32];
        assert_ne!(regions.hit(at), Some(0));
        if regions.hit(at) == Some(19) {
            found = true;
            assert_eq!(regions.cursor(at), Cursor::Pointer);
        }
    }
    assert!(
        found,
        "last keyboard selection is visible inside the clipped viewport"
    );
}
fn controls() -> Vec<dialog::Control> {
    use dialog::{Control, ControlKind::*};
    let mut blocked = Control::new("blocked", Action);
    blocked.enabled = false;
    vec![
        Control::new("purpose", Field),
        blocked,
        Control::new("cancel", Action),
        Control::new("approve", Action),
    ]
}
#[test]
fn dialog_focus_is_trapped_and_submit_respects_ime_repeat_and_validation() {
    let mut dialog = dialog::Controller::new(controls(), Some("approve".into()));
    assert_eq!(dialog.focused(), Some("purpose"));
    dialog.handle(dialog::Command::Next { backwards: true });
    assert_eq!(dialog.focused(), Some("approve"));
    dialog.handle(dialog::Command::Next { backwards: false });
    assert_eq!(dialog.focused(), Some("purpose"));
    assert_eq!(
        dialog.handle(dialog::Command::Focus("blocked".into())),
        Outcome::Local
    );
    for (composing, repeat) in [(true, false), (false, true), (true, true)] {
        assert_eq!(
            dialog.handle(dialog::Command::Submit { composing, repeat }),
            Outcome::Local
        );
    }
    assert_eq!(
        dialog.handle(dialog::Command::Submit {
            composing: false,
            repeat: false
        }),
        Outcome::Invoke("approve".into())
    );
    dialog
        .controls
        .iter_mut()
        .find(|c| c.id == "approve")
        .unwrap()
        .enabled = false;
    assert_eq!(
        dialog.handle(dialog::Command::Submit {
            composing: false,
            repeat: false
        }),
        Outcome::Local
    );
    dialog.handle(dialog::Command::Focus("cancel".into()));
    assert_eq!(
        dialog.handle(dialog::Command::Submit {
            composing: false,
            repeat: false
        }),
        Outcome::Invoke("cancel".into())
    );
    assert_eq!(
        dialog.handle(dialog::Command::Cancel { composing: true }),
        Outcome::Local
    );
    assert_eq!(
        dialog.handle(dialog::Command::Cancel { composing: false }),
        Outcome::Dismiss
    );
    assert_eq!(
        dialog.handle(dialog::Command::Activate("cancel".into())),
        Outcome::Local
    );
}
#[test]
fn arbitrary_nested_components_keep_field_and_approval_hit_regions() {
    let editor = ui::text_edit::Editor::new("render crew");
    let input = ui::widgets::Input {
        value: "render crew".into(),
        placeholder: "Purpose".into(),
        state: ui::widgets::InputState::Focus,
        icon: None,
        error: None,
        edit: Some(editor.edit()),
    }
    .element(ctx());
    let button = ui::widgets::Button {
        label: "Rename".into(),
        kind: ui::widgets::ButtonKind::Primary,
        state: ui::widgets::ButtonState::Idle,
        icon: None,
        key: Some("↵".into()),
    }
    .element(ctx())
    .unwrap();
    let body = c::Element::Stack(c::Stack {
        direction: Axis::Column,
        gap: ui::Length::Spacing(3),
        align: c::Alignment::Stretch,
        children: vec![
            label("Any toolkit content").into(),
            named("purpose", input).into(),
            c::Element::Icon {
                name: "gear".into(),
                size: ui::Length::Px(12.),
                paint: ui::Paint::solid(ui::Color::Muted),
            }
            .into(),
        ],
    });
    let panel = overlays::Modal::new("Rename swarm", body, vec![named("approve", button)])
        .element(ctx())
        .layout(Rect::new(20., 30., 380., 0.), ctx(), &Metrics)
        .unwrap();
    let mut dialog = dialog::Controller::new(controls(), Some("approve".into()));
    let regions = dialog::Regions::retained(&panel, &dialog.controls);
    let mut field = None;
    let mut action = None;
    for y in panel.rect.y.ceil() as usize..panel.rect.bottom() as usize {
        for x in panel.rect.x.ceil() as usize..panel.rect.right() as usize {
            let point = [x as f32, y as f32];
            match regions.hit(point) {
                Some("purpose") => field = Some(point),
                Some("approve") => action = Some(point),
                _ => {}
            }
        }
    }
    let field = field.unwrap();
    let action = action.unwrap();
    assert_eq!(regions.cursor(field), Cursor::Text);
    assert_eq!(regions.cursor(action), Cursor::Pointer);
    assert_eq!(
        dialog.click(action, &regions),
        Outcome::Invoke("approve".into())
    );
    assert_eq!(dialog.click([0., 0.], &regions), Outcome::Dismiss);
}
