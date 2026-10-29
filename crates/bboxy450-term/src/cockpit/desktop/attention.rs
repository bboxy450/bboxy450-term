// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The existing desktop owner consumes current waiting state without a GUI
//! event lease, terminal watches, replay, or a second frontend.
use super::*;
use crate::attention::{EpisodeKey, Episodes, Notifications};
use bboxy450_desktop_notifications::{NotificationId, Notifier, Response};
use bbt_proto::gui::AttentionUpdate;
use std::{collections::BTreeSet, time::Instant};
use tokio::sync::{mpsc, watch};

const CONTEXT_SOCKET: &str = "desktop-context.sock";
const CONTEXT_BYTES: u64 = 256 * 1024;

/// Replaceable UI state; it carries no native authorization or posting authority.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub policy: Notifications,
    pub frontmost: bool,
    pub focused: Option<SeatId>,
    pub handled: Vec<(SeatId, EpisodeKey)>,
}
impl Default for Context {
    fn default() -> Self {
        Self {
            policy: Notifications {
                enabled: false,
                ..Default::default()
            },
            frontmost: false,
            focused: None,
            handled: vec![],
        }
    }
}

enum Event {
    Context {
        attachment: bbt_mux::Ulid,
        context: Context,
    },
    Detached(bbt_mux::Ulid),
    State(AttentionUpdate),
    SourceLost,
    Response(Response),
    Ready,
    Error(String),
    FixtureClick(String),
}

/// Construct the native delegate on the process main thread, before AppKit run.
pub(super) struct Prepared {
    backend: Option<Box<dyn Notifier>>,
    events: mpsc::Sender<Event>,
    received: mpsc::Receiver<Event>,
    on_click: Arc<dyn Fn(String, EpisodeKey) + Send + Sync>,
    #[cfg(debug_assertions)]
    recorder: Option<crate::notifications::recording::Recorder>,
}
impl Prepared {
    pub(super) fn new(
        config: &Config,
        on_click: Arc<dyn Fn(String, EpisodeKey) + Send + Sync>,
    ) -> Self {
        let (events, received) = mpsc::channel(32);
        let response_events = events.clone();
        let response = Arc::new(move |response| {
            let _ = response_events.try_send(Event::Response(response));
        });
        #[cfg(debug_assertions)]
        let recorder = config
            .test_account
            .as_ref()
            .map(|_| crate::notifications::recording::Recorder::new(response.clone()));
        let backend = if config.test_account.is_some() {
            // A fixture can never reach the operator's notification account,
            // even when its executable lives in a signed application bundle.
            #[cfg(debug_assertions)]
            {
                recorder
                    .clone()
                    .map(|recorder| Box::new(recorder) as Box<dyn Notifier>)
            }
            #[cfg(not(debug_assertions))]
            {
                None
            }
        } else {
            match bboxy450_desktop_notifications::open(Box::new(move |event| response(event))) {
                Ok(backend) => Some(backend),
                Err(error) => {
                    eprintln!("desktop notifications unavailable: {error}");
                    None
                }
            }
        };
        Self {
            backend,
            events,
            received,
            on_click,
            #[cfg(debug_assertions)]
            recorder,
        }
    }

    pub(super) async fn start(self, config: Config, supervisor: u32) -> Result<Service> {
        let events = self.events.clone();
        let listener = context_listener(&config)?;
        let inode = std::fs::symlink_metadata(config.run_dir.join(CONTEXT_SOCKET))?.ino();
        let context_task = tokio::spawn(receive_context(
            config.clone(),
            supervisor,
            listener,
            events.clone(),
        ));
        let observer_task = tokio::spawn(observe(config.clone(), events.clone()));
        let path = config.run_dir.join(CONTEXT_SOCKET);
        let fixture = config.test_account.is_some();
        let task = tokio::spawn(async move {
            self.run(config).await;
        });
        Ok(Service {
            events,
            task,
            context_task,
            observer_task,
            path,
            inode,
            fixture,
        })
    }

    async fn run(mut self, _config: Config) {
        let events = self.events.clone();
        let mut worker = self.backend.take().and_then(|backend| {
            match crate::notifications::Worker::start(
                backend,
                Arc::new(move |event| {
                    let event = match event {
                        crate::notifications::Event::Ready => Event::Ready,
                        crate::notifications::Event::Error(reason) => Event::Error(reason),
                    };
                    let _ = events.try_send(event);
                }),
            ) {
                Ok(worker) => Some(worker),
                Err(error) => {
                    eprintln!("desktop notification worker unavailable: {error}");
                    None
                }
            }
        });
        let mut state = State::default();
        let policy = tokio::task::spawn_blocking(|| {
            let path = crate::settings::default_path()?;
            let preferences = crate::settings::Store::new(path).load()?;
            let mut state = crate::settings::State::default();
            state.accept(&crate::settings::Snapshot {
                preferences,
                loaded: true,
                ..Default::default()
            });
            Ok::<_, String>(state.notification_policy())
        })
        .await;
        match policy {
            Ok(Ok(policy)) => state.context.policy = policy,
            Ok(Err(error)) => eprintln!("desktop notification policy unavailable: {error}"),
            Err(error) => eprintln!("desktop notification policy reader stopped: {error}"),
        }
        refresh(&state, &mut worker);
        while let Some(event) = self.received.recv().await {
            match event {
                Event::Context {
                    attachment,
                    context,
                } => state.attach(attachment, context),
                Event::Detached(attachment) => state.detach(attachment),
                Event::State(update) => state.apply(update),
                Event::SourceLost => state.source_ready = false,
                Event::Response(Response::Clicked(id)) => {
                    if let Some(episode) = state.current_key(&id.0) {
                        (self.on_click)(id.0, episode);
                    }
                }
                Event::Response(Response::Dismissed(_)) => {}
                Event::Ready => {
                    if worker.as_ref().is_some_and(|worker| {
                        let (submitted, processed) = worker.progress();
                        submitted == processed
                    }) {
                        state
                            .known
                            .retain(|id| state.seats.keys().any(|seat| id.0 == seat.to_string()));
                    }
                }
                Event::Error(reason) => eprintln!("desktop notifications: {reason}"),
                Event::FixtureClick(id) => {
                    #[cfg(debug_assertions)]
                    if let Some(recorder) = &self.recorder {
                        if let Err(error) = recorder.activate(&NotificationId(id)) {
                            eprintln!("desktop fixture notification click refused: {error}");
                        }
                    }
                    #[cfg(not(debug_assertions))]
                    let _ = id;
                }
            }
            refresh(&state, &mut worker);
            #[cfg(debug_assertions)]
            if let Some(recorder) = &self.recorder {
                if let Ok(mut snapshot) = recorder.snapshot() {
                    let (submitted, processed) =
                        worker.as_ref().map_or((0, 0), |worker| worker.progress());
                    snapshot["owner"] = serde_json::json!("desktop");
                    snapshot["source_ready"] = serde_json::json!(state.source_ready);
                    snapshot["plan_revision"] = serde_json::json!(submitted);
                    snapshot["processed_revision"] = serde_json::json!(processed);
                    snapshot["settled"] = serde_json::json!(submitted == processed);
                    snapshot["ui_attached"] = serde_json::json!(state.attachment.is_some());
                    if let Err(error) = write_fixture_state(&_config, &snapshot) {
                        eprintln!("desktop fixture recording unavailable: {error}");
                    }
                }
            }
        }
    }
}

pub(super) struct Service {
    events: mpsc::Sender<Event>,
    task: JoinHandle<()>,
    context_task: JoinHandle<()>,
    observer_task: JoinHandle<()>,
    path: PathBuf,
    inode: u64,
    fixture: bool,
}
impl Service {
    pub(super) fn activate_fixture(&self, id: String) -> Result<()> {
        if !cfg!(debug_assertions) || !self.fixture || id.len() > 128 {
            return Err(invalid("desktop notification fixture command refused"));
        }
        self.events
            .try_send(Event::FixtureClick(id))
            .map_err(|_| invalid("desktop notification fixture queue full"))
    }
}
impl Drop for Service {
    fn drop(&mut self) {
        self.task.abort();
        self.context_task.abort();
        self.observer_task.abort();
        if std::fs::symlink_metadata(&self.path).is_ok_and(|m| m.ino() == self.inode) {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

#[derive(Default)]
struct State {
    context: Context,
    attachment: Option<bbt_mux::Ulid>,
    seats: BTreeMap<SeatId, GuiSeat>,
    episodes: Episodes,
    epoch: Option<bbt_mux::Ulid>,
    revision: u64,
    known: BTreeSet<NotificationId>,
    source_ready: bool,
}
impl State {
    fn attach(&mut self, attachment: bbt_mux::Ulid, context: Context) {
        self.attachment = Some(attachment);
        self.context = context;
        for (seat, key) in &self.context.handled {
            self.episodes.handle(*seat, *key);
        }
    }
    fn detach(&mut self, attachment: bbt_mux::Ulid) {
        if self.attachment == Some(attachment) {
            self.attachment = None;
            self.context.frontmost = false;
            self.context.focused = None;
        }
    }
    fn apply(&mut self, update: AttentionUpdate) {
        if update.snapshot && self.epoch == Some(update.epoch) && update.revision < self.revision {
            return;
        }
        if !update.snapshot
            && (self.epoch != Some(update.epoch) || update.revision <= self.revision)
        {
            return;
        }
        if update.snapshot {
            if self.epoch != Some(update.epoch) {
                self.episodes = Episodes::default();
            }
            self.seats.clear();
        }
        self.epoch = Some(update.epoch);
        self.revision = update.revision;
        self.source_ready = true;
        for id in update.removed {
            self.seats.remove(&SeatId::from_u128(id.0));
            self.known.insert(NotificationId(id.to_string()));
        }
        for seat in update.seats {
            let id = SeatId::from_u128(seat.record.id.0);
            self.known
                .insert(NotificationId(seat.record.id.to_string()));
            // This source has no fixture-key fallback: every row must be a
            // current waiting episode from this exact logic epoch/generation.
            if !seat.notification_episode.is_some_and(|key| {
                key.epoch == update.epoch
                    && key.epoch != bbt_mux::Ulid::nil()
                    && key.generation == seat.record.generation
                    && key.generation == seat.state.generation
            }) || !matches!(seat.record.lifecycle, bbt_proto::registry::Lifecycle::Open)
                || !matches!(seat.state.activity, bbt_proto::Activity::Waiting(_))
            {
                self.seats.remove(&id);
                continue;
            }
            self.episodes.observe(&seat, Instant::now());
            self.seats.insert(id, seat);
        }
        self.episodes.retain(self.seats.keys().copied());
        for (seat, key) in &self.context.handled {
            self.episodes.handle(*seat, *key);
        }
    }
    fn desired(&self) -> crate::attention::Desired {
        if !self.source_ready {
            return Default::default();
        }
        self.episodes.notifications(
            &self.context.policy,
            self.context.frontmost,
            self.context.focused,
        )
    }
    /// Capture before the asynchronous Show/reattach path. A disconnected
    /// observer or an ended episode must never create a new focus intent.
    fn current_key(&self, id: &str) -> Option<EpisodeKey> {
        if !self.source_ready {
            return None;
        }
        let id = id.parse::<bbt_mux::Ulid>().ok()?;
        self.episodes.key(SeatId::from_u128(id.0))
    }
}
fn refresh(state: &State, worker: &mut Option<crate::notifications::Worker>) {
    if let Some(worker) = worker {
        worker.submit(
            state.desired(),
            state.known.clone(),
            state.context.policy.enabled,
        );
    }
}

fn context_listener(config: &Config) -> Result<UnixListener> {
    let path = config.run_dir.join(CONTEXT_SOCKET);
    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        use std::os::unix::fs::FileTypeExt;
        if !meta.file_type().is_socket() {
            return Err(invalid("desktop context path is not a socket"));
        }
        std::fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}
async fn receive_context(
    config: Config,
    supervisor: u32,
    listener: UnixListener,
    events: mpsc::Sender<Event>,
) {
    let permits = Arc::new(tokio::sync::Semaphore::new(16));
    while let Ok((stream, _)) = listener.accept().await {
        let Ok(permit) = permits.clone().try_acquire_owned() else {
            continue;
        };
        let config = config.clone();
        let events = events.clone();
        tokio::spawn(async move {
            let _permit = permit;
            let checked = async {
                let peer = stream
                    .peer_cred()?
                    .pid()
                    .and_then(|pid| u32::try_from(pid).ok());
                let owner = owners::read::<FrontendReady>(&config, "frontend")?
                    .ok_or_else(|| invalid("desktop context has no frontend owner"))?;
                if peer != Some(owner.process.pid())
                    || owner.ready.supervisor_pid != supervisor
                    || !owners::frontend_running(&config.run_dir)?.unwrap_or(false)
                {
                    return Err(invalid("desktop context peer refused"));
                }
                let attachment = bbt_mux::Ulid::new();
                let mut reader = BufReader::new(stream);
                let result = async {
                    loop {
                        let mut line = String::new();
                        let n = (&mut reader)
                            .take(CONTEXT_BYTES + 1)
                            .read_line(&mut line)
                            .await?;
                        if n == 0 {
                            break;
                        }
                        if n as u64 > CONTEXT_BYTES || !line.ends_with('\n') {
                            return Err(invalid("desktop context frame refused"));
                        }
                        let context: Context = serde_json::from_str(&line)?;
                        events
                            .send(Event::Context {
                                attachment,
                                context,
                            })
                            .await
                            .map_err(|_| invalid("desktop context receiver stopped"))?;
                    }
                    Ok(())
                }
                .await;
                let _ = events.send(Event::Detached(attachment)).await;
                result
            }
            .await;
            if let Err(error) = checked {
                eprintln!("desktop context unavailable: {error}");
            }
        });
    }
}

pub(super) async fn publish_context(
    config: Config,
    supervisor: Arc<std::sync::atomic::AtomicU32>,
    mut source: watch::Receiver<Option<Context>>,
) {
    let mut previous_error = None;
    loop {
        let result = async {
            let supervisor_pid = supervisor.load(std::sync::atomic::Ordering::Acquire);
            owners::write(&config, "frontend", FrontendReady { supervisor_pid })?;
            ensure(&config).await?;
            let stream = UnixStream::connect(config.run_dir.join(CONTEXT_SOCKET)).await?;
            let peer = stream
                .peer_cred()?
                .pid()
                .and_then(|pid| u32::try_from(pid).ok());
            if !peer.is_some_and(|pid| admit_observer(&config, pid, supervisor_pid)) {
                return Err(invalid("desktop context server refused"));
            }
            previous_error = None;
            let (mut read, mut write) = stream.into_split();
            let mut unexpected = [0_u8; 1];
            loop {
                let context = source.borrow_and_update().clone();
                if let Some(context) = context {
                    let bytes = json_frame(&context)?;
                    if bytes.len() as u64 > CONTEXT_BYTES {
                        return Err(invalid("desktop context exceeds its frame bound"));
                    }
                    write_frame(&mut write, &bytes).await?;
                }
                tokio::select! {
                    changed = source.changed() => if changed.is_err() { return Ok(()); },
                    ended = read.read(&mut unexpected) => {
                        ended?;
                        return Err(invalid("desktop context owner disconnected"));
                    }
                }
            }
        }
        .await;
        match result {
            Ok(()) => break,
            Err(error) => {
                let reason = error.to_string();
                if previous_error.as_ref() != Some(&reason) {
                    eprintln!("desktop context: {reason}");
                    previous_error = Some(reason);
                }
            }
        }
        tokio::select! {
            changed = source.changed() => if changed.is_err() { break; },
            _ = tokio::time::sleep(Duration::from_secs(1)) => {}
        }
    }
}

async fn observe(config: Config, events: mpsc::Sender<Event>) {
    let mut previous_error = None;
    loop {
        let result = async {
            let mut endpoint = owners::read::<LogicReady>(&config, "logic")?
                .ok_or_else(|| invalid("desktop attention logic unavailable"))?
                .ready;
            let mut io = attach_attention(&config, &mut endpoint).await?;
            let id = bbt_mux::Ulid::new();
            io.try_send(GuiAction::ObserveAttention { id })?;
            previous_error = None;
            loop {
                match io.event().await {
                    crate::cockpit::Event::Reply(reply) => match *reply {
                        GuiReply::CurrentAttention { op, update }
                            if op.is_none() || op == Some(id) =>
                        {
                            events
                                .send(Event::State(update))
                                .await
                                .map_err(|_| invalid("desktop attention receiver stopped"))?;
                        }
                        GuiReply::Error { reason, .. } => return Err(invalid(reason)),
                        _ => {}
                    },
                    crate::cockpit::Event::Disconnected(reason) => return Err(invalid(reason)),
                    crate::cockpit::Event::Exited { .. } => {
                        return Err(invalid("desktop attention source exited"));
                    }
                }
            }
        }
        .await;
        let _ = events.send(Event::SourceLost).await;
        if let Err(error) = result {
            let reason = error.to_string();
            if previous_error.as_ref() != Some(&reason) {
                eprintln!("desktop attention: {reason}");
                previous_error = Some(reason);
            }
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

#[cfg(debug_assertions)]
fn write_fixture_state(config: &Config, value: &serde_json::Value) -> Result<()> {
    let mut temporary = tempfile::NamedTempFile::new_in(&config.run_dir)?;
    serde_json::to_writer(temporary.as_file_mut(), value)?;
    temporary
        .persist(config.run_dir.join("desktop-notifications.json"))
        .map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::{Activity, Grade, ProcessStatus, SeatState, WaitKind, registry::SeatRecord};

    fn waiting(epoch: u128, ordinal: u64) -> GuiSeat {
        GuiSeat {
            attention: None,
            notification_episode: Some(EpisodeKey {
                epoch: bbt_mux::Ulid(epoch),
                generation: 1,
                ordinal,
            }),
            record: SeatRecord::new(bbt_mux::Ulid(1), bbt_mux::Ulid(2), "fixture", "claude"),
            state: SeatState {
                generation: 1,
                process: ProcessStatus::Running { pid: 9 },
                activity: Activity::Waiting(WaitKind::Question),
                grade: Grade::Shim,
                title: String::new(),
                detail: Some("Answer this?".into()),
                turn_open: true,
                observed_ms: 10,
                subagents: vec![],
            },
        }
    }
    fn snapshot(epoch: u128, revision: u64, ordinal: u64) -> AttentionUpdate {
        AttentionUpdate {
            epoch: bbt_mux::Ulid(epoch),
            revision,
            snapshot: true,
            seats: vec![waiting(epoch, ordinal)],
            removed: vec![],
        }
    }
    fn enabled_context() -> Context {
        Context {
            policy: Notifications::default(),
            ..Default::default()
        }
    }

    #[test]
    fn source_revisions_and_epochs_refuse_stale_state_and_clicks() {
        let mut state = State {
            context: enabled_context(),
            ..Default::default()
        };
        state.apply(snapshot(4, 8, 1));
        let id = bbt_mux::Ulid(1).to_string();
        let captured = state.current_key(&id).unwrap();
        assert_eq!(captured.epoch, bbt_mux::Ulid(4));
        assert_eq!(state.desired().len(), 1);
        state.apply(snapshot(4, 7, 2));
        assert_eq!(state.current_key(&id), Some(captured));
        state.apply(AttentionUpdate {
            epoch: bbt_mux::Ulid(5),
            revision: 9,
            snapshot: false,
            seats: vec![waiting(5, 2)],
            removed: vec![],
        });
        assert_eq!(state.current_key(&id), Some(captured));
        state.source_ready = false;
        assert!(state.desired().is_empty());
        assert!(state.current_key(&id).is_none());
        state.apply(snapshot(5, 0, 1));
        let next = state.current_key(&id).unwrap();
        assert_ne!(next, captured);
        assert!(!state.episodes.handle(SeatId::from_u128(1), captured));
        assert_eq!(state.desired().len(), 1);
    }

    #[test]
    fn frontend_detach_preserves_policy_and_exact_handled_episode() {
        let mut state = State::default();
        state.apply(snapshot(4, 1, 1));
        let id = SeatId::from_u128(1);
        let key = state.episodes.key(id).unwrap();
        let first = bbt_mux::Ulid(10);
        let second = bbt_mux::Ulid(11);
        let mut context = enabled_context();
        context.frontmost = true;
        context.focused = Some(id);
        context.handled = vec![(id, key)];
        state.attach(first, context.clone());
        state.attach(second, context);
        state.detach(first);
        assert!(state.context.frontmost, "old EOF cannot detach a newer UI");
        state.detach(second);
        assert!(state.context.policy.enabled);
        assert!(!state.context.frontmost);
        assert!(state.context.focused.is_none());
        assert!(state.desired().is_empty(), "handled survives UI loss");
        state.apply(snapshot(4, 2, 1));
        assert!(
            state.desired().is_empty(),
            "bootstrap preserves that episode"
        );
        state.apply(snapshot(4, 3, 2));
        assert_eq!(state.desired().len(), 1, "new question is not handled");
    }

    #[test]
    fn nonwaiting_removal_withdraws_known_notice_and_ends_click_target() {
        let mut state = State {
            context: enabled_context(),
            ..Default::default()
        };
        state.apply(snapshot(4, 1, 1));
        let id = bbt_mux::Ulid(1).to_string();
        assert!(state.current_key(&id).is_some());
        state.apply(AttentionUpdate {
            epoch: bbt_mux::Ulid(4),
            revision: 2,
            snapshot: false,
            seats: vec![],
            removed: vec![bbt_mux::Ulid(1)],
        });
        assert!(state.desired().is_empty());
        assert!(state.current_key(&id).is_none());
        assert!(state.known.contains(&NotificationId(id)));
    }

    #[test]
    fn invalid_current_rows_end_prior_episode_instead_of_using_fixture_fallback() {
        for invalid in [
            "missing key",
            "different epoch",
            "nil epoch",
            "key generation",
            "record generation",
            "state generation",
            "closed seat",
            "not waiting",
        ] {
            let mut state = State {
                context: enabled_context(),
                ..Default::default()
            };
            state.apply(snapshot(4, 1, 1));
            let mut row = waiting(4, 2);
            let mut epoch = bbt_mux::Ulid(4);
            match invalid {
                "missing key" => row.notification_episode = None,
                "different epoch" => {
                    row.notification_episode.as_mut().unwrap().epoch = bbt_mux::Ulid(5)
                }
                "nil epoch" => {
                    epoch = bbt_mux::Ulid::nil();
                    row.notification_episode.as_mut().unwrap().epoch = epoch;
                }
                "key generation" => row.notification_episode.as_mut().unwrap().generation = 2,
                "record generation" => row.record.generation = 2,
                "state generation" => row.state.generation = 2,
                "closed seat" => row.record.lifecycle = bbt_proto::registry::Lifecycle::ClosedGrace,
                "not waiting" => row.state.activity = Activity::Idle,
                _ => unreachable!(),
            }
            state.apply(AttentionUpdate {
                epoch,
                revision: 2,
                snapshot: invalid == "nil epoch",
                seats: vec![row],
                removed: vec![],
            });
            assert!(state.desired().is_empty(), "{invalid}");
            let id = bbt_mux::Ulid(1).to_string();
            assert!(state.current_key(&id).is_none(), "{invalid}");
            assert!(state.known.contains(&NotificationId(id)), "{invalid}");
        }
    }

    #[tokio::test]
    async fn context_requires_current_frontend_owner_and_tracks_eof() {
        let root = tempfile::tempdir().unwrap();
        let config = Config {
            executable: std::env::current_exe().unwrap(),
            socket: root.path().join("mux.sock"),
            run_dir: root.path().into(),
            repository: root.path().into(),
            shell_config: None,
            ctl: false,
            test_account: Some(root.path().join("account")),
        };
        let _lock = owners::FrontendLock::claim(&config).unwrap().unwrap();
        let listener = context_listener(&config).unwrap();
        assert_eq!(
            std::fs::metadata(config.run_dir.join(CONTEXT_SOCKET))
                .unwrap()
                .mode()
                & 0o777,
            0o600
        );
        let (events, mut received) = mpsc::channel(4);
        let task = tokio::spawn(receive_context(config.clone(), 19, listener, events));
        let mut unrecorded = UnixStream::connect(config.run_dir.join(CONTEXT_SOCKET))
            .await
            .unwrap();
        let mut reply = vec![];
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), unrecorded.read_to_end(&mut reply))
                .await
                .unwrap()
                .unwrap(),
            0
        );
        assert!(received.try_recv().is_err());
        owners::write(&config, "frontend", FrontendReady { supervisor_pid: 19 }).unwrap();
        let mut stream = UnixStream::connect(config.run_dir.join(CONTEXT_SOCKET))
            .await
            .unwrap();
        let expected = enabled_context();
        write_json(&mut stream, &expected).await.unwrap();
        let event = tokio::time::timeout(Duration::from_secs(2), received.recv())
            .await
            .unwrap()
            .unwrap();
        let Event::Context {
            attachment,
            context,
        } = event
        else {
            panic!("expected current frontend context");
        };
        assert_eq!(context, expected);
        drop(stream);
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(2), received.recv()).await.unwrap(),
            Some(Event::Detached(ended)) if ended == attachment
        ));
        task.abort();
    }
}
