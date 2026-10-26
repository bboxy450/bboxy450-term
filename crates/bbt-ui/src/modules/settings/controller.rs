//! Pure Settings interaction. The host owns records, persistence, focus routing and IO.
use super::{Appearance, Notifications, Records, SECTIONS};
use crate::{
    components as c,
    text_edit::{Command as EditCommand, Editor},
};
use bbt_ui_model::{TOKENS, layout::Rect};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Patch {
    Theme(String),
    Density(String),
    Rim(f32),
    Glow(bool),
    Subtitle(bool),
    NotificationsEnabled(bool),
    BackgroundOnly(bool),
    DisabledHarnesses(BTreeSet<String>),
}
impl Patch {
    pub fn validate(&self) -> Result<(), String> {
        let valid = match self {
            Self::Theme(s) => ["auto", "dark", "light"].contains(&s.as_str()),
            Self::Density(s) => ["touch", "comfortable", "cozy", "compact"].contains(&s.as_str()),
            Self::Rim(v) => {
                v.is_finite()
                    && *v >= TOKENS.glow.rim_min as f32
                    && *v <= TOKENS.glow.rim_max as f32
            }
            Self::DisabledHarnesses(set) => {
                set.len() <= 256
                    && set.iter().all(|s| {
                        !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control)
                    })
            }
            _ => true,
        };
        if valid {
            Ok(())
        } else {
            Err("invalid Settings value".into())
        }
    }
    pub fn apply(&self, appearance: &mut Appearance, notifications: &mut Notifications) {
        match self {
            Self::Theme(v) => appearance.theme = v.clone(),
            Self::Density(v) => appearance.density = v.clone(),
            Self::Rim(v) => appearance.rim = *v,
            Self::Glow(v) => appearance.glow = *v,
            Self::Subtitle(v) => appearance.subtitle = *v,
            Self::NotificationsEnabled(v) => notifications.enabled = *v,
            Self::BackgroundOnly(v) => notifications.only_when_not_frontmost = *v,
            Self::DisabledHarnesses(v) => notifications.disabled_harnesses = v.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Theme,
    Density,
    Rim,
    Glow,
    Subtitle,
    Enabled,
    Background,
    Harness(String),
    IdentityUnlock,
    Attention { destination: String, class: String },
    PluginEnable(String),
    PluginTrust(String),
    PluginSetup(String),
    KnowledgeProvider(String),
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    None,
    Search,
    Navigation,
    Control(Field),
}
#[derive(Debug, Clone)]
pub enum Action {
    Section(String),
    Search(EditCommand),
    Focus(Focus),
    NextFocus(bool),
    Set(Patch),
    Attention(super::messaging::Rule),
    Toggle(Field),
    Reset(Field),
    Nudge(i32),
    Drag(f32),
    EndDrag,
    Blur,
}
#[derive(Debug, Clone)]
pub struct Controller {
    pub section: String,
    pub query: String,
    pub focus: Focus,
    pub scroll: f32,
    pub error: Option<String>,
    editor: Editor,
    dragging: bool,
    identity_unlock: bool,
    attention: Option<super::messaging::Rule>,
}
impl Default for Controller {
    fn default() -> Self {
        Self {
            section: "Account".into(),
            query: String::new(),
            focus: Focus::None,
            scroll: 0.,
            error: None,
            editor: Editor::default(),
            dragging: false,
            identity_unlock: false,
            attention: None,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Presentation {
    pub query: String,
    pub edit: Option<c::Edit>,
    pub scroll: f32,
    pub window_controls_available: bool,
    pub read_only: bool,
    pub focus: Focus,
}
impl Default for Presentation {
    fn default() -> Self {
        Self {
            query: String::new(),
            edit: None,
            scroll: 0.,
            window_controls_available: true,
            read_only: false,
            focus: Focus::None,
        }
    }
}
impl Controller {
    pub fn new(section: &str) -> Self {
        Self {
            section: if SECTIONS.iter().any(|(name, _)| *name == section) {
                section.into()
            } else {
                "Account".into()
            },
            ..Default::default()
        }
    }
    pub fn presentation(&self) -> Presentation {
        Presentation {
            query: self.query.clone(),
            edit: (self.focus == Focus::Search).then(|| self.editor.edit()),
            scroll: self.scroll,
            window_controls_available: false,
            read_only: true,
            focus: self.focus.clone(),
        }
    }
    pub fn editing(&self) -> bool {
        self.focus == Focus::Search
    }
    pub fn scroll_by(&mut self, dy: f32, limit: f32) -> bool {
        if !dy.is_finite() || !limit.is_finite() {
            return false;
        }
        let next = (self.scroll + dy).clamp(0., limit.max(0.));
        let changed = next != self.scroll;
        self.scroll = next;
        changed
    }
    pub fn take_attention(&mut self) -> Option<super::messaging::Rule> {
        self.attention.take()
    }
    fn attention_change(
        &mut self,
        destination: String,
        class: String,
        mode: String,
        values: &Records,
    ) {
        let field = Field::Attention {
            destination: destination.clone(),
            class: class.clone(),
        };
        if !self.editable(&field, values)
            || !["wake", "inbox", "digest", "default"].contains(&mode.as_str())
        {
            self.error = Some("Messaging policy is unavailable or the mode is invalid".into());
            return;
        }
        self.focus = Focus::Control(field);
        let mode = if mode == values.messaging.default_mode(&destination, &class) {
            "default".into()
        } else {
            mode
        };
        self.attention = Some(super::messaging::Rule {
            destination,
            class,
            mode,
        });
        self.error = None;
    }
    fn editable(&self, field: &Field, values: &Records) -> bool {
        match field {
            Field::PluginEnable(_)
            | Field::PluginTrust(_)
            | Field::PluginSetup(_)
            | Field::KnowledgeProvider(_) => false,
            Field::Attention { destination, class } => {
                (self.section == "Messaging" || !self.query.trim().is_empty())
                    && !values.messaging.seat.id.is_empty()
                    && values.messaging.seat.generation != 0
                    && ["conversation", "event"].contains(&class.as_str())
                    && (destination == "direct" || values.messaging.channels.contains(destination))
            }
            Field::IdentityUnlock => {
                (self.section == "Account" || !self.query.trim().is_empty())
                    && values
                        .account
                        .custody
                        .as_ref()
                        .is_some_and(|value| value.enabled)
            }
            Field::Theme | Field::Density | Field::Rim | Field::Glow | Field::Subtitle => {
                self.section == "Appearance" || !self.query.trim().is_empty()
            }
            Field::Enabled => self.section == "Notifications" || !self.query.trim().is_empty(),
            Field::Background | Field::Harness(_) => {
                (self.section == "Notifications" || !self.query.trim().is_empty())
                    && values.notifications.enabled
            }
        }
    }
    fn fields(&self, values: &Records) -> Vec<Field> {
        match self.section.as_str() {
            "Account" if self.editable(&Field::IdentityUnlock, values) => {
                vec![Field::IdentityUnlock]
            }
            "Appearance" => vec![
                Field::Theme,
                Field::Density,
                Field::Rim,
                Field::Glow,
                Field::Subtitle,
            ],
            "Messaging" => std::iter::once("direct")
                .chain(values.messaging.channels.iter().map(String::as_str))
                .flat_map(|destination| {
                    ["conversation", "event"].map(|class| Field::Attention {
                        destination: destination.into(),
                        class: class.into(),
                    })
                })
                .filter(|field| self.editable(field, values))
                .collect(),
            "Notifications" => {
                let mut fields = vec![Field::Enabled];
                if values.notifications.enabled {
                    fields.push(Field::Background);
                    fields.extend(
                        values
                            .harness
                            .iter()
                            .map(|(id, _)| Field::Harness(id.clone())),
                    );
                }
                fields
            }
            _ => vec![],
        }
    }
    /// G3/G4's current host is a read-only reference. Never optimistically change
    /// a fixture's enable bit or a root's provider when no runtime can do the act.
    fn unavailable(&mut self, field: &Field, values: &Records) -> bool {
        let reason = match field {
            Field::PluginEnable(name) => values
                .plugins
                .iter()
                .find(|p| p.name == *name)
                .and_then(|p| p.root.as_ref())
                .map_or_else(
                    || "Plugin enable/disable is unavailable in this build".into(),
                    |root| format!("Cannot disable the provider of {root}"),
                ),
            Field::PluginTrust(_) => "Plugin trust is unavailable in this build".into(),
            Field::PluginSetup(_) => "Plugin setup is unavailable in this build".into(),
            Field::KnowledgeProvider(_) => {
                "Provider swap is unavailable: the bundled provider remains active".into()
            }
            _ => return false,
        };
        self.focus = Focus::Control(field.clone());
        self.error = Some(reason);
        true
    }
    pub fn handle(
        &mut self,
        action: Action,
        values: &mut Records,
        defaults: &Records,
    ) -> Option<Patch> {
        let reset = matches!(action, Action::Reset(_));
        let patch = match action {
            Action::Attention(rule) => {
                self.attention_change(rule.destination, rule.class, rule.mode, values);
                return None;
            }
            Action::Section(section) => {
                if SECTIONS.iter().any(|(name, _)| *name == section) {
                    self.section = section;
                    self.query.clear();
                    self.editor.reset("");
                    self.scroll = 0.;
                    self.focus = Focus::Navigation;
                    self.dragging = false;
                    self.error = None;
                }
                return None;
            }
            Action::Search(command) => {
                if self.editing() {
                    let old = self.query.clone();
                    let editor = self.editor.clone();
                    if self.editor.handle(&mut self.query, command) {
                        self.scroll = 0.;
                    }
                    self.error = None;
                    if self.query.len() > 4096 {
                        self.query = old;
                        self.editor = editor;
                        self.error = Some("Search is limited to 4096 bytes".into());
                    }
                }
                return None;
            }
            Action::Focus(focus) => {
                self.editor
                    .handle(&mut self.query, EditCommand::CancelPreedit);
                self.focus = focus;
                return None;
            }
            Action::Blur => {
                self.editor
                    .handle(&mut self.query, EditCommand::CancelPreedit);
                self.focus = Focus::None;
                self.dragging = false;
                return None;
            }
            Action::NextFocus(backwards) => {
                let mut order = vec![Focus::Search, Focus::Navigation];
                order.extend(self.fields(values).into_iter().map(Focus::Control));
                let at = order
                    .iter()
                    .position(|f| *f == self.focus)
                    .unwrap_or(if backwards { 0 } else { order.len() - 1 });
                self.focus =
                    order[(at + if backwards { order.len() - 1 } else { 1 }) % order.len()].clone();
                return None;
            }
            Action::Set(patch) => patch,
            Action::Toggle(field) => {
                if self.unavailable(&field, values) {
                    return None;
                }
                if !self.editable(&field, values) {
                    return None;
                }
                self.focus = Focus::Control(field.clone());
                if field == Field::IdentityUnlock {
                    self.identity_unlock = true;
                    return None;
                }
                match field {
                    Field::Attention { destination, class } => {
                        let mode = cycle(
                            &["wake", "inbox", "digest"],
                            values.messaging.mode(&destination, &class),
                            1,
                        );
                        self.attention_change(destination, class, mode, values);
                        return None;
                    }
                    Field::Glow => Patch::Glow(!values.appearance.glow),
                    Field::Subtitle => Patch::Subtitle(!values.appearance.subtitle),
                    Field::Enabled => Patch::NotificationsEnabled(!values.notifications.enabled),
                    Field::Background => {
                        Patch::BackgroundOnly(!values.notifications.only_when_not_frontmost)
                    }
                    Field::Harness(id) => {
                        let mut set = values.notifications.disabled_harnesses.clone();
                        if !set.remove(&id) {
                            set.insert(id);
                        }
                        Patch::DisabledHarnesses(set)
                    }
                    _ => return None,
                }
            }
            Action::Reset(field) => {
                if self.unavailable(&field, values) {
                    return None;
                }
                if !self.editable(&field, values)
                    && !(self.section == "Notifications" || !self.query.trim().is_empty())
                {
                    return None;
                }
                match field {
                    Field::Attention { destination, class } => {
                        self.attention_change(destination, class, "default".into(), values);
                        return None;
                    }
                    Field::Theme => Patch::Theme(defaults.appearance.theme.clone()),
                    Field::IdentityUnlock => return None,
                    Field::PluginEnable(_)
                    | Field::PluginTrust(_)
                    | Field::PluginSetup(_)
                    | Field::KnowledgeProvider(_) => return None,
                    Field::Density => Patch::Density(defaults.appearance.density.clone()),
                    Field::Rim => Patch::Rim(defaults.appearance.rim),
                    Field::Glow => Patch::Glow(defaults.appearance.glow),
                    Field::Subtitle => Patch::Subtitle(defaults.appearance.subtitle),
                    Field::Enabled => Patch::NotificationsEnabled(defaults.notifications.enabled),
                    Field::Background => {
                        Patch::BackgroundOnly(defaults.notifications.only_when_not_frontmost)
                    }
                    Field::Harness(id) => {
                        let mut set = values.notifications.disabled_harnesses.clone();
                        if defaults.notifications.disabled_harnesses.contains(&id) {
                            set.insert(id);
                        } else {
                            set.remove(&id);
                        }
                        Patch::DisabledHarnesses(set)
                    }
                }
            }
            Action::Nudge(delta) => match &self.focus {
                Focus::Control(Field::Attention { destination, class }) => {
                    let mode = cycle(
                        &["wake", "inbox", "digest"],
                        values.messaging.mode(destination, class),
                        delta,
                    );
                    self.attention_change(destination.clone(), class.clone(), mode, values);
                    return None;
                }
                Focus::Control(
                    field @ (Field::PluginEnable(_)
                    | Field::PluginTrust(_)
                    | Field::PluginSetup(_)
                    | Field::KnowledgeProvider(_)),
                ) => {
                    let field = field.clone();
                    self.unavailable(&field, values);
                    return None;
                }
                Focus::Navigation => {
                    let at = SECTIONS
                        .iter()
                        .position(|(s, _)| *s == self.section)
                        .unwrap_or(0);
                    let at = (at as i32 + delta).rem_euclid(SECTIONS.len() as i32) as usize;
                    self.section = SECTIONS[at].0.into();
                    self.query.clear();
                    self.editor.reset("");
                    self.scroll = 0.;
                    return None;
                }
                Focus::Control(Field::Rim) => Patch::Rim(
                    (values.appearance.rim + delta as f32)
                        .clamp(TOKENS.glow.rim_min as f32, TOKENS.glow.rim_max as f32),
                ),
                Focus::Control(Field::Theme) => Patch::Theme(cycle(
                    &["auto", "dark", "light"],
                    &values.appearance.theme,
                    delta,
                )),
                Focus::Control(Field::Density) => Patch::Density(cycle(
                    &["touch", "comfortable", "cozy", "compact"],
                    &values.appearance.density,
                    delta,
                )),
                _ => return None,
            },
            Action::Drag(fraction) => {
                if !self.editable(&Field::Rim, values) || !fraction.is_finite() {
                    return None;
                }
                self.dragging = true;
                self.focus = Focus::Control(Field::Rim);
                Patch::Rim(
                    (TOKENS.glow.rim_min as f32
                        + fraction.clamp(0., 1.)
                            * (TOKENS.glow.rim_max - TOKENS.glow.rim_min) as f32)
                        .round(),
                )
            }
            Action::EndDrag => {
                self.dragging = false;
                return None;
            }
        };
        let field = match &patch {
            Patch::Theme(_) => Field::Theme,
            Patch::Density(_) => Field::Density,
            Patch::Rim(_) => Field::Rim,
            Patch::Glow(_) => Field::Glow,
            Patch::Subtitle(_) => Field::Subtitle,
            Patch::NotificationsEnabled(_) => Field::Enabled,
            Patch::BackgroundOnly(_) => Field::Background,
            Patch::DisabledHarnesses(_) => Field::Harness(String::new()),
        };
        if !self.editable(&field, values) && !reset {
            return None;
        }
        if !matches!(field, Field::Harness(_)) {
            self.focus = Focus::Control(field);
        }
        if let Err(error) = patch.validate() {
            self.error = Some(error);
            return None;
        }
        patch.apply(&mut values.appearance, &mut values.notifications);
        self.error = None;
        Some(patch)
    }
    pub fn dragging(&self) -> bool {
        self.dragging
    }
    /// Consuming the native intent never mutates or persists preferences.
    pub fn take_identity_unlock(&mut self) -> bool {
        std::mem::take(&mut self.identity_unlock)
    }
    pub fn activate(&mut self, values: &mut Records, defaults: &Records) -> Option<Patch> {
        match self.focus.clone() {
            Focus::Control(field) => self.handle(Action::Toggle(field), values, defaults),
            _ => None,
        }
    }
}
fn cycle(items: &[&str], current: &str, delta: i32) -> String {
    let at = items.iter().position(|s| *s == current).unwrap_or(0) as i32;
    items[(at + delta).rem_euclid(items.len() as i32) as usize].into()
}

#[derive(Debug, Clone)]
pub enum Hit {
    Search,
    Section(String),
    Set(Patch),
    Attention(super::messaging::Rule),
    Toggle(Field),
    Reset(Field),
    Rim,
}
pub struct Regions {
    pub panel: Rect,
    pub viewport: Rect,
    pub search: Rect,
    pub caret: Option<Rect>,
    pub max_scroll: f32,
    pub rim: Option<Rect>,
    hits: Vec<(Rect, Hit, bool)>,
    section: String,
    theme: String,
    density: String,
}
fn find<'a>(node: &'a c::Layout, key: &str) -> Option<&'a c::Layout> {
    if matches!(&node.kind,c::LayoutKind::Named(name) if name==key) {
        Some(node)
    } else {
        node.children.iter().find_map(|n| find(n, key))
    }
}
fn contents(mut node: &c::Layout) -> &c::Layout {
    while matches!(node.kind, c::LayoutKind::Named(_)) && node.children.len() == 1 {
        node = &node.children[0];
    }
    node
}
fn contains(rect: Rect, at: [f32; 2]) -> bool {
    at[0] >= rect.x && at[0] < rect.right() && at[1] >= rect.y && at[1] < rect.bottom()
}
impl Regions {
    pub fn new(layout: &c::Layout, view: &super::Settings) -> Option<Self> {
        let main = contents(find(layout, "main")?);
        let max_scroll = if let c::LayoutKind::ScrollView { content, .. } = main.kind {
            (content[1] - main.rect.h).max(0.)
        } else {
            0.
        };
        let mut out = Self {
            panel: layout.rect,
            viewport: main.rect,
            search: find(layout, "search")?.rect,
            caret: find(layout, "settings/query/caret").map(|n| n.rect),
            max_scroll,
            rim: None,
            hits: vec![],
            section: view.section.clone(),
            theme: view.values.appearance.theme.clone(),
            density: view.values.appearance.density.clone(),
        };
        for (i, (name, _)) in SECTIONS.iter().enumerate() {
            if let Some(n) = find(layout, &format!("nav{i}")) {
                out.hits.push((n.rect, Hit::Section((*name).into()), false));
            }
        }
        out.hits.push((out.search, Hit::Search, false));
        let searching = SECTIONS
            .iter()
            .any(|(name, _)| find(layout, &format!("results/{name}")).is_some());
        for (section, _) in SECTIONS {
            let scope = if searching {
                find(layout, &format!("results/{section}"))
            } else {
                (section == view.section).then_some(layout)
            };
            let Some(scope) = scope else { continue };
            out.controls(scope, section, view);
        }
        Some(out)
    }
    fn controls(&mut self, layout: &c::Layout, section: &str, view: &super::Settings) {
        if section == "Plugins" {
            for (i, plugin) in view.values.plugins.iter().enumerate() {
                if plugin.root.is_none()
                    && let Some(node) = find(layout, &format!("card0/plugin{i}/control"))
                {
                    let field = if plugin.trust == "trusted" {
                        Field::PluginEnable(plugin.name.clone())
                    } else {
                        Field::PluginTrust(plugin.name.clone())
                    };
                    self.hits.push((node.rect, Hit::Toggle(field), true));
                }
                if let Some(node) = find(layout, &format!("card0/plugin{i}/setup")) {
                    self.hits.push((
                        node.rect,
                        Hit::Toggle(Field::PluginSetup(plugin.name.clone())),
                        true,
                    ));
                }
            }
        }
        if section == "Knowledge" {
            for (i, root) in view.values.knowledge.roots.iter().enumerate() {
                let field = Field::KnowledgeProvider(root.name.clone());
                if let Some(node) = find(layout, &format!("card0/root{i}/provider")) {
                    self.hits
                        .push((node.rect, Hit::Toggle(field.clone()), true));
                }
                if let Some(node) = find(layout, &format!("card0/r{i}/diff")) {
                    self.hits.push((node.rect, Hit::Reset(field), true));
                }
            }
        }
        if section == "Account"
            && view
                .values
                .account
                .custody
                .as_ref()
                .is_some_and(|value| value.enabled)
            && let Some(node) = find(layout, "custody/unlock")
        {
            self.hits
                .push((node.rect, Hit::Toggle(Field::IdentityUnlock), true));
        }
        if section == "Appearance" {
            for (row, choices, field, theme) in [
                (0, vec!["auto", "dark", "light"], Field::Theme, true),
                (
                    1,
                    vec!["touch", "comfortable", "cozy", "compact"],
                    Field::Density,
                    false,
                ),
            ] {
                let key = format!("card0/r{row}/ctl");
                if let Some(n) = find(layout, &key) {
                    for (child, value) in contents(n).children.iter().zip(choices) {
                        self.hits.push((
                            child.rect,
                            Hit::Set(if theme {
                                Patch::Theme(value.into())
                            } else {
                                Patch::Density(value.into())
                            }),
                            true,
                        ));
                    }
                }
                if let Some(n) = find(layout, &format!("card0/r{row}/diff")) {
                    self.hits.push((n.rect, Hit::Reset(field), true));
                }
            }
            if let Some(n) = find(layout, "card0/r2/ctl") {
                let rect = contents(n).children.first().map_or(n.rect, |n| n.rect);
                self.rim = Some(rect);
                self.hits.push((rect, Hit::Rim, true));
            }
            if let Some(n) = find(layout, "card0/r2/diff") {
                self.hits.push((n.rect, Hit::Reset(Field::Rim), true));
            }
            let row = if view.values.appearance.blur { 6 } else { 5 };
            for (i, field) in [(row, Field::Glow), (row + 1, Field::Subtitle)] {
                if let Some(n) = find(layout, &format!("card0/r{i}/ctl")) {
                    self.hits.push((n.rect, Hit::Toggle(field.clone()), true));
                }
                if let Some(n) = find(layout, &format!("card0/r{i}/diff")) {
                    self.hits.push((n.rect, Hit::Reset(field), true));
                }
            }
        } else if section == "Messaging" {
            for (i, destination) in std::iter::once("direct")
                .chain(view.values.messaging.channels.iter().map(String::as_str))
                .enumerate()
            {
                for (index, class) in ["conversation", "event"].into_iter().enumerate() {
                    let key = format!("card0/r{}", i * 2 + index);
                    if let Some(node) = find(layout, &format!("{key}/ctl")) {
                        let segment = contents(node);
                        for (index, mode) in ["wake", "inbox", "digest"].into_iter().enumerate() {
                            if let Some(node) = segment.children.get(index) {
                                self.hits.push((
                                    node.rect,
                                    Hit::Attention(super::messaging::Rule {
                                        destination: destination.into(),
                                        class: class.into(),
                                        mode: mode.into(),
                                    }),
                                    true,
                                ));
                            }
                        }
                    }
                    if let Some(node) = find(layout, &format!("{key}/diff")) {
                        self.hits.push((
                            node.rect,
                            Hit::Reset(Field::Attention {
                                destination: destination.into(),
                                class: class.into(),
                            }),
                            true,
                        ));
                    }
                }
            }
        } else if section == "Notifications" {
            let mut control = |key: &str, hit: Hit, reset: Field| {
                if (view.values.notifications.enabled || reset == Field::Enabled)
                    && let Some(n) = find(layout, key)
                {
                    self.hits.push((n.rect, hit, true));
                }
                if let Some(n) = find(layout, &key.replace("/ctl", "/diff")) {
                    self.hits.push((n.rect, Hit::Reset(reset), true));
                }
            };
            control("card0/r0/ctl", Hit::Toggle(Field::Enabled), Field::Enabled);
            {
                control(
                    "card0/r1/ctl",
                    Hit::Toggle(Field::Background),
                    Field::Background,
                );
                for (i, (name, _)) in view.values.harness.iter().enumerate() {
                    let field = Field::Harness(name.clone());
                    control(
                        &format!("card0/r{}/ctl", i + 2),
                        Hit::Toggle(field.clone()),
                        field,
                    );
                }
            }
        }
    }
    pub fn hit(&self, at: [f32; 2]) -> Option<Hit> {
        self.hits
            .iter()
            .rev()
            .find(|(rect, _, main)| (!*main || contains(self.viewport, at)) && contains(*rect, at))
            .map(|(_, hit, _)| hit.clone())
    }
    pub fn rim_fraction(&self, x: f32) -> Option<f32> {
        self.rim.map(|r| ((x - r.x) / r.w.max(1.)).clamp(0., 1.))
    }
    fn field(hit: &Hit) -> Option<Field> {
        Some(match hit {
            Hit::Attention(rule) => Field::Attention {
                destination: rule.destination.clone(),
                class: rule.class.clone(),
            },
            Hit::Set(Patch::Theme(_)) => Field::Theme,
            Hit::Set(Patch::Density(_)) => Field::Density,
            Hit::Rim => Field::Rim,
            Hit::Toggle(field) => field.clone(),
            _ => return None,
        })
    }
    pub fn fields(&self) -> Vec<Field> {
        let mut fields = vec![];
        for (_, hit, _) in &self.hits {
            if let Some(field) = Self::field(hit)
                && !fields.contains(&field)
            {
                fields.push(field);
            }
        }
        fields
    }
    pub fn focus_rect(&self, focus: &Focus) -> Option<Rect> {
        match focus {
            Focus::Search => Some(self.search),
            Focus::Navigation => self.hits.iter().find_map(|(rect, hit, _)| {
                matches!(hit,Hit::Section(name) if name==&self.section).then_some(*rect)
            }),
            Focus::Control(field) => self.hits.iter().find_map(|(rect, hit, _)| {
                let selected = match hit {
                    Hit::Set(Patch::Theme(value)) => value == &self.theme,
                    Hit::Set(Patch::Density(value)) => value == &self.density,
                    _ => true,
                };
                (selected && Self::field(hit).as_ref() == Some(field)).then_some(*rect)
            }),
            Focus::None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec gui-components §4.1
    #[test]
    fn native_unlock_is_a_consumed_intent_and_never_a_preferences_patch() {
        let mut values = Records::default();
        values.account.custody = Some(super::super::Custody {
            tier: "keychain".into(),
            hold: Some(super::super::CustodyHold {
                reason: "held".into(),
            }),
            enabled: true,
            ..Default::default()
        });
        let defaults = values.clone();
        let before = serde_json::to_value(&values).unwrap();
        let mut controller = Controller::new("Account");
        controller.handle(
            Action::Focus(Focus::Control(Field::IdentityUnlock)),
            &mut values,
            &defaults,
        );
        assert!(controller.activate(&mut values, &defaults).is_none());
        assert!(controller.take_identity_unlock());
        assert!(!controller.take_identity_unlock());
        assert_eq!(serde_json::to_value(&values).unwrap(), before);
        values.account.custody.as_mut().unwrap().enabled = false;
        controller.handle(
            Action::Toggle(Field::IdentityUnlock),
            &mut values,
            &defaults,
        );
        assert!(!controller.take_identity_unlock());
    }
    // keeps: spec gui-components §4.1
    #[test]
    fn notification_master_preserves_subordinates_and_harness_membership() {
        let mut records = Records::default();
        records.notifications.only_when_not_frontmost = true;
        records
            .notifications
            .disabled_harnesses
            .insert("codex".into());
        let mut c = Controller::new("Notifications");
        let defaults = Records::default();
        c.handle(Action::Toggle(Field::Enabled), &mut records, &defaults);
        assert!(!records.notifications.enabled);
        assert!(records.notifications.only_when_not_frontmost);
        assert!(
            c.handle(
                Action::Toggle(Field::Harness("codex".into())),
                &mut records,
                &defaults
            )
            .is_none()
        );
        assert!(records.notifications.disabled_harnesses.contains("codex"));
        c.handle(Action::Toggle(Field::Enabled), &mut records, &defaults);
        c.handle(
            Action::Toggle(Field::Harness("codex".into())),
            &mut records,
            &defaults,
        );
        assert!(!records.notifications.disabled_harnesses.contains("codex"));
        assert!(records.notifications.only_when_not_frontmost);
    }
    // keeps: spec gui-components §4.1
    #[test]
    fn ime_search_keeps_preedit_out_of_query_and_navigation_resets_scroll() {
        let mut c = Controller::new("Account");
        let mut records = Records::default();
        let defaults = Records::default();
        c.handle(Action::Focus(Focus::Search), &mut records, &defaults);
        c.handle(
            Action::Search(EditCommand::Preedit {
                text: "通知".into(),
                caret: Some(2),
            }),
            &mut records,
            &defaults,
        );
        let preedit = c.presentation();
        assert!(preedit.query.is_empty());
        assert!(preedit.edit.unwrap().preedit.is_some());
        c.handle(
            Action::Search(EditCommand::Commit("通知".into())),
            &mut records,
            &defaults,
        );
        assert_eq!(c.presentation().query, "通知");
        c.scroll_by(40., 80.);
        c.handle(
            Action::Section("Appearance".into()),
            &mut records,
            &defaults,
        );
        let presented = c.presentation();
        assert!(presented.query.is_empty());
        assert_eq!(presented.scroll, 0.);
        assert!(!c.scroll_by(f32::NAN, 80.));
    }
}
