// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Desktop orchestration for the shared answer form.
use crate::{
    App,
    app_overlays::{Dialog, Surface},
    live::UiMsg,
};
use bbt_mux::Ulid;
use bbt_proto::gui::{
    ApprovalOption, InboxQuestion, QuestionAnswerResult, QuestionAnswerer, QuestionContent,
    QuestionCursor,
};
use bbt_ui::{
    self as ui, components as c,
    modules::question_form::{self, Form, button, text},
    overlays::{
        Modal,
        dialog::{self, Control, ControlKind},
    },
    text_edit::Command as EditCommand,
};
use bbt_ui_model::layout::Axis;
use serde_json::Value;

const VISIBLE: usize = 6;
const CLOSE: &str = "inbox/close";
const BACK: &str = "inbox/back";
const PREVIOUS: &str = "inbox/previous";
const MORE: &str = "inbox/more";
const RETRY: &str = "inbox/retry";
const ITEM_PREFIX: &str = "inbox/item/";
type Page = (Vec<InboxQuestion>, Option<QuestionCursor>);

#[derive(Debug)]
pub enum Event {
    Page(Result<Page, String>),
    Changed,
    Answer(Ulid, Result<QuestionAnswerResult, String>),
}

struct Pending {
    target: QuestionCursor,
    op: Ulid,
    reply: Value,
    sending: bool,
}
impl Pending {
    fn message(&self) -> UiMsg {
        UiMsg::InboxAnswer {
            seat: self.target.seat,
            generation: self.target.generation,
            item: self.target.item,
            op_id: self.op,
            reply: self.reply.clone(),
        }
    }
}

#[derive(Default)]
pub(super) struct Inbox {
    items: Vec<InboxQuestion>,
    cursor: Option<QuestionCursor>,
    offset: usize,
    reading: Option<Option<QuestionCursor>>,
    refresh: bool,
    selection: Option<(QuestionCursor, Form)>,
    pending: Option<Pending>,
    status: Option<String>,
}

impl Inbox {
    pub(super) fn render(&self, dialog: &mut Dialog, ctx: ui::Context) {
        let focus = dialog.controller.focused().map(str::to_owned);
        let mut controls = Vec::new();
        let mut body = Vec::new();
        let mut default = None;
        if let Some((target, form)) = &self.selection
            && let Some(item) = self.items.iter().find(|item| item.cursor() == *target)
        {
            body.push(text(format!("{} · {}", item.seat_name, item.role)).into());
            body.push(text(&item.message).into());
            form.compose(
                ctx,
                focus.as_deref(),
                self.pending.is_none(),
                &mut body,
                &mut controls,
            );
            default = Some(question_form::SUBMIT);
            add_button(ctx, &mut body, &mut controls, BACK, "Back");
        } else {
            for item in self.items.iter().skip(self.offset).take(VISIBLE) {
                let label = format!("{} · {}", item.seat_name, item.message);
                let id = format!(
                    "{ITEM_PREFIX}{}/{}/{}",
                    item.seat, item.generation, item.item
                );
                add_button(ctx, &mut body, &mut controls, &id, &label);
            }
            if self.items.is_empty() {
                body.push(
                    text(if self.reading.is_some() {
                        "Loading inbox…"
                    } else {
                        "No open questions or approvals"
                    })
                    .into(),
                );
            }
            if self.offset > 0 {
                add_button(ctx, &mut body, &mut controls, PREVIOUS, "Previous");
            }
            if self.offset + VISIBLE < self.items.len() || self.cursor.is_some() {
                add_button(ctx, &mut body, &mut controls, MORE, "More");
            }
        }
        if let Some(status) = &self.status {
            body.push(text(status).into());
        }
        if self.pending.as_ref().is_some_and(|op| !op.sending) {
            add_button(ctx, &mut body, &mut controls, RETRY, "Retry answer");
        }
        add_button(ctx, &mut body, &mut controls, CLOSE, "Close");
        dialog.modal = modal(body);
        let mut controller = dialog::Controller::new(controls, default.map(str::to_owned));
        if let Some(focus) = focus {
            controller.handle(dialog::Command::Focus(focus));
        }
        dialog.controller = controller;
    }
}

fn modal(body: Vec<c::Item>) -> Modal {
    let mut modal = Modal::new(
        "Inbox",
        c::Element::Stack(c::Stack {
            direction: Axis::Column,
            gap: ui::Length::Spacing(3),
            align: c::Alignment::Stretch,
            children: body,
        }),
        vec![],
    );
    modal.style.overrides.width = Some(ui::Length::Px(520.));
    modal
}

fn add_button(
    ctx: ui::Context,
    body: &mut Vec<c::Item>,
    controls: &mut Vec<Control>,
    id: &str,
    label: &str,
) {
    body.push(button(ctx, id, label, true, false).into());
    controls.push(Control::new(id, ControlKind::Action));
}

fn form(item: &InboxQuestion) -> Form {
    match &item.content {
        QuestionContent::Question { schema } => Form::question(schema),
        QuestionContent::Approval { options } => Form::approval(
            options
                .iter()
                .map(|option| {
                    let label = match option {
                        ApprovalOption::Allow => "Allow once",
                        ApprovalOption::AllowAlways => "Always allow",
                        ApprovalOption::Deny => "Deny",
                    };
                    (
                        label.into(),
                        serde_json::to_value(option).expect("approval option"),
                    )
                })
                .collect(),
        ),
    }
}

impl App {
    pub(super) fn open_inbox(&mut self) {
        self.show_app_dialog(Dialog {
            modal: modal(vec![]),
            controller: dialog::Controller::new(vec![], None),
            composing: false,
            on_action: App::inbox_action,
            on_edit: Some(App::inbox_edit),
        });
        self.app_overlays.inbox = Some(Inbox::default());
        self.inbox_reload();
    }

    fn inbox_read(&mut self, after: Option<QuestionCursor>) {
        let Some(inbox) = self.app_overlays.inbox.as_mut() else {
            return;
        };
        if inbox.reading.is_some() {
            inbox.refresh |= after.is_none();
            return;
        }
        inbox.reading = Some(after);
        self.operation_backlog.push_back(UiMsg::InboxPage(after));
        self.flush_operations();
    }

    pub(super) fn inbox_reload(&mut self) {
        self.inbox_read(None);
    }

    pub(super) fn inbox_disconnected(&mut self) {
        if let Some(inbox) = self.app_overlays.inbox.as_mut() {
            inbox.reading = None;
            inbox.refresh = false;
            if let Some(pending) = inbox.pending.as_mut() {
                pending.sending = false;
                inbox.status = Some("Answer outcome unknown".into());
            }
            self.app_overlays.layout = None;
            self.request_redraw();
        }
    }

    pub(super) fn inbox_event(&mut self, event: Event) {
        let Some(inbox) = self.app_overlays.inbox.as_mut() else {
            return;
        };
        let mut reload = false;
        match event {
            Event::Changed => self.inbox_reload(),
            Event::Page(result) => {
                let Some(after) = inbox.reading.take() else {
                    return;
                };
                match result {
                    Ok((items, cursor)) => {
                        if after.is_none() {
                            inbox.items.clear();
                            inbox.offset = 0;
                        }
                        inbox.cursor = cursor;
                        inbox.items.extend(items);
                        if inbox.selection.as_ref().is_some_and(|(target, _)| {
                            !inbox.items.iter().any(|item| item.cursor() == *target)
                        }) {
                            inbox.selection = None;
                        }
                    }
                    Err(reason) => inbox.status = Some(reason),
                }
                reload = std::mem::take(&mut inbox.refresh);
            }
            Event::Answer(id, result) => {
                let Some(pending) = inbox.pending.as_mut().filter(|pending| pending.op == id)
                else {
                    return;
                };
                inbox.status = Some(match result {
                    Ok(QuestionAnswerResult::Applied) => "Answer sent".into(),
                    Ok(QuestionAnswerResult::AlreadyAnswered { by }) => {
                        let name = match by {
                            QuestionAnswerer::Client(by) => by.name,
                            QuestionAnswerer::Resolver { by, details } => details
                                .get("name")
                                .or_else(|| details.get("model"))
                                .and_then(Value::as_str)
                                .map(str::to_owned)
                                .unwrap_or(by),
                        };
                        format!("Already answered by {name}")
                    }
                    Ok(QuestionAnswerResult::StaleGeneration) => "This seat has restarted".into(),
                    Ok(QuestionAnswerResult::Refused { .. }) => "Answer permission refused".into(),
                    Err(reason) => {
                        pending.sending = false;
                        inbox.status = Some(reason);
                        self.app_overlays.layout = None;
                        self.request_redraw();
                        return;
                    }
                });
                let target = pending.target;
                inbox.items.retain(|item| item.cursor() != target);
                inbox.pending = None;
                inbox.selection = None;
                reload = true;
            }
        }
        if reload {
            self.inbox_reload();
        }
        self.app_overlays.layout = None;
        self.request_redraw();
    }

    fn inbox_action(&mut self, id: &str) {
        let Some(inbox) = self.app_overlays.inbox.as_mut() else {
            return;
        };
        match id {
            CLOSE => self.close_app_overlay(),
            BACK => inbox.selection = None,
            PREVIOUS => inbox.offset = inbox.offset.saturating_sub(VISIBLE),
            MORE => {
                if inbox.offset + VISIBLE < inbox.items.len() {
                    inbox.offset += VISIBLE;
                } else if let Some(cursor) = inbox.cursor {
                    inbox.offset = inbox.items.len();
                    self.inbox_read(Some(cursor));
                }
            }
            RETRY => {
                if let Some(pending) = inbox.pending.as_mut() {
                    pending.sending = true;
                    inbox.status = Some("Sending answer…".into());
                    self.operation_backlog.push_back(pending.message());
                    self.flush_operations();
                }
            }
            id if id.starts_with(ITEM_PREFIX) => {
                if let Some(item) = inbox.items.iter().find(|item| {
                    id == format!(
                        "{ITEM_PREFIX}{}/{}/{}",
                        item.seat, item.generation, item.item
                    )
                }) {
                    inbox.selection = Some((item.cursor(), form(item)));
                }
            }
            id if inbox.pending.is_none() => {
                if let Some((target, form)) = &mut inbox.selection
                    && let Some(reply) = form.act(id)
                {
                    match reply {
                        Ok(reply) => {
                            let pending = Pending {
                                target: *target,
                                op: Ulid::new(),
                                reply,
                                sending: true,
                            };
                            self.operation_backlog.push_back(pending.message());
                            inbox.pending = Some(pending);
                            inbox.status = Some("Sending answer…".into());
                            self.flush_operations();
                        }
                        Err(reason) => inbox.status = Some(reason.into()),
                    }
                }
            }
            _ => {}
        }
    }

    fn inbox_edit(&mut self, command: EditCommand) {
        if let Some(Surface::Dialog { dialog, .. }) = &mut self.app_overlays.surface
            && let Some(inbox) = &mut self.app_overlays.inbox
            && inbox.pending.is_none()
            && let Some((_, form)) = &mut inbox.selection
            && let Some(id) = dialog.controller.focused()
        {
            dialog.composing = form.edit(id, command);
            inbox.status = None;
            self.app_overlays.layout = None;
            self.request_redraw();
        }
    }
}
