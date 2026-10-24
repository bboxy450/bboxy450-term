//! IO-owned preferences. Only the explicit Appearance/Notifications patches write.
#![allow(
    clippy::disallowed_methods,
    reason = "bounded preferences IO owner, guarded against UI-thread calls"
)]
use bbt_ui::modules::settings::{Appearance, Notifications, controller::Patch};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::{fd::AsRawFd, unix::fs::OpenOptionsExt},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use tokio::sync::watch;
use toml_edit::{DocumentMut, Item, Value};

const MAX_CONFIG: u64 = 1024 * 1024;
static TEMP_ID: AtomicU64 = AtomicU64::new(0);
#[derive(Debug, Clone, PartialEq)]
pub struct Preferences {
    pub appearance: Appearance,
    pub notifications: Notifications,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            appearance: Appearance {
                theme: "auto".into(),
                density: "cozy".into(),
                rim: bbt_ui_model::TOKENS.glow.rim as f32,
                glow: true,
                subtitle: false,
                opacity: 1.,
                blur: false,
                blur_radius: 20.,
            },
            notifications: Notifications::default(),
        }
    }
}
impl Preferences {
    pub fn apply(&mut self, patch: &Patch) -> Result<(), String> {
        patch.validate()?;
        patch.apply(&mut self.appearance, &mut self.notifications);
        Ok(())
    }
    pub fn context(&self, system: bbt_ui::Theme) -> bbt_ui::Context {
        use bbt_ui::{Density, Theme};
        let theme = match self.appearance.theme.as_str() {
            "dark" => Theme::Dark,
            "light" => Theme::Light,
            _ => system,
        };
        let density = match self.appearance.density.as_str() {
            "touch" => Density::Touch,
            "comfortable" => Density::Comfortable,
            "compact" => Density::Compact,
            _ => Density::Cozy,
        };
        bbt_ui::Context::new(theme, density)
    }
}
#[derive(Clone)]
pub struct ContextUpdate {
    pub theme: bbt_ui::Theme,
    pub density: bbt_ui::Density,
    pub snapshot: Arc<Snapshot>,
}

/// One real mute intent; a reply is not a substitute for authoritative metadata.
#[derive(Default)]
pub struct Mute {
    pending: Option<(bbt_mux::Ulid, bbt_ui_model::SeatId, bool, bool)>,
    pub notice: Option<bbt_ui::modules::composer::Notice>,
}
impl Mute {
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn requested(&mut self, message: &crate::cockpit::UiMsg) {
        if let crate::cockpit::UiMsg::Messaging {
            id,
            request: bbt_proto::messaging::Request::Mute { seat, muted },
            ..
        } = message
        {
            self.pending = Some((*id, bbt_ui_model::SeatId::from_u128(seat.0), *muted, false));
            self.notice = Some(bbt_ui::modules::composer::Notice {
                kind: "pending".into(),
                text: "Seat mute change pending".into(),
            });
        }
    }
    pub fn observe(&mut self, model: &crate::model::Model) {
        let Some((op, seat, muted, mut acknowledged)) = self.pending else {
            return;
        };
        if let Some(reply) = model.operation_results.rows().find(|row| row.op == op) {
            match &reply.reply {
                crate::messaging::Reply::Messaging(bbt_proto::messaging::Response::Muted {
                    ..
                }) => acknowledged = true,
                reply => {
                    let reason = match reply {
                        crate::messaging::Reply::Refused(reason) => {
                            format!("Mute refused · {reason}")
                        }
                        crate::messaging::Reply::Unknown(reason) => {
                            format!("Mute outcome unknown · {reason}; check the seat metadata")
                        }
                        _ => "Mute refused · unexpected operation reply".into(),
                    };
                    self.pending = None;
                    self.notice = Some(bbt_ui::modules::composer::Notice {
                        kind: "refused".into(),
                        text: reason,
                    });
                    return;
                }
            }
        }
        self.pending = Some((op, seat, muted, acknowledged));
        if acknowledged
            && model
                .ui
                .seats
                .get(&seat)
                .is_some_and(|row| row.muted == muted)
        {
            self.pending = None;
            self.notice = Some(bbt_ui::modules::composer::Notice {
                kind: "sent".into(),
                text: if muted { "Seat muted" } else { "Seat unmuted" }.into(),
            });
        }
    }
}

/// The host projects only data actually exposed by the backend and built-in registry.
pub fn records(
    model: Option<&crate::model::Model>,
    preferences: &Preferences,
) -> bbt_ui::modules::settings::Records {
    use bbt_ui::modules::settings::{Harness, Role};
    let mut out = model.map_or_else(Default::default, |m| crate::hosts::settings(&m.hosts));
    out.appearance = preferences.appearance.clone();
    out.notifications = preferences.notifications.clone();
    if let Some(identity) = model.and_then(|m| m.identity.as_ref()) {
        out.account.key_state = format!("{:?}", identity.state);
        out.account.name = identity
            .reason
            .clone()
            .unwrap_or_else(|| out.account.key_state.clone());
    } else {
        out.account.key_state = "unavailable".into();
        out.account.name = "Identity status is not available".into();
    }
    for name in ["oracle", "architect", "creativity", "executor", "shell"] {
        if let Ok(role) = bbt_mux::adapters::role_preset(name) {
            out.roles.push((
                name.into(),
                Role {
                    cls: format!("{:?}", role.class).to_lowercase(),
                    prompt: "Not exposed by the runtime".into(),
                    send: "Authority is read-only here".into(),
                    ..Default::default()
                },
            ));
        }
    }
    for kind in ["claude", "codex", "zsh"] {
        if let Ok(h) = bbt_mux::adapters::HarnessRecord::builtin(kind) {
            out.harness.push((
                kind.into(),
                Harness {
                    role: h.default_role.name,
                    wake: format!("{:?}", h.wake_method),
                    hooks: h.hooks,
                    command: h.command.join(" "),
                    tile: h.tile,
                    icon: h.icon,
                    state_from: format!("{:?}", h.state_source),
                    custom: false,
                },
            ));
        }
    }
    if let Some(model) = model {
        for seat in model.ui.seats.values() {
            if !out.harness.iter().any(|(id, _)| *id == seat.harness.0) {
                out.harness.push((
                    seat.harness.0.clone(),
                    Harness {
                        role: seat.role.as_ref().map_or(String::new(), |r| r.name.clone()),
                        command: "Not exposed by the runtime".into(),
                        state_from: "Current seat metadata".into(),
                        icon: seat.harness.0.clone(),
                        ..Default::default()
                    },
                ));
            }
        }
    }
    out.shortcuts = [
        ("Settings", "⌘,"),
        ("Launcher", "⌘T"),
        ("Graph", "⌘G"),
        ("Mute focused seat", "⌘M"),
        ("Back to panes", "Esc"),
    ]
    .map(|(a, b)| (a.into(), b.into()))
    .to_vec();
    out
}
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub revision: u64,
    pub preferences: Preferences,
    pub loaded: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Request {
    Load { revision: u64 },
    Save { revision: u64, patches: Vec<Patch> },
}
impl Request {
    fn revision(&self) -> u64 {
        match self {
            Self::Load { revision } | Self::Save { revision, .. } => *revision,
        }
    }
}

/// Revision ownership stays in the host; old IO completions cannot acknowledge newer edits.
#[derive(Debug, Clone, Default)]
pub struct State {
    pub preferences: Preferences,
    pub loaded: bool,
    pub error: Option<String>,
    pub revision: u64,
    pending: BTreeMap<&'static str, (u64, Patch)>,
}
impl State {
    /// Do not post with defaults while the persisted policy is still loading.
    pub fn notification_policy(&self) -> crate::attention::Notifications {
        let values = &self.preferences.notifications;
        crate::attention::Notifications {
            enabled: self.loaded && values.enabled,
            only_when_not_frontmost: values.only_when_not_frontmost,
            disabled_harnesses: values.disabled_harnesses.clone(),
        }
    }
    pub fn edit(&mut self, patch: Patch) -> Result<Request, String> {
        if !self.loaded {
            return Err("Settings are not loaded; configuration was not changed".into());
        }
        self.preferences.apply(&patch)?;
        self.revision += 1;
        self.pending.insert(key(&patch).1, (self.revision, patch));
        self.error = None;
        Ok(Request::Save {
            revision: self.revision,
            patches: self.pending.values().map(|(_, p)| p.clone()).collect(),
        })
    }
    pub fn reload(&mut self) -> Request {
        self.revision += 1;
        Request::Load {
            revision: self.revision,
        }
    }
    pub fn accept(&mut self, snapshot: &Snapshot) -> bool {
        if snapshot.error.is_none() {
            self.pending
                .retain(|_, (revision, _)| *revision > snapshot.revision);
        }
        if snapshot.revision < self.revision {
            return false;
        }
        self.revision = snapshot.revision;
        self.error = snapshot.error.clone();
        if snapshot.error.is_none() {
            self.preferences = snapshot.preferences.clone();
            self.loaded = snapshot.loaded;
        }
        true
    }
    pub fn saved(&self) -> bool {
        self.loaded && self.pending.is_empty() && self.error.is_none()
    }
    pub fn refused(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
    }
    pub fn snapshot(&self) -> Arc<Snapshot> {
        Arc::new(Snapshot {
            revision: self.revision,
            preferences: self.preferences.clone(),
            loaded: self.loaded,
            error: self.error.clone(),
        })
    }
}
pub fn default_path() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".config/bboxy/config.toml"))
        .ok_or_else(|| "HOME is unavailable for Settings".into())
}
fn key(patch: &Patch) -> (&'static str, &'static str) {
    match patch {
        Patch::Theme(_) => ("appearance", "theme"),
        Patch::Density(_) => ("appearance", "density"),
        Patch::Rim(_) => ("appearance", "rim"),
        Patch::Glow(_) => ("appearance", "glow"),
        Patch::Subtitle(_) => ("appearance", "subtitle"),
        Patch::NotificationsEnabled(_) => ("notifications", "enabled"),
        Patch::BackgroundOnly(_) => ("notifications", "only_when_not_frontmost"),
        Patch::DisabledHarnesses(_) => ("notifications", "disabled_harnesses"),
    }
}
fn value(patch: &Patch) -> Value {
    match patch {
        Patch::Theme(v) | Patch::Density(v) => Value::from(v.clone()),
        Patch::Rim(v) => Value::from(*v as f64),
        Patch::Glow(v)
        | Patch::Subtitle(v)
        | Patch::NotificationsEnabled(v)
        | Patch::BackgroundOnly(v) => Value::from(*v),
        Patch::DisabledHarnesses(v) => Value::Array(v.iter().map(String::as_str).collect()),
    }
}
fn harness_array(
    old: &toml_edit::Array,
    selected: &std::collections::BTreeSet<String>,
) -> toml_edit::Array {
    let mut array = old.clone();
    let mut retained_comments = String::new();
    for index in (0..array.len()).rev() {
        if !array
            .get(index)
            .and_then(Value::as_str)
            .is_some_and(|id| selected.contains(id))
        {
            let removed = array.remove(index);
            for decor in [removed.decor().prefix(), removed.decor().suffix()]
                .into_iter()
                .flatten()
            {
                if let Some(text) = decor.as_str() {
                    for line in text.lines() {
                        if let Some(at) = line.find('#') {
                            retained_comments.push_str(&line[at..]);
                            retained_comments.push('\n');
                        }
                    }
                }
            }
        }
    }
    for id in selected {
        if !array.iter().any(|v| v.as_str() == Some(id)) {
            array.push(id.clone());
        }
    }
    if !retained_comments.is_empty() {
        let trailing = array.trailing().as_str().unwrap_or("");
        array.set_trailing(format!("\n{retained_comments}{trailing}"));
    }
    array
}
fn parse(doc: &DocumentMut) -> Result<Preferences, String> {
    let mut preferences = Preferences::default();
    for section in ["appearance", "notifications"] {
        if doc
            .get(section)
            .is_some_and(|item| item.as_table_like().is_none())
        {
            return Err(format!("Settings [{section}] must be a table"));
        }
    }
    for field in ["theme", "density", "rim", "glow", "subtitle"] {
        let Some(item) = doc.get("appearance").and_then(|s| s.get(field)) else {
            continue;
        };
        let invalid = || format!("invalid Settings appearance.{field}; file was not changed");
        let patch = match field {
            "theme" => Patch::Theme(item.as_str().ok_or_else(invalid)?.into()),
            "density" => Patch::Density(item.as_str().ok_or_else(invalid)?.into()),
            "rim" => Patch::Rim(
                item.as_float()
                    .or_else(|| item.as_integer().map(|v| v as f64))
                    .ok_or_else(invalid)? as f32,
            ),
            "glow" => Patch::Glow(item.as_bool().ok_or_else(invalid)?),
            _ => Patch::Subtitle(item.as_bool().ok_or_else(invalid)?),
        };
        preferences.apply(&patch)?;
    }
    for field in ["enabled", "only_when_not_frontmost", "disabled_harnesses"] {
        let Some(item) = doc.get("notifications").and_then(|s| s.get(field)) else {
            continue;
        };
        let invalid = || format!("invalid Settings notifications.{field}; file was not changed");
        let patch = match field {
            "enabled" => Patch::NotificationsEnabled(item.as_bool().ok_or_else(invalid)?),
            "only_when_not_frontmost" => Patch::BackgroundOnly(item.as_bool().ok_or_else(invalid)?),
            _ => Patch::DisabledHarnesses(
                item.as_array()
                    .ok_or_else(invalid)?
                    .iter()
                    .map(|v| v.as_str().map(str::to_owned).ok_or_else(invalid))
                    .collect::<Result<_, _>>()?,
            ),
        };
        preferences.apply(&patch)?;
    }
    Ok(preferences)
}

#[derive(Clone)]
pub struct Store {
    path: PathBuf,
}
impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
    fn document(path: &Path) -> Result<DocumentMut, String> {
        bbt_rt::assert_not_ui_thread();
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(DocumentMut::new()),
            Err(e) => return Err(format!("Settings read failed: {e}")),
        };
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Settings configuration must be a regular file".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_CONFIG + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| format!("Settings read failed: {e}"))?;
        if bytes.len() as u64 > MAX_CONFIG {
            return Err("Settings configuration exceeds 1 MiB".into());
        }
        let text = String::from_utf8(bytes).map_err(|_| "Settings configuration is not UTF-8")?;
        text.parse()
            .map_err(|_| "Settings configuration is invalid TOML; file was not changed".into())
    }
    pub fn load(&self) -> Result<Preferences, String> {
        parse(&Self::document(&self.path)?)
    }
    pub fn save(&self, patches: &[Patch]) -> Result<Preferences, String> {
        bbt_rt::assert_not_ui_thread();
        if patches.is_empty() || patches.len() > 8 {
            return Err("invalid Settings patch batch".into());
        }
        for patch in patches {
            patch.validate()?;
        }
        // Resolve an existing symlink so atomic replacement preserves the user's link.
        if fs::symlink_metadata(&self.path).is_ok_and(|meta| meta.file_type().is_symlink())
            && !self.path.exists()
        {
            return Err(
                "Settings configuration symlink target is unavailable; file was not changed".into(),
            );
        }
        let path = if self.path.exists() {
            self.path.canonicalize().map_err(|e| e.to_string())?
        } else {
            self.path.clone()
        };
        let parent = path.parent().ok_or("Settings path has no parent")?;
        fs::create_dir_all(parent).map_err(|e| format!("Settings save failed: {e}"))?;
        let lock = File::open(parent).map_err(|e| e.to_string())?;
        // SAFETY: flock borrows a live directory descriptor until this IO operation ends.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let mut doc = Self::document(&path)?;
        parse(&doc)?;
        for patch in patches {
            let (section, key) = key(patch);
            let slot = &mut doc[section][key];
            let mut replacement = match (patch, slot.as_array()) {
                (Patch::DisabledHarnesses(selected), Some(old)) => {
                    Value::Array(harness_array(old, selected))
                }
                _ => value(patch),
            };
            if let Some(old) = slot.as_value() {
                *replacement.decor_mut() = old.decor().clone();
            }
            *slot = Item::Value(replacement);
        }
        let preferences = parse(&doc)?;
        let bytes = doc.to_string().into_bytes();
        if bytes.len() as u64 > MAX_CONFIG {
            return Err("Settings configuration would exceed 1 MiB".into());
        }
        let temporary = parent.join(format!(
            ".bboxy-settings-{}-{}.tmp",
            std::process::id(),
            TEMP_ID.fetch_add(1, Ordering::Relaxed)
        ));
        struct Temporary(PathBuf);
        impl Drop for Temporary {
            fn drop(&mut self) {
                let _ = fs::remove_file(&self.0);
            }
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|e| format!("Settings save failed: {e}"))?;
        let _temporary = Temporary(temporary.clone());
        if let Ok(meta) = fs::metadata(&path) {
            file.set_permissions(meta.permissions())
                .map_err(|e| e.to_string())?;
        }
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| format!("Settings save failed: {e}"))?;
        fs::rename(&temporary, &path).map_err(|e| format!("Settings save failed: {e}"))?;
        lock.sync_all()
            .map_err(|e| format!("Settings saved; directory sync failed: {e}"))?;
        Ok(preferences)
    }
}

/// One coalesced request and one blocking file operation; terminal IO never awaits it.
pub struct Io {
    requests: watch::Sender<Option<Request>>,
    pub updates: watch::Receiver<Arc<Snapshot>>,
    task: tokio::task::JoinHandle<()>,
}
impl Io {
    pub fn start(
        path: PathBuf,
        handle: &tokio::runtime::Handle,
        notify: impl Fn(Arc<Snapshot>) + Send + 'static,
    ) -> Self {
        let (requests, mut pending) = watch::channel(None::<Request>);
        let (publish, updates) = watch::channel(Arc::new(Snapshot::default()));
        let task = handle.spawn(async move {
            let store = Store::new(path);
            let initial = store.clone();
            let result = tokio::task::spawn_blocking(move || initial.load())
                .await
                .unwrap_or_else(|e| Err(format!("Settings IO stopped: {e}")));
            let mut snapshot = match result {
                Ok(preferences) => Snapshot {
                    preferences,
                    loaded: true,
                    ..Default::default()
                },
                Err(error) => Snapshot {
                    error: Some(error),
                    ..Default::default()
                },
            };
            publish.send_replace(Arc::new(snapshot.clone()));
            notify(Arc::new(snapshot.clone()));
            while pending.changed().await.is_ok() {
                let Some(request) = pending.borrow_and_update().clone() else {
                    continue;
                };
                let store = store.clone();
                let revision = request.revision();
                let result = tokio::task::spawn_blocking(move || match request {
                    Request::Load { .. } => store.load(),
                    Request::Save { patches, .. } => store.save(&patches),
                })
                .await
                .unwrap_or_else(|e| Err(format!("Settings IO stopped: {e}")));
                snapshot.revision = revision;
                match result {
                    Ok(preferences) => {
                        snapshot.preferences = preferences;
                        snapshot.loaded = true;
                        snapshot.error = None;
                    }
                    Err(error) => snapshot.error = Some(error),
                }
                publish.send_replace(Arc::new(snapshot.clone()));
                notify(Arc::new(snapshot.clone()));
            }
        });
        Self {
            requests,
            updates,
            task,
        }
    }
    pub fn submit(&self, request: Request) -> Result<(), String> {
        self.requests
            .send(Some(request))
            .map_err(|_| "Settings IO is unavailable".into())
    }
}
impl Drop for Io {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_survive_restart_and_preserve_comments_and_unmanaged_keys() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let original = "# identity stays\n[identity]\nname = 'Boxy' # keep inline\n\n[appearance] # keep table\ntheme = 'auto' # keep theme note\ncustom = 7\n";
        fs::write(&path, original).unwrap();
        let store = Store::new(path.clone());
        store
            .save(&[
                Patch::Theme("light".into()),
                Patch::NotificationsEnabled(false),
            ])
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        for piece in [
            "# identity stays",
            "name = 'Boxy' # keep inline",
            "# keep table",
            "# keep theme note",
            "custom = 7",
        ] {
            assert!(text.contains(piece), "{piece}");
        }
        let prefs = Store::new(path).load().unwrap();
        assert_eq!(prefs.appearance.theme, "light");
        assert!(!prefs.notifications.enabled);
    }
    #[test]
    fn refused_save_keeps_original_bytes_and_read_does_not_create_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let store = Store::new(path.clone());
        assert_eq!(store.load().unwrap(), Preferences::default());
        assert!(!path.exists());
        fs::write(&path, "[appearance\n# broken").unwrap();
        assert!(store.save(&[Patch::Glow(false)]).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "[appearance\n# broken");
        assert!(store.save(&[Patch::Rim(f32::NAN)]).is_err());
    }
    #[test]
    fn stale_save_cannot_acknowledge_a_newer_edit() {
        let mut state = State::default();
        state.accept(&Snapshot {
            loaded: true,
            ..Default::default()
        });
        state.edit(Patch::Theme("light".into())).unwrap();
        state.edit(Patch::NotificationsEnabled(false)).unwrap();
        assert!(!state.accept(&Snapshot {
            revision: 1,
            loaded: true,
            ..Default::default()
        }));
        assert!(!state.saved());
        assert!(!state.preferences.notifications.enabled);
        let latest = state.preferences.clone();
        state.accept(&Snapshot {
            revision: 2,
            preferences: latest,
            loaded: true,
            error: None,
        });
        assert!(state.saved());
    }
    #[test]
    fn atomic_patch_preserves_a_config_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("actual.toml");
        let link = dir.path().join("config.toml");
        fs::write(&target, "# actual\n").unwrap();
        std::os::unix::fs::symlink(&target, &link).unwrap();
        Store::new(link.clone())
            .save(&[Patch::Glow(false)])
            .unwrap();
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(fs::read_to_string(target).unwrap().contains("# actual"));
    }
    #[test]
    fn busy_configuration_refuses_without_waiting_or_altering_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "# retained\n").unwrap();
        let guard = File::open(dir.path()).unwrap();
        // SAFETY: the fixture keeps this descriptor alive through the competing save.
        assert_eq!(
            unsafe { libc::flock(guard.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
            0
        );
        assert!(
            Store::new(path.clone())
                .save(&[Patch::Glow(false)])
                .is_err()
        );
        assert_eq!(fs::read_to_string(path).unwrap(), "# retained\n");
    }
    #[test]
    fn harness_array_patches_preserve_entry_comments_even_when_an_entry_is_removed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path,"# header\n[notifications]\ndisabled_harnesses = [\n  # codex rationale\n  'codex', # codex note\n  'pi', # pi note\n] # array note\n").unwrap();
        let selected = ["pi".into(), "zsh".into()].into_iter().collect();
        Store::new(path.clone())
            .save(&[Patch::DisabledHarnesses(selected)])
            .unwrap();
        let text = fs::read_to_string(&path).unwrap();
        for comment in [
            "# header",
            "# codex rationale",
            "# codex note",
            "# pi note",
            "# array note",
        ] {
            assert!(text.contains(comment), "missing {comment}: {text}");
        }
        let loaded = Store::new(path).load().unwrap();
        assert_eq!(
            loaded.notifications.disabled_harnesses,
            ["pi".into(), "zsh".into()].into_iter().collect()
        );
    }
}
