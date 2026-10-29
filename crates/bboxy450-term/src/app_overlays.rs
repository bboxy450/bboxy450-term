// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! App-owned floating surfaces. The toolkit owns geometry/focus; this host owns
//! field values, validation and effects bound to the target captured on opening.
use crate::*;
use bbt_ui::{
    self as ui, components as c,
    overlays::{self, Outcome, context_menu as menu, dialog},
    text_edit::Editor,
};

const FIELD: &str = "app-dialog:input";
const SUBMIT: &str = "app-dialog:submit";
const CANCEL: &str = "app-dialog:cancel";
#[derive(Clone, Copy)]
pub(super) enum Target {
    Swarm(bbt_proto::SwarmId),
    Seat(SeatId),
}
/// Any existing toolkit components can be supplied as the modal body/actions.
/// Editors and effects remain outside this reusable presentation/focus shell.
pub(super) struct Dialog {
    pub modal: overlays::Modal,
    pub controller: dialog::Controller,
    pub composing: bool,
    pub on_action: fn(&mut App, &str),
    pub on_edit: Option<fn(&mut App, EditCommand)>,
}
pub(super) struct Rename {
    swarm: bbt_proto::SwarmId,
    value: String,
    editor: Editor,
    pending: Option<bbt_mux::Ulid>,
    error: Option<String>,
}
#[derive(Clone)]
struct SeatTarget {
    id: bbt_mux::Ulid,
    generation: u64,
    name: String,
    harness: String,
    scope: Result<bbt_proto::gui::SwarmScope, String>,
}
enum SeatFormKind {
    Roles,
    Tasks { next: Option<bbt_mux::Ulid> },
    Kill,
    Restart,
}
pub(super) struct SeatForm {
    target: SeatTarget,
    kind: SeatFormKind,
    choices: Vec<(String, String)>,
    pending: Option<bbt_mux::Ulid>,
    reading: bool,
    error: Option<String>,
}
pub(super) enum Surface {
    Menu {
        target: Target,
        anchor: [f32; 2],
        controller: menu::Controller,
    },
    Dialog {
        dialog: Box<Dialog>,
        rename: Option<Rename>,
        seat: Option<SeatForm>,
    },
}
pub(super) enum Regions {
    Menu(menu::Regions),
    Dialog(dialog::Regions),
}
pub(super) struct Layout {
    pub solved: c::Layout,
    regions: Regions,
}
#[derive(Default)]
pub(super) struct Host {
    pub(super) surface: Option<Surface>,
    pub layout: Option<Layout>,
    parent: Option<TextFocus>,
    seat: Option<SeatTarget>,
}
impl Host {
    pub fn open(&self) -> bool {
        self.surface.is_some()
    }
    pub fn focused_field(&self) -> Option<&str> {
        let Some(Surface::Dialog { dialog, .. }) = &self.surface else {
            return None;
        };
        dialog.controller.focused().filter(|id| {
            dialog
                .controller
                .controls
                .iter()
                .any(|c| c.id == *id && c.kind == dialog::ControlKind::Field)
        })
    }
    pub fn cursor(&self, at: [f32; 2]) -> ui::cursor::Cursor {
        self.layout
            .as_ref()
            .map_or(ui::cursor::Cursor::Default, |l| match &l.regions {
                Regions::Menu(r) => r.cursor(at),
                Regions::Dialog(r) => r.cursor(at),
            })
    }
    pub fn tree(&self) -> Option<&c::Layout> {
        self.layout.as_ref().map(|l| &l.solved)
    }
    pub fn regions(&self) -> Vec<(String, Rect)> {
        let Some(layout) = &self.layout else {
            return vec![];
        };
        let mut result = vec![("app-overlay".into(), layout.solved.rect)];
        match &self.surface {
            Some(Surface::Menu { controller, .. }) => {
                for (i, item) in controller.menu.items.iter().enumerate() {
                    if let Some(node) = named(&layout.solved, &format!("context-menu/item/{i}")) {
                        result.push((format!("context-menu:{}", item.id), node.rect));
                    }
                }
            }
            Some(Surface::Dialog { dialog, .. }) => {
                for control in &dialog.controller.controls {
                    if let Some(node) = named(&layout.solved, &control.id) {
                        result.push((control.id.clone(), node.rect));
                    }
                }
            }
            None => {}
        }
        result
    }
}
fn named<'a>(layout: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&layout.kind,c::LayoutKind::Named(name) if name==key) {
        return Some(layout);
    }
    layout.children.iter().find_map(|child| named(child, key))
}
fn text(value: impl Into<String>) -> c::Element {
    c::Element::Text(c::Text::new(value, ui::Text::default()))
}
fn control(id: &str, content: c::Element) -> c::Element {
    c::Element::Named {
        name: id.into(),
        content: Box::new(content),
    }
}
fn rename_dialog(rename: &Rename, focused: Option<&str>, ctx: ui::Context) -> overlays::Modal {
    let field = ui::widgets::Input {
        value: rename.value.clone(),
        placeholder: "What is this swarm for?".into(),
        state: if focused == Some(FIELD) {
            ui::widgets::InputState::Focus
        } else {
            ui::widgets::InputState::Value
        },
        icon: None,
        error: None,
        edit: (focused == Some(FIELD)).then(|| rename.editor.edit()),
    }
    .element(ctx);
    let mut children = vec![
        text("Give this swarm a purpose · up to 64 characters").into(),
        control(FIELD, field).into(),
    ];
    if let Some(error) = &rename.error {
        children.push(
            c::Element::Text(c::Text::new(
                error.clone(),
                ui::Text {
                    step: ui::TypeStep::Caption,
                    paint: ui::Paint::solid(ui::Color::Rgba(
                        bbt_ui_model::draw::Color::hex(bbt_ui_model::TOKENS.color.state.errored).0,
                    )),
                    ..Default::default()
                },
            ))
            .into(),
        );
    }
    let body = c::Element::Stack(c::Stack {
        direction: bbt_ui_model::layout::Axis::Column,
        gap: ui::Length::Spacing(3),
        align: c::Alignment::Stretch,
        children,
    });
    let button = |label: &str, kind, state, key: Option<&str>| {
        ui::widgets::Button {
            label: label.into(),
            kind,
            state,
            icon: None,
            key: key.map(Into::into),
        }
        .element(ctx)
        .expect("built-in dialog button style")
    };
    let mut modal = overlays::Modal::new(
        "Rename swarm",
        body,
        vec![
            control(
                CANCEL,
                button(
                    "Cancel",
                    ui::widgets::ButtonKind::Ghost,
                    ui::widgets::ButtonState::Idle,
                    Some("esc"),
                ),
            ),
            control(
                SUBMIT,
                button(
                    if rename.pending.is_some() {
                        "Saving…"
                    } else {
                        "Rename"
                    },
                    ui::widgets::ButtonKind::Primary,
                    if rename.pending.is_some() {
                        ui::widgets::ButtonState::Disabled
                    } else if focused == Some(SUBMIT) {
                        ui::widgets::ButtonState::Hover
                    } else {
                        ui::widgets::ButtonState::Idle
                    },
                    Some("↵"),
                ),
            ),
        ],
    );
    modal.style.overrides.width = Some(ui::Length::Px(480.));
    modal
}
fn seat_dialog(form: &SeatForm, ctx: ui::Context) -> (overlays::Modal, Vec<dialog::Control>) {
    let (title, explanation) = match form.kind {
        SeatFormKind::Roles => (
            "Change role",
            "Authority changes immediately. Session instructions follow the selected role policy.",
        ),
        SeatFormKind::Tasks { .. } => (
            "Assign task",
            "Open tasks from this seat’s swarm working folder",
        ),
        SeatFormKind::Kill => (
            "Kill seat process?",
            "Stops this seat’s current process. Its terminal and history remain available. This cannot be undone.",
        ),
        SeatFormKind::Restart => (
            "Restart seat?",
            "Stops this seat’s current process and starts its next generation. Supported harnesses resume the same session.",
        ),
    };
    let button = |id: &str, label: &str, enabled: bool, primary: bool| {
        control(
            id,
            ui::widgets::Button {
                label: label.into(),
                kind: if primary {
                    ui::widgets::ButtonKind::Primary
                } else {
                    ui::widgets::ButtonKind::Ghost
                },
                state: if enabled {
                    ui::widgets::ButtonState::Idle
                } else {
                    ui::widgets::ButtonState::Disabled
                },
                icon: None,
                key: None,
            }
            .element(ctx)
            .expect("built-in seat action style"),
        )
    };
    let mut controls = vec![dialog::Control::new(CANCEL, dialog::ControlKind::Action)];
    let mut body = vec![
        text(format!("{} · {}", form.target.name, form.target.harness)).into(),
        text(explanation).into(),
    ];
    for (index, (_, label)) in form.choices.iter().enumerate() {
        let id = format!("seat-choice:{index}");
        let enabled = form.pending.is_none();
        body.push(button(&id, label, enabled, false).into());
        let mut action = dialog::Control::new(id, dialog::ControlKind::Action);
        action.enabled = enabled;
        controls.push(action);
    }
    if form.reading {
        body.push(text("Loading tasks…").into());
    }
    if let Some(error) = &form.error {
        body.push(text(error).into());
    }
    if matches!(form.kind, SeatFormKind::Tasks { .. })
        && form.choices.is_empty()
        && !form.reading
        && form.error.is_none()
    {
        body.push(text("No open tasks in this working folder").into());
    }
    let mut actions = vec![button(CANCEL, "Cancel", true, false)];
    if let SeatFormKind::Tasks { next: Some(_) } = form.kind {
        let mut next = dialog::Control::new("seat-next", dialog::ControlKind::Action);
        next.enabled = form.pending.is_none();
        actions.push(button("seat-next", "Next tasks", next.enabled, false));
        controls.push(next);
    }
    if matches!(form.kind, SeatFormKind::Kill | SeatFormKind::Restart) {
        let mut submit = dialog::Control::new(SUBMIT, dialog::ControlKind::Action);
        submit.enabled = form.pending.is_none();
        actions.push(button(
            SUBMIT,
            if form.pending.is_some() {
                "Stopping…"
            } else if matches!(form.kind, SeatFormKind::Kill) {
                "Kill process"
            } else {
                "Restart"
            },
            submit.enabled,
            true,
        ));
        controls.push(submit);
    }
    let body = c::Element::Stack(c::Stack {
        direction: bbt_ui_model::layout::Axis::Column,
        gap: ui::Length::Spacing(3),
        align: c::Alignment::Stretch,
        children: body,
    });
    let mut modal = overlays::Modal::new(title, body, actions);
    modal.style.overrides.width = Some(ui::Length::Px(520.));
    (modal, controls)
}
impl App {
    fn open_seat_dialog(&mut self, action: &str) {
        let Some(target) = self.app_overlays.seat.clone() else {
            return;
        };
        let kind = match action {
            "role" => SeatFormKind::Roles,
            "assign-task" => SeatFormKind::Tasks { next: None },
            "kill" => SeatFormKind::Kill,
            "restart" => SeatFormKind::Restart,
            _ => return,
        };
        let mut form = SeatForm {
            target,
            kind,
            choices: vec![],
            pending: None,
            reading: false,
            error: None,
        };
        if matches!(form.kind, SeatFormKind::Roles) {
            let catalog = self.preferences.preferences.role_catalog();
            if let Some(policy) = catalog.harnesses.get(&form.target.harness) {
                form.choices = policy
                    .available_roles
                    .iter()
                    .filter(|name| catalog.roles.contains_key(*name))
                    .map(|name| (name.clone(), name.clone()))
                    .collect();
            }
            if !self.role_settings.applied(&catalog) {
                form.error = Some("Wait for connected role settings before changing a role".into());
            }
        }
        let (modal, controls) = seat_dialog(&form, self.scene.ui_context());
        self.show_app_dialog(Dialog {
            modal,
            controller: dialog::Controller::new(controls, None),
            composing: false,
            on_action: App::seat_dialog_action,
            on_edit: None,
        });
        if let Some(Surface::Dialog { seat, .. }) = &mut self.app_overlays.surface {
            *seat = Some(form);
        }
        if action == "assign-task" {
            self.read_seat_tasks(None);
        }
    }
    fn read_seat_tasks(&mut self, after: Option<bbt_mux::Ulid>) {
        let Some(Surface::Dialog {
            seat: Some(form), ..
        }) = &mut self.app_overlays.surface
        else {
            return;
        };
        let scope = match form.target.scope.clone() {
            Ok(scope) => scope,
            Err(reason) => {
                form.error = Some(reason);
                return;
            }
        };
        if form.pending.is_some() {
            return;
        }
        let id = bbt_mux::Ulid::new();
        form.pending = Some(id);
        form.reading = true;
        form.error = None;
        form.choices.clear();
        self.operation_backlog.push_back(live::UiMsg::Task {
            id,
            scope: Some(scope),
            request: bbt_proto::messaging::TaskRequest::List {
                unassigned: true,
                swarm_id: None,
                status: Some("open".into()),
                assignee: None,
                after,
                limit: Some(8),
            },
        });
        self.flush_operations();
    }
    fn seat_dialog_action(&mut self, action: &str) {
        if action == CANCEL {
            self.close_app_overlay();
            return;
        }
        let Some(Surface::Dialog {
            seat: Some(form), ..
        }) = &mut self.app_overlays.surface
        else {
            return;
        };
        if form.pending.is_some() {
            return;
        }
        if matches!(form.kind, SeatFormKind::Roles)
            && !self
                .role_settings
                .applied(&self.preferences.preferences.role_catalog())
        {
            form.error = Some("Wait for connected role settings before changing a role".into());
            return;
        }
        if action == "seat-next" {
            if let SeatFormKind::Tasks { next } = form.kind {
                let next = next;
                self.read_seat_tasks(next);
            }
            return;
        }
        let selected = action
            .strip_prefix("seat-choice:")
            .and_then(|index| index.parse::<usize>().ok())
            .and_then(|index| form.choices.get(index))
            .map(|(value, _)| value.clone());
        let command = match (&form.kind, selected, action) {
            (SeatFormKind::Roles, Some(name), _) => bbt_proto::gui::SeatControl::Role { name },
            (SeatFormKind::Tasks { .. }, Some(task_path), _) => match form.target.scope.clone() {
                Ok(scope) => bbt_proto::gui::SeatControl::AssignTask { scope, task_path },
                Err(reason) => {
                    form.error = Some(reason);
                    return;
                }
            },
            (SeatFormKind::Kill, _, SUBMIT) => bbt_proto::gui::SeatControl::Kill,
            (SeatFormKind::Restart, _, SUBMIT) => bbt_proto::gui::SeatControl::Restart,
            _ => return,
        };
        if self.backend_lost || self.closing || self.live_commands.is_none() {
            form.error = Some("Backend unavailable · reconnect before retrying".into());
            return;
        }
        let id = bbt_mux::Ulid::new();
        let target = form.target.clone();
        form.pending = Some(id);
        form.error = None;
        self.operation_backlog.push_back(live::UiMsg::SeatControl {
            id,
            seat: target.id,
            generation: target.generation,
            command,
        });
        self.flush_operations();
    }
    fn sync_seat_dialog(&mut self) {
        use bboxy450_term::messaging::Reply;
        let Some(Surface::Dialog {
            seat: Some(form), ..
        }) = &mut self.app_overlays.surface
        else {
            return;
        };
        let Some(model) = self.live_state.as_ref() else {
            return;
        };
        // A response from a former working folder is never installed in a picker.
        if matches!(form.kind, SeatFormKind::Tasks { .. })
            && let Ok(scope) = form.target.scope
            && model
                .swarm_scope(bbt_proto::SwarmId::from_u128(scope.swarm.0))
                .ok()
                != Some(scope)
        {
            form.choices.clear();
            form.error = Some("Swarm working folder changed · reopen this menu".into());
            return;
        }
        let Some(op) = form.pending else {
            return;
        };
        let Some(result) = model.operation_results.rows().find(|row| row.op == op) else {
            return;
        };
        match &result.reply {
            Reply::Task(bbt_proto::messaging::TaskResponse::Items { items, next })
                if form.reading =>
            {
                form.choices = items
                    .iter()
                    .map(|item| (item.path.clone(), item.title.clone()))
                    .collect();
                form.kind = SeatFormKind::Tasks { next: *next };
                form.pending = None;
                form.reading = false;
            }
            Reply::SeatControlled if !form.reading => {
                self.close_app_overlay();
            }
            Reply::Refused(reason) => {
                form.pending = None;
                form.reading = false;
                form.error = Some(reason.clone());
            }
            Reply::Unknown(reason) => {
                form.reading = false;
                form.error = Some(format!("Outcome unknown · {reason}"));
            }
            _ => {
                form.pending = None;
                form.reading = false;
                form.error = Some("Unexpected seat action response".into());
            }
        }
    }
    pub(super) fn overlay_right_click(&mut self, at: [f32; 2]) {
        self.pointer = at;
        if self.app_overlays.open() || self.document.open || self.launcher.is_some() {
            return;
        }
        let hit =
            |r: &Rect| at[0] >= r.x && at[0] < r.right() && at[1] >= r.y && at[1] < r.bottom();
        let target = self
            .navigation_regions
            .iter()
            .filter(|(_, r)| hit(r))
            .find_map(|(key, _)| {
                key.strip_prefix("seat:")
                    .or_else(|| key.strip_prefix("mini:"))
                    .and_then(|id| id.parse().ok())
                    .map(Target::Seat)
                    .or_else(|| {
                        key.strip_prefix("swarm:")
                            .or_else(|| key.strip_prefix("toggle-swarm:"))
                            .and_then(|id| id.parse().ok())
                            .map(Target::Swarm)
                    })
            })
            .or_else(|| {
                if self.local_tab.is_some() {
                    return None;
                }
                self.focus_bounds
                    .iter()
                    .find(|(_, r)| hit(r))
                    .map(|(id, _)| Target::Seat(*id))
            });
        if let Some(target) = target {
            self.open_context(target, at);
        }
    }
    pub(super) fn open_context(&mut self, target: Target, anchor: [f32; 2]) {
        self.app_overlays.seat = match target {
            Target::Seat(id) => self.live_state.as_ref().and_then(|model| {
                model
                    .gui_seats
                    .iter()
                    .find(|row| row.record.id.0 == id.0.0)
                    .map(|row| SeatTarget {
                        id: row.record.id,
                        generation: row.record.generation,
                        name: row.record.name.clone(),
                        harness: row.record.harness_kind.clone(),
                        scope: model
                            .swarm_scope(bbt_proto::SwarmId::from_u128(row.record.swarm.0))
                            .map_err(str::to_owned),
                    })
            }),
            Target::Swarm(_) => None,
        };
        let items = match target {
            Target::Swarm(id) => vec![
                ("rename", "Rename swarm"),
                ("cwd", "Choose cwd…"),
                ("new-seat", "New seat"),
                (
                    "collapse",
                    if self.scene.open_swarms.contains(&id) {
                        "Collapse"
                    } else {
                        "Expand"
                    },
                ),
            ],
            Target::Seat(id) => {
                let muted = self
                    .live_state
                    .as_ref()
                    .and_then(|m| m.ui.seats.get(&id))
                    .is_some_and(|s| s.muted);
                vec![
                    ("focus", "Focus seat"),
                    ("mute", if muted { "Unmute" } else { "Mute" }),
                    ("role", "Change role…"),
                    ("assign-task", "Assign task…"),
                    ("restart", "Restart seat…"),
                    ("kill", "Kill process…"),
                    ("close", "Close seat"),
                ]
            }
        };
        self.suspend_overlay_focus();
        self.app_overlays.surface = Some(Surface::Menu {
            target,
            anchor,
            controller: menu::Controller::new(menu::Menu::new(
                items
                    .into_iter()
                    .map(|(id, label)| menu::Item::new(id, text(label)))
                    .collect(),
            )),
        });
        self.app_overlays.layout = None;
        if let Some(window) = &self.window {
            window.set_ime_allowed(false);
        }
        self.request_redraw();
    }
    fn suspend_overlay_focus(&mut self) {
        self.app_overlays.parent = if self.reject_focus {
            self.reject.as_ref().map(|(id, _)| TextFocus::Reject(*id))
        } else {
            self.composer
                .view
                .focus
                .then_some(TextFocus::Composer(self.composer.view.field))
        };
        self.blur_reject();
        self.composer.handle(ComposeCommand::Blur);
        self.query_dragging = false;
        self.composer_dragging = false;
        self.end_sidebar_resize();
    }
    pub(super) fn show_app_dialog(&mut self, dialog: Dialog) {
        if !self.app_overlays.open() {
            self.suspend_overlay_focus();
        }
        self.app_overlays.surface = Some(Surface::Dialog {
            dialog: Box::new(dialog),
            rename: None,
            seat: None,
        });
        self.app_overlays.layout = None;
        if let Some(window) = &self.window {
            window.set_ime_allowed(true)
        }
        self.request_redraw();
    }
    pub(super) fn close_app_overlay(&mut self) {
        // End the native marked-text session before restoring a different field.
        if let Some(window) = &self.window {
            window.set_ime_allowed(false);
        }
        self.app_overlays.surface = None;
        self.app_overlays.layout = None;
        match self.app_overlays.parent.take() {
            Some(TextFocus::Composer(field)) => {
                self.composer.handle(ComposeCommand::Focus(field));
            }
            Some(TextFocus::Reject(id)) => {
                if let Some((actual, reason)) = &mut self.reject
                    && *actual == id
                {
                    reason.focus(true);
                    self.reject_focus = true;
                }
            }
            None => {}
        }
        if let Some(window) = &self.window {
            window.set_ime_allowed(
                self.composer.view.focus
                    || self.reject_focus
                    || self.settings_ime()
                    || self.launcher.is_some(),
            )
        }
        self.request_redraw();
    }
    pub(super) fn overlay_outcome(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Dismiss => self.close_app_overlay(),
            Outcome::Invoke(action) => match self.app_overlays.surface.as_ref() {
                Some(Surface::Menu { target, .. }) => {
                    let target = *target;
                    if matches!(action.as_str(), "role" | "assign-task" | "kill" | "restart")
                        && matches!(target, Target::Seat(_))
                    {
                        self.open_seat_dialog(&action);
                    } else if action == "rename"
                        && let Target::Swarm(swarm) = target
                    {
                        let value = self
                            .live_state
                            .as_ref()
                            .and_then(|m| m.ui.window.sidebar.swarms.iter().find(|s| s.id == swarm))
                            .map_or_else(String::new, |s| s.name.clone());
                        let mut editor = Editor::new(&value);
                        let mut selected = value.clone();
                        editor.handle(&mut selected, EditCommand::SelectAll);
                        let rename = Rename {
                            swarm,
                            value,
                            editor,
                            pending: None,
                            error: None,
                        };
                        let controller = dialog::Controller::new(
                            vec![
                                dialog::Control::new(FIELD, dialog::ControlKind::Field),
                                dialog::Control::new(CANCEL, dialog::ControlKind::Action),
                                dialog::Control::new(SUBMIT, dialog::ControlKind::Action),
                            ],
                            Some(SUBMIT.into()),
                        );
                        let modal =
                            rename_dialog(&rename, controller.focused(), self.scene.ui_context());
                        self.show_app_dialog(Dialog {
                            modal,
                            controller,
                            composing: false,
                            on_action: App::rename_dialog_action,
                            on_edit: Some(App::rename_dialog_edit),
                        });
                        if let Some(Surface::Dialog { rename: form, .. }) =
                            &mut self.app_overlays.surface
                        {
                            *form = Some(rename);
                        }
                        if let Some(window) = &self.window {
                            window.set_ime_allowed(true)
                        }
                    } else {
                        self.close_app_overlay();
                        match (target, action.as_str()) {
                            (Target::Swarm(id), "new-seat") => {
                                self.navigate_to(&format!("new-seat:{id}"))
                            }
                            (Target::Swarm(id), "collapse") => {
                                self.toggle_sidebar_section(&format!("swarm:{id}"))
                            }
                            (Target::Swarm(id), "cwd") => {
                                let directory = self
                                    .live_state
                                    .as_ref()
                                    .and_then(|m| {
                                        m.ui.window.sidebar.swarms.iter().find(|s| s.id == id)
                                    })
                                    .and_then(|s| s.workdir.as_deref());
                                if !self.swarm_folder_pending {
                                    match swarm_folder::choose(id, self.proxy.clone(), directory) {
                                        Ok(()) => self.swarm_folder_pending = true,
                                        Err(reason) => self.logger.write(reason),
                                    }
                                }
                            }
                            (Target::Seat(id), "focus") => {
                                self.focus_action(FocusAction::Activate(id, FocusOrigin::Row))
                            }
                            (Target::Seat(id), "mute") => self.mute_seat(id),
                            (Target::Seat(id), "close") => {
                                self.close_seat(id, bbt_mux::Ulid::new(), false)
                            }
                            _ => {}
                        }
                    }
                }
                Some(Surface::Dialog { dialog, .. }) => {
                    let invoke = dialog.on_action;
                    invoke(self, &action);
                }
                _ => {}
            },
            Outcome::Local | Outcome::Changed => {}
        }
        self.app_overlays.layout = None;
        self.request_redraw();
    }
    fn rename_dialog_action(&mut self, action: &str) {
        match action {
            CANCEL => self.close_app_overlay(),
            SUBMIT => self.submit_rename(),
            _ => {}
        }
    }
    fn submit_rename(&mut self) {
        let Some(Surface::Dialog {
            dialog,
            rename: Some(rename),
            ..
        }) = &mut self.app_overlays.surface
        else {
            return;
        };
        if rename.pending.is_some() {
            return;
        }
        if self.backend_lost || self.closing || self.live_commands.is_none() {
            rename.error = Some("Backend unavailable · try again when reconnected".into());
            return;
        }
        let value = rename.value.trim();
        if value.is_empty() || value.chars().count() > 64 || value.chars().any(char::is_control) {
            rename.error = Some("Use 1–64 printable characters".into());
            return;
        }
        let op = bbt_mux::Ulid::new();
        rename.pending = Some(op);
        rename.error = None;
        for c in &mut dialog.controller.controls {
            if c.id == FIELD || c.id == SUBMIT {
                c.enabled = false
            }
        }
        let message = live::UiMsg::SwarmRename {
            id: op,
            swarm: rename.swarm,
            name: value.into(),
        };
        self.operation_backlog.push_back(message);
        self.flush_operations();
    }
    pub(super) fn overlay_metadata_reply(&mut self, op: bbt_mux::Ulid, result: Result<(), String>) {
        let Some(Surface::Dialog {
            dialog,
            rename: Some(rename),
            ..
        }) = &mut self.app_overlays.surface
        else {
            return;
        };
        if rename.pending != Some(op) {
            return;
        }
        match result {
            Ok(()) => self.close_app_overlay(),
            Err(reason) => {
                rename.pending = None;
                rename.error = Some(reason);
                for c in &mut dialog.controller.controls {
                    c.enabled = true
                }
                dialog
                    .controller
                    .handle(dialog::Command::Focus(FIELD.into()));
                self.app_overlays.layout = None;
                self.request_redraw();
            }
        }
    }
    pub(super) fn overlay_backend_lost(&mut self) {
        if let Some(Surface::Dialog {
            rename: Some(rename),
            ..
        }) = &self.app_overlays.surface
            && let Some(op) = rename.pending
        {
            self.overlay_metadata_reply(
                op,
                Err(
                    "Connection lost · the rename outcome is unknown; reconnect before retrying"
                        .into(),
                ),
            );
        }
    }
    pub(super) fn overlay_edit(&mut self, command: EditCommand) {
        if let Some(Surface::Dialog { dialog, .. }) = &self.app_overlays.surface
            && let Some(edit) = dialog.on_edit
        {
            edit(self, command);
        }
    }
    fn rename_dialog_edit(&mut self, command: EditCommand) {
        if let Some(Surface::Dialog {
            dialog,
            rename: Some(rename),
            ..
        }) = &mut self.app_overlays.surface
            && (dialog.controller.focused() == Some(FIELD)
                || matches!(command, EditCommand::CancelPreedit))
            && rename.pending.is_none()
        {
            rename.editor.handle(&mut rename.value, command);
            dialog.composing = rename.editor.composing();
            rename.error = None;
            self.app_overlays.layout = None;
            self.request_redraw();
        }
    }
    pub(super) fn overlay_pointer(&mut self, at: [f32; 2]) -> bool {
        if !self.app_overlays.open() {
            return false;
        }
        let outcome = match (&mut self.app_overlays.surface, &self.app_overlays.layout) {
            (
                Some(Surface::Menu { controller, .. }),
                Some(Layout {
                    regions: Regions::Menu(regions),
                    ..
                }),
            ) => controller.click(at, regions),
            (
                Some(Surface::Dialog { dialog, .. }),
                Some(Layout {
                    regions: Regions::Dialog(regions),
                    ..
                }),
            ) => dialog.controller.click(at, regions),
            _ => Outcome::Local,
        };
        self.overlay_edit(EditCommand::CancelPreedit);
        self.overlay_outcome(outcome);
        true
    }
    pub(super) fn overlay_hover(&mut self, at: [f32; 2]) -> bool {
        if !self.app_overlays.open() {
            return false;
        }
        if let (
            Some(Surface::Menu { controller, .. }),
            Some(Layout {
                regions: Regions::Menu(regions),
                ..
            }),
        ) = (&mut self.app_overlays.surface, &self.app_overlays.layout)
            && controller.hover(at, regions) == Outcome::Changed
        {
            self.app_overlays.layout = None;
            self.request_redraw()
        }
        true
    }
    pub(super) fn solve_app_overlay(&mut self, size: [f32; 2]) -> Result<(), String> {
        self.sync_seat_dialog();
        let ctx = self.scene.ui_context();
        self.app_overlays.layout = match &mut self.app_overlays.surface {
            Some(Surface::Menu {
                anchor, controller, ..
            }) => {
                let solved = self
                    .scene
                    .context_menu_layout(&controller.menu, *anchor, size)?;
                let regions = Regions::Menu(menu::Regions::retained(&solved, &controller.menu));
                Some(Layout { solved, regions })
            }
            Some(Surface::Dialog {
                dialog,
                rename,
                seat,
            }) => {
                if let Some(rename) = rename {
                    dialog.modal = rename_dialog(rename, dialog.controller.focused(), ctx);
                }
                if let Some(form) = seat {
                    let (modal, controls) = seat_dialog(form, ctx);
                    dialog.modal = modal;
                    let focused = dialog.controller.focused().map(str::to_owned);
                    dialog.controller = dialog::Controller::new(controls, None);
                    if let Some(focused) = focused {
                        dialog.controller.handle(dialog::Command::Focus(focused));
                    }
                }
                let solved = self.scene.app_dialog_layout(dialog.modal.clone(), size)?;
                let regions = Regions::Dialog(dialog::Regions::retained(
                    &solved,
                    &dialog.controller.controls,
                ));
                Some(Layout { solved, regions })
            }
            None => None,
        };
        if let Some(window) = &self.window
            && self.app_overlays.open()
        {
            window.set_ime_allowed(self.app_overlays.focused_field().is_some());
        }
        if let Some(window) = &self.window
            && let Some(layout) = &self.app_overlays.layout
            && let Some(field) = self
                .app_overlays
                .focused_field()
                .and_then(|key| named(&layout.solved, key))
        {
            window.set_ime_cursor_area(
                winit::dpi::LogicalPosition::new(field.rect.x, field.rect.bottom()),
                winit::dpi::LogicalSize::new(1., field.rect.h),
            );
        }
        Ok(())
    }
    pub(super) fn paint_app_overlay(&self) -> Vec<c::Paint> {
        let Some(layout) = &self.app_overlays.layout else {
            return vec![];
        };
        let mut ops = if matches!(self.app_overlays.surface, Some(Surface::Dialog { .. })) {
            self.scene.app_dialog_backdrop(self.solved_size)
        } else {
            vec![]
        };
        ops.extend(c::paint(&layout.solved, self.scene.ui_context()));
        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[allow(
        clippy::disallowed_methods,
        reason = "isolated process-wide UI registration and private home"
    )]
    fn arbitrary_dialog_traps_keys_and_resolves_only_its_control_cursors() {
        const CHILD: &str = "BBOXY_APP_OVERLAY_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let home = tempfile::tempdir().unwrap();
            let output=std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact","app_overlays::tests::arbitrary_dialog_traps_keys_and_resolves_only_its_control_cursors","--nocapture","--test-threads=1"])
                .env(CHILD,"1").env("HOME",home.path()).env("ZDOTDIR",home.path()).output().unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        struct QuietWake;
        impl bbt_rt::Waker for QuietWake {
            fn wake(&self) {}
        }
        let inbox = bbt_rt::init_ui(QuietWake);
        let updates = Arc::new(Latest::new(inbox.dirty.paint_source(5)));
        let mut app = App::owner(
            UiProxy::Headless(Arc::new(|_| {})),
            inbox,
            log::Logger::new(),
            updates,
            &["--headless".into()],
            Verify::default(),
        );
        app.composer
            .handle(ComposeCommand::Focus(ComposeField::Body));
        let body = c::Element::Stack(c::Stack {
            direction: bbt_ui_model::layout::Axis::Column,
            gap: ui::Length::Spacing(3),
            align: c::Alignment::Stretch,
            children: vec![
                text("Custom arbitrary body").into(),
                control(
                    "custom-field",
                    ui::widgets::Input {
                        value: "editable outside the shell".into(),
                        placeholder: String::new(),
                        state: ui::widgets::InputState::Value,
                        icon: None,
                        error: None,
                        edit: None,
                    }
                    .element(app.scene.ui_context()),
                )
                .into(),
            ],
        });
        let button = ui::widgets::Button {
            label: "Approve".into(),
            kind: ui::widgets::ButtonKind::Primary,
            state: ui::widgets::ButtonState::Idle,
            icon: None,
            key: None,
        }
        .element(app.scene.ui_context())
        .unwrap();
        app.show_app_dialog(Dialog {
            modal: overlays::Modal::new(
                "Generic dialog",
                body,
                vec![control("custom-approve", button)],
            ),
            controller: dialog::Controller::new(
                vec![
                    dialog::Control::new("custom-field", dialog::ControlKind::Field),
                    dialog::Control::new("custom-approve", dialog::ControlKind::Action),
                ],
                Some("custom-approve".into()),
            ),
            composing: false,
            on_edit: None,
            on_action: |app, _| {
                app.unread_errors += 1;
                app.close_app_overlay();
            },
        });
        app.solve_app_overlay([800., 600.]).unwrap();
        for (key, rect) in app.app_overlays.regions() {
            let center = [rect.x + rect.w / 2., rect.y + rect.h / 2.];
            if key == "custom-field" {
                assert_eq!(app.cursor_at(center), ui::cursor::Cursor::Text)
            }
            if key == "custom-approve" {
                assert_eq!(app.cursor_at(center), ui::cursor::Cursor::Pointer)
            }
        }
        assert_eq!(app.cursor_at([0., 0.]), ui::cursor::Cursor::Default);
        assert!(app.overlay_key(&chrome_input("cmd-t").unwrap()));
        assert!(app.launcher.is_none());
        let mut enter = chrome_input("enter").unwrap();
        enter.repeat = true;
        assert!(app.overlay_key(&enter));
        assert_eq!(app.unread_errors, 0);
        if let Some(Surface::Dialog { dialog, .. }) = &mut app.app_overlays.surface {
            dialog.composing = true;
        }
        enter.repeat = false;
        app.overlay_key(&enter);
        assert_eq!(app.unread_errors, 0);
        if let Some(Surface::Dialog { dialog, .. }) = &mut app.app_overlays.surface {
            dialog.composing = false;
        }
        app.overlay_key(&enter);
        assert_eq!(app.unread_errors, 1);
        assert!(!app.app_overlays.open());
        assert!(app.composer.view.focus);

        app.open_context(Target::Seat(SeatId::from_u128(3)), [100., 100.]);
        let mut close = chrome_input("cmd-w").unwrap();
        close.repeat = true;
        assert!(app.overlay_key(&close));
        assert!(app.app_overlays.open());
        for modifiers in [
            ModifiersState::SUPER,
            ModifiersState::SUPER | ModifiersState::ALT,
        ] {
            let mut quit = chrome_input("q").unwrap();
            quit.modifiers.state = modifiers;
            assert!(!app.overlay_key(&quit));
            assert!(app.app_overlays.open());
        }
        close.repeat = false;
        assert!(app.overlay_key(&close));
        assert!(!app.app_overlays.open());
        assert_eq!(app.unread_errors, 1, "dismissal cannot invoke approval");
        assert!(app.composer.view.focus);

        let swarm = bbt_proto::SwarmId::from_u128(7);
        app.focus.stage = Some(Stage {
            swarm: Some(bbt_ui_model::SwarmId::from_u128(swarm.0.0)),
            tab: None,
            focused: Some(SeatId::from_u128(3)),
        });
        app.focus_bounds = vec![(SeatId::from_u128(3), Rect::new(300., 40., 500., 500.))];
        app.open_settings("Roles");
        for chord in ["menu", "shift-f10"] {
            assert!(!app.overlay_key(&chrome_input(chord).unwrap()));
            assert!(!app.app_overlays.open());
        }
        app.overlay_right_click([400., 100.]);
        assert!(!app.app_overlays.open(), "a page hides prior pane bounds");
        app.navigation_regions = vec![(format!("swarm:{swarm}"), Rect::new(0., 40., 200., 30.))];
        app.overlay_right_click([100., 50.]);
        assert!(
            matches!(
                app.app_overlays.surface,
                Some(Surface::Menu { target: Target::Swarm(id), .. }) if id == swarm
            ),
            "the visible sidebar keeps its context menu"
        );
    }
}
