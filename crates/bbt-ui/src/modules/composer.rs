//! Retained composer and pure draft control. Destinations are supplied by the
//! host's routing tree; labels are never turned into routing authority here.
mod hit;
mod report;
mod view;
use crate::{
    components::Edit,
    text_edit::{Command as EditCommand, Editor},
};
pub use hit::{Hit, Regions};
pub use report::{Criterion, Report, Verification};
mod conversation;
pub use conversation::Conversation;
use serde::{Deserialize, Serialize};
pub const SEND_KINDS: [&str; 3] = ["text", "chunk", "decision"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Address {
    pub kind: String,
    pub label: String,
    pub role: String,
    pub parent: String,
    pub n: Option<usize>,
    pub color: Option<String>,
    pub sub: String,
    pub load: Option<usize>,
    pub sel: bool,
    /// Host-supplied identity and exact routing-tree destination, never a label.
    pub id: String,
    pub destination: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Attachment {
    pub kind: String,
    pub akind: String,
    pub title: String,
    pub meta: String,
    #[serde(alias = "state")]
    pub status: String,
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PickGroup {
    pub title: String,
    pub items: Vec<Address>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Address,
    #[default]
    Body,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Composer {
    pub mode: String,
    pub to: Vec<Address>,
    pub attach: Vec<Attachment>,
    pub priority: u8,
    pub kind: String,
    #[serde(rename = "kindMenu")]
    pub kind_menu: bool,
    pub focus: bool,
    pub typed: String,
    pub text: String,
    pub edit: Option<Edit>,
    pub field: Field,
    pub pick: Vec<PickGroup>,
    pub notice: Option<Notice>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notice {
    pub kind: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Submission {
    pub delegate: bool,
    pub to: Vec<Address>,
    pub attach: Vec<Attachment>,
    pub text: String,
    pub kind: String,
    pub priority: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Focus(Field),
    Blur,
    Edit(EditCommand),
    Choose(usize),
    ChooseSelected,
    Next { backwards: bool },
    Remove(usize),
    Priority(u8),
    Kind(String),
    ToggleKindMenu,
    Attach(Attachment),
    RemoveAttachment(usize),
    Delegate(Attachment),
    Send,
    Escape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Local,
    Changed,
    Refused(&'static str),
    Submit(Submission),
}

/// Pure editing and choice projection. The host admits effects and reports the
/// correlated result. Neither preedit nor a send gesture can clear a draft.
#[derive(Debug, Clone)]
pub struct Controller {
    pub view: Composer,
    address: Editor,
    body: Editor,
    catalog: Vec<PickGroup>,
    selected: usize,
}
impl Controller {
    pub fn new(view: Composer) -> Self {
        let mut result = Self {
            address: Editor::new(&view.typed),
            body: Editor::new(&view.text),
            catalog: view.pick.clone(),
            view,
            selected: 0,
        };
        result.sync();
        result
    }
    pub fn replace_choices(&mut self, choices: Vec<PickGroup>) {
        self.catalog = choices;
        self.filter();
    }
    pub fn composing(&self) -> bool {
        match self.view.field {
            Field::Address => self.address.composing(),
            Field::Body => self.body.composing(),
        }
    }
    pub fn handle(&mut self, command: Command) -> Outcome {
        if self.view.kind_menu {
            match command {
                Command::Edit(_) => return Outcome::Local,
                Command::Send | Command::ChooseSelected | Command::Escape => {
                    self.view.kind_menu = false;
                    self.sync();
                    return Outcome::Changed;
                }
                Command::Next { backwards } => {
                    let at = SEND_KINDS
                        .iter()
                        .position(|kind| *kind == self.view.kind)
                        .unwrap_or(0);
                    self.view.kind = SEND_KINDS[if backwards {
                        (at + 2) % 3
                    } else {
                        (at + 1) % 3
                    }]
                    .into();
                    return Outcome::Changed;
                }
                _ => {}
            }
        }
        let changed = match command {
            Command::Focus(field) => {
                self.view.kind_menu = false;
                self.view.field = field;
                self.view.focus = true;
                self.filter();
                true
            }
            Command::Blur => {
                self.view.kind_menu = false;
                self.address
                    .handle(&mut self.view.typed, EditCommand::CancelPreedit);
                self.body
                    .handle(&mut self.view.text, EditCommand::CancelPreedit);
                self.view.focus = false;
                self.view.pick.clear();
                true
            }
            Command::Edit(command) => {
                if self.view.field == Field::Body
                    && self.view.text.is_empty()
                    && matches!(&command, EditCommand::Insert(text) if matches!(text.as_str(),"@"|"#"|"↳"))
                {
                    self.view.field = Field::Address;
                }
                let changed = match self.view.field {
                    Field::Address => self.address.handle(&mut self.view.typed, command),
                    Field::Body => self.body.handle(&mut self.view.text, command),
                };
                if self.view.field == Field::Address && changed {
                    self.selected = 0;
                    self.filter();
                }
                changed
            }
            Command::Choose(index) => {
                if self.composing() {
                    return Outcome::Local;
                }
                let Some(address) = self
                    .view
                    .pick
                    .iter()
                    .flat_map(|g| &g.items)
                    .nth(index)
                    .cloned()
                else {
                    return Outcome::Local;
                };
                if address.destination.is_empty()
                    && (address.kind != "role" || address.role.is_empty())
                {
                    return Outcome::Refused("address_unresolved");
                }
                if !self.view.to.iter().any(|a| {
                    if address.kind == "role" {
                        a.kind == "role" && a.role == address.role
                    } else {
                        a.destination == address.destination
                    }
                }) {
                    self.view.to.push(address);
                }
                self.view.typed.clear();
                self.address.reset("");
                self.view.field = Field::Body;
                self.view.pick.clear();
                true
            }
            Command::ChooseSelected => return self.handle(Command::Choose(self.selected)),
            Command::Next { backwards } => {
                if self.composing() {
                    return Outcome::Local;
                }
                let count = self.view.pick.iter().map(|g| g.items.len()).sum::<usize>();
                if count == 0 {
                    return Outcome::Local;
                }
                self.selected = if backwards {
                    (self.selected + count - 1) % count
                } else {
                    (self.selected + 1) % count
                };
                self.mark_selected();
                true
            }
            Command::Remove(index) => {
                if index >= self.view.to.len() {
                    return Outcome::Local;
                }
                self.view.to.remove(index);
                true
            }
            Command::Priority(priority) => {
                if priority > 9 {
                    return Outcome::Refused("priority_out_of_range");
                }
                self.view.priority = priority;
                true
            }
            Command::Kind(kind) => {
                if !SEND_KINDS.contains(&kind.as_str()) {
                    return Outcome::Refused("message_kind_not_sendable");
                }
                self.view.kind = kind;
                self.view.kind_menu = false;
                true
            }
            Command::ToggleKindMenu => {
                self.view.kind_menu = !self.view.kind_menu;
                self.view.pick.clear();
                true
            }
            Command::Attach(attachment) => {
                self.view.attach.push(attachment);
                true
            }
            Command::RemoveAttachment(index) => {
                if index >= self.view.attach.len() {
                    return Outcome::Local;
                }
                self.view.attach.remove(index);
                true
            }
            Command::Delegate(task) => {
                self.view.mode = "delegate".into();
                self.view.attach = vec![task];
                self.view.to.clear();
                self.view.field = Field::Address;
                self.view.focus = true;
                self.filter();
                true
            }
            Command::Send => {
                if self.composing() {
                    return Outcome::Local;
                }
                if self.view.to.is_empty()
                    || self.view.to.iter().any(|a| {
                        a.destination.is_empty() && (a.kind != "role" || a.role.is_empty())
                    })
                {
                    return Outcome::Refused("destination_required");
                }
                let delegate = self.view.mode == "delegate";
                if delegate
                    && (self.view.to.len() != 1
                        || self.view.to[0].kind != "seat"
                        || self.view.attach.len() != 1
                        || self.view.attach[0].kind != "task")
                {
                    return Outcome::Refused("task_and_assignee_required");
                }
                if !delegate && self.view.text.trim().is_empty() {
                    return Outcome::Refused("message_required");
                }
                return Outcome::Submit(Submission {
                    delegate,
                    to: self.view.to.clone(),
                    attach: self.view.attach.clone(),
                    text: self.view.text.clone(),
                    kind: if self.view.kind.is_empty() {
                        "text".into()
                    } else {
                        self.view.kind.clone()
                    },
                    priority: self.view.priority,
                });
            }
            Command::Escape => {
                if self.composing() {
                    return self.handle(Command::Edit(EditCommand::CancelPreedit));
                }
                self.view.focus = false;
                self.view.pick.clear();
                true
            }
        };
        self.sync();
        if changed {
            Outcome::Changed
        } else {
            Outcome::Local
        }
    }
    /// A stored reply clears only the submitted revision. Newer typing stays.
    pub fn stored(&mut self, sent: &Submission) {
        if self.view.text == sent.text
            && self.view.to == sent.to
            && self.view.attach == sent.attach
            && self.view.priority == sent.priority
            && (self.view.kind == sent.kind || self.view.kind.is_empty() && sent.kind == "text")
        {
            self.view.text.clear();
            self.view.attach.clear();
            if sent.delegate {
                self.view.mode = "message".into();
            }
            self.body.reset("");
            self.sync();
        }
    }
    fn sync(&mut self) {
        self.view.edit =
            (self.view.focus && self.view.field == Field::Body).then(|| self.body.edit());
    }
    fn filter(&mut self) {
        if !self.view.focus || self.view.field != Field::Address {
            self.view.pick.clear();
            return;
        }
        let query = self
            .view
            .typed
            .trim_start_matches(['@', '#', '↳'])
            .to_lowercase();
        self.view.pick = self
            .catalog
            .iter()
            .filter_map(|group| {
                let items: Vec<_> = group
                    .items
                    .iter()
                    .filter(|item| {
                        (self.view.mode != "delegate" || item.kind == "seat")
                            && (query.is_empty()
                                || item.label.to_lowercase().contains(&query)
                                || item.id.starts_with(&query))
                    })
                    .cloned()
                    .collect();
                (!items.is_empty()).then(|| PickGroup {
                    title: group.title.clone(),
                    items,
                })
            })
            .collect();
        self.mark_selected();
    }
    fn mark_selected(&mut self) {
        for (index, item) in self
            .view
            .pick
            .iter_mut()
            .flat_map(|g| &mut g.items)
            .enumerate()
        {
            item.sel = index == self.selected;
        }
    }
}
