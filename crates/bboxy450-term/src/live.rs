//! GUI socket IO, immutable model publication and worker-only preparation.
use std::{
    collections::{BTreeMap, VecDeque},
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{UiEvent, log::Logger};
use bboxy450_term::{
    cockpit::{self, Session},
    grids::Grids,
    model::{self, Model, PanePreparer},
};
use bbt_mux::{
    Ulid,
    store::{Result, invalid},
};
use bbt_proto::{
    GuiAction, GuiIdentity, GuiReply, GuiSeat, ProcessStatus, SeatId, SpawnSpec, SwarmId, Update,
    Where,
};
use bbt_render::{cells::terminal, device::DeviceContext};
use bbt_rt::Latest;
use tokio::sync::{mpsc, oneshot};
use winit::event_loop::EventLoopProxy;

pub struct RenderContext {
    pub context: Arc<DeviceContext>,
    pub scale: f32,
    pub theme: bbt_ui_model::tokens::Theme,
}
pub use bboxy450_term::cockpit::UiMsg;
const COMMAND_CAPACITY: usize = 64;

/// Pure translation on the IO side. UI inputs never choose argv, cwd, role
/// authority or a process handle; the existing backend validates the spec.
fn spawn_spec(message: &UiMsg, swarm: Ulid, cwd: &Path) -> Option<SpawnSpec> {
    let (harness_kind, name, location) = match message {
        UiMsg::Spawn {
            harness_kind,
            name,
            location,
        } => (harness_kind.as_str(), name.clone(), location.clone()),
        UiMsg::SpawnZsh => ("zsh", None, Where::Local),
        _ => return None,
    };
    Some(SpawnSpec {
        swarm: SwarmId::from_u128(swarm.0),
        harness_kind: harness_kind.into(),
        name,
        role: None,
        cwd: cwd.to_string_lossy().into_owned(),
        location,
        resume: None,
    })
}

pub struct CommandSender {
    sender: mpsc::Sender<UiMsg>,
    // Dropping the UI owner wakes shutdown even when the queue and retained
    // retry slot are full. No polling timer and no UI wait are needed.
    _stop: oneshot::Sender<()>,
}
impl std::ops::Deref for CommandSender {
    type Target = mpsc::Sender<UiMsg>;
    fn deref(&self) -> &Self::Target {
        &self.sender
    }
}
struct Commands {
    messages: mpsc::Receiver<UiMsg>,
    stop: oneshot::Receiver<()>,
}
fn command_channel() -> (CommandSender, Commands) {
    let (sender, messages) = mpsc::channel(COMMAND_CAPACITY);
    let (stop, stopped) = oneshot::channel();
    (
        CommandSender {
            sender,
            _stop: stop,
        },
        Commands {
            messages,
            stop: stopped,
        },
    )
}

pub fn start(
    updates: Arc<Latest<Arc<Model>>>,
    proxy: EventLoopProxy<UiEvent>,
    logger: Logger,
    run_dir: Option<PathBuf>,
    rendering: oneshot::Receiver<RenderContext>,
) -> CommandSender {
    let (sender, commands) = command_channel();
    bbt_rt::handle().spawn(async move {
        let result = async {
            let config = cockpit::Config::discover(run_dir)?;
            let cwd = config.repository.clone();
            let session = Session::start(config).await?;
            run(
                updates,
                commands,
                logger.clone(),
                session,
                cwd,
                rendering,
                |reason| {
                    let event = reason.map_or(UiEvent::BackendReady, UiEvent::BackendLost);
                    let _ = proxy.send_event(event);
                },
            )
            .await
        }
        .await;
        if let Err(error) = &result {
            logger.write(format!("cockpit stopped: {error}"));
        }
        let _ = proxy.send_event(UiEvent::BackendStopped(result.map_err(|e| e.to_string())));
    });
    sender
}

fn publish(
    seats: &BTreeMap<Ulid, GuiSeat>,
    identity: &GuiIdentity,
    grids: &Grids,
    previous: &mut Arc<Model>,
    preparer: &mut PanePreparer,
    updates: &Latest<Arc<Model>>,
) -> Result<()> {
    let seats: Vec<_> = seats.values().cloned().collect();
    let records: Vec<_> = seats.iter().map(|seat| seat.record.clone()).collect();
    let mut next = model::build(&records, grids, previous);
    next.apply_seat_states(&seats);
    next.identity = Some(identity.clone());
    preparer.prepare(&mut next)?;
    *previous = Arc::new(next);
    updates.publish(previous.clone());
    Ok(())
}

fn queue_action(outgoing: &mut VecDeque<GuiAction>, action: GuiAction) {
    if let GuiAction::Unwatch { seat } = &action
        && outgoing
            .iter()
            .rev()
            .find(|old| matches!(old, GuiAction::Watch { seat: old_seat, .. } | GuiAction::Unwatch { seat: old_seat } if old_seat == seat))
            .is_some_and(|old| matches!(old, GuiAction::Unwatch { .. }))
    {
        return;
    }
    outgoing.push_back(action);
}

async fn run(
    updates: Arc<Latest<Arc<Model>>>,
    mut commands: Commands,
    logger: Logger,
    mut session: Session,
    cwd: PathBuf,
    rendering: oneshot::Receiver<RenderContext>,
    backend_lost: impl Fn(Option<String>),
) -> Result<()> {
    logger.write(format!(
        "cockpit ready: supervisor={} logic={}",
        session.supervisor.id().unwrap_or(0),
        session.logic.id().unwrap_or(0)
    ));
    let mut seats: BTreeMap<_, _> = session
        .list()
        .await?
        .into_iter()
        .map(|seat| (seat.record.id, seat))
        .collect();
    let mut identity = session.endpoint.identity.clone();
    let swarm = seats
        .values()
        .next()
        .map_or_else(Ulid::new, |seat| seat.record.swarm);
    let rendering = rendering
        .await
        .map_err(|_| invalid("renderer bootstrap stopped"))?;
    let mut options = terminal::Options::default();
    options.font.scale = rendering.scale.round().max(1.0) as u32;
    let mut preparer = PanePreparer::with_options(rendering.context, options)?;
    let mut grids = Grids::new();
    let mut initial = Model::default();
    initial.ui.window.theme = rendering.theme;
    let mut model = Arc::new(initial);
    publish(
        &seats,
        &identity,
        &grids,
        &mut model,
        &mut preparer,
        &updates,
    )?;
    let mut lost = false;
    let mut retained = VecDeque::new();
    let mut in_flight = None;
    let mut closing = false;
    // Accept one UI command's actions at a time. Watch diffs are finite;
    // the stream allows one unapplied update per watch. Deduplicate Unwatch
    // notifications so a full writer cannot create an unbounded control queue.
    let mut outgoing = VecDeque::from([GuiAction::ListHosts]);
    while !closing {
        let mut writer_blocked = false;
        if !lost {
            while let Some(action) = outgoing.front() {
                match session.send(action.clone()).await {
                    Ok(()) => {
                        outgoing.pop_front();
                    }
                    Err(error) if error.is::<cockpit::Busy>() => {
                        writer_blocked = true;
                        break;
                    }
                    Err(error) => {
                        logger.write(format!("GUI writer stopped: {error}"));
                        backend_lost(Some(error.to_string()));
                        lost = true;
                        break;
                    }
                }
            }
        }
        let writer_ready = session.writer_ready();
        let result:Result<()>=async {
            tokio::select! {
                command=async {
                    if !lost && let Some(message)=retained.pop_front() { return Some(message); }
                    commands.messages.recv().await
                }, if (lost && retained.len() < COMMAND_CAPACITY) || (!lost && outgoing.is_empty())=>match command {
                    Some(message) if lost=>retained.push_back(message),
                    Some(message)=> {
                    in_flight=Some(message);
                    match in_flight.as_ref().unwrap() {
                    message @ (UiMsg::SpawnZsh | UiMsg::Spawn {..})=>queue_action(&mut outgoing, GuiAction::Spawn {
                        spec:spawn_spec(message,swarm,&cwd).expect("spawn intent branch"),
                    }),
                    UiMsg::Input {seat,generation,id,bytes}=>queue_action(&mut outgoing, GuiAction::Input {seat:*seat,generation:*generation,id:*id,bytes:bytes.clone()}),
                    UiMsg::Resize {seat,generation,cols,rows}=>queue_action(&mut outgoing, GuiAction::Resize {seat:*seat,generation:*generation,cols:*cols,rows:*rows}),
                    UiMsg::Watched(panes)=> {
                        Arc::make_mut(&mut model).ui.window.stage.focused = panes.first().map(|(seat, _)| *seat);
                        for action in grids.set_watched(panes) {queue_action(&mut outgoing, action);}
                        publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                    }
                    }
                    in_flight=None;
                    }
                    None=>closing=true,
                },
                _=&mut commands.stop=>closing=true,
                ready=writer_ready, if writer_blocked && !lost=>{ready?;},
                event=session.event()=>match event.unwrap_or_else(|error| cockpit::Event::Disconnected(error.to_string())) {
                    cockpit::Event::Reply(reply)=>match *reply {
                        GuiReply::Hosts(hosts)=> {
                            if model.hosts != hosts {
                                Arc::make_mut(&mut model).hosts = hosts;
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                        GuiReply::HostState {host}=> {
                            let current = &mut Arc::make_mut(&mut model).hosts;
                            let changed = if let Some(old) = current.iter_mut().find(|old| old.location == host.location) {
                                if old == &host { false } else { *old = host; true }
                            } else { current.push(host); true };
                            if changed { publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?; }
                        }
                        GuiReply::Identity(next)=> {
                            if identity!=next {
                                identity=next;
                                logger.write(format!("identity status: {:?}",identity.state));
                                publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            }
                        }
                        GuiReply::Seats(next)=> {
                            seats=next.into_iter().map(|seat|(seat.record.id,seat)).collect();
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                            if lost { lost=false; backend_lost(None); }
                        }
                        GuiReply::Seat(seat)=> {
                            if seats.get(&seat.record.id).is_some_and(|old|old==&seat||old.state.generation>seat.state.generation) {return Ok(());}
                            let id=SeatId::from_u128(seat.record.id.0);
                            if matches!(seat.state.process,ProcessStatus::Exited {..}|ProcessStatus::Retained) {
                                grids.forget(id);
                                queue_action(&mut outgoing, GuiAction::Unwatch {seat:id});
                            }
                            logger.write(format!("seat {} activity={:?} process={:?}",seat.record.id,seat.state.activity,seat.state.process));
                            seats.insert(seat.record.id,seat);
                            publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;
                        }
                        GuiReply::Update {seat,update}=> {
                            let (view,sequence)=match &update {
                                Update::Snapshot {snapshot}=>(snapshot.view,snapshot.sequence),
                                Update::Delta {delta}=>(delta.view,delta.sequence),
                            };
                            match grids.apply(seat,&update) {
                                Ok(changed)=> {
                                    if changed.is_some() {publish(&seats,&identity,&grids,&mut model,&mut preparer,&updates)?;}
                                    // A valid byte-equivalent update still needs
                                    // its ack; an unwatched in-flight one does not.
                                    if grids.screen(seat).is_some() {
                                        queue_action(&mut outgoing, GuiAction::Applied {seat,view,sequence});
                                    }
                                }
                                Err(error)=>logger.write(format!("grid update rejected (no ack): {error}")),
                            }
                        }
                        GuiReply::Ok {op}=>logger.write(format!("GUI operation {op} accepted")),
                        GuiReply::Error {op,reason}=>logger.write(format!("GUI operation {op} rejected: {reason}")),
                        GuiReply::History {..}=>logger.write("history reply deferred to C1c"),
                    },
                    cockpit::Event::Exited {process,status}=> {
                        let reason=format!("{process} exited: {status}");
                        logger.write(&reason);
                        if !lost { backend_lost(Some(reason)); }
                        lost=true;
                    }
                    cockpit::Event::Disconnected(reason)=> {
                        logger.write(format!("GUI disconnected: {reason}"));
                        if !lost { backend_lost(Some(reason)); }
                        lost=true;
                    }
                }
            }
        Ok(())
        }.await;
        if let Err(error) = result {
            logger.write(format!("live IO stopped: {error}"));
            if !lost {
                backend_lost(Some(error.to_string()));
            }
            lost = true;
            if let Some(message) = in_flight.take() {
                if let UiMsg::Watched(panes) = &message {
                    for (seat, _) in panes {
                        grids.forget(*seat);
                    }
                }
                retained.push_front(message);
            }
        }
    }
    let stopped = session.shutdown().await;
    if stopped.is_ok() {
        logger.write("cockpit children reaped after orderly shutdown");
    }
    stopped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_spawn_preserves_inputs_and_uses_io_defaults() {
        let swarm = Ulid::from(42_u128);
        let cwd = Path::new("/fixture/repository with spaces");
        for location in [
            Where::Local,
            Where::Ssh { host: "rig".into() },
            Where::Cloud {
                provider: "provider".into(),
                host: "worker".into(),
            },
        ] {
            let message = UiMsg::Spawn {
                harness_kind: "codex".into(),
                name: Some("named seat".into()),
                location: location.clone(),
            };
            let spec = spawn_spec(&message, swarm, cwd).unwrap();
            assert_eq!(spec.swarm, SwarmId::from_u128(swarm.0));
            assert_eq!(spec.harness_kind, "codex");
            assert_eq!(spec.name.as_deref(), Some("named seat"));
            assert_eq!(spec.location, location);
            assert_eq!(spec.cwd, "/fixture/repository with spaces");
            assert_eq!(spec.role, None);
            assert_eq!(spec.resume, None);
            let wire = serde_json::to_value(&spec).unwrap();
            assert_eq!(serde_json::from_value::<SpawnSpec>(wire).unwrap(), spec);
        }
        let generic = UiMsg::Spawn {
            harness_kind: "zsh".into(),
            name: None,
            location: Where::Local,
        };
        assert_eq!(
            spawn_spec(&generic, swarm, cwd),
            spawn_spec(&UiMsg::SpawnZsh, swarm, cwd)
        );
        assert!(spawn_spec(&UiMsg::Watched(vec![]), swarm, cwd).is_none());
        // Unsupported kinds reach the existing backend rejection, never a
        // shell fallback or a command guessed from a label.
        let unsupported = UiMsg::Spawn {
            harness_kind: "pi".into(),
            name: None,
            location: Where::Local,
        };
        assert_eq!(
            spawn_spec(&unsupported, swarm, cwd).unwrap().harness_kind,
            "pi"
        );
    }
    use std::{os::unix::fs::PermissionsExt, time::Duration};

    #[test]
    fn queued_unwatch_after_a_rewatch_is_not_lost_to_deduplication() {
        let seat = SeatId::from_u128(42);
        let view = bbt_proto::View {
            seat: Ulid(seat.0.0),
            generation: 2,
            view_generation: Ulid::new(),
        };
        let mut outgoing = VecDeque::new();
        queue_action(&mut outgoing, GuiAction::Unwatch { seat });
        queue_action(&mut outgoing, GuiAction::Watch { seat, view });
        queue_action(&mut outgoing, GuiAction::Unwatch { seat });
        queue_action(&mut outgoing, GuiAction::Unwatch { seat });
        assert_eq!(outgoing.len(), 3);
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::Unwatch { .. })
        ));
        assert!(
            matches!(outgoing.pop_front(), Some(GuiAction::Watch { view: token, .. }) if token == view)
        );
        assert!(matches!(
            outgoing.pop_front(),
            Some(GuiAction::Unwatch { .. })
        ));
    }

    struct NoWake;
    impl bbt_rt::Waker for NoWake {
        fn wake(&self) {}
    }

    fn lines(screen: &bbt_proto::Screen) -> Vec<String> {
        screen
            .cells
            .iter()
            .map(|row| {
                row.iter()
                    .map(|cell| cell.c)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }
    async fn until(
        updates: &Latest<Arc<Model>>,
        current: &mut Arc<Model>,
        check: impl Fn(&Model) -> bool,
    ) {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Some(next) = updates.take() {
                    *current = next;
                }
                if check(current) {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            panic!(
                "model condition timed out; screens: {:?}",
                current
                    .screens
                    .iter()
                    .map(|(id, screen)| (id, lines(screen)))
                    .collect::<Vec<_>>()
            )
        });
    }
    async fn input(sender: &CommandSender, seat: SeatId, generation: u64, bytes: &[u8]) {
        sender
            .send(UiMsg::Input {
                seat,
                generation,
                id: Ulid::new(),
                bytes: bytes.into(),
            })
            .await
            .unwrap();
    }

    /// Exercise the same IO owner the native app starts, with real children,
    /// a real PTY, the Replica and worker preparation (no synthetic Updates).
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[allow(
        clippy::disallowed_methods,
        reason = "headless integration test owns its runtime and child processes"
    )]
    async fn real_io_echo_resize_reflow_and_logic_loss_retains_last_frame() {
        let root = tempfile::Builder::new()
            .prefix("c1i-io-")
            .tempdir()
            .unwrap();
        let repository = root.path().join("repository");
        let shell_config = root.path().join("zsh-config");
        tokio::fs::create_dir(&repository).await.unwrap();
        tokio::fs::create_dir(&shell_config).await.unwrap();
        tokio::fs::write(shell_config.join(".zshrc"), "PROMPT='> '\nRPROMPT=''\n")
            .await
            .unwrap();
        let binary = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("bboxy450-term");
        assert!(
            binary.is_file(),
            "build the app binary before this real-process test"
        );
        let executable = root.path().join("test-app");
        tokio::fs::write(
            &executable,
            format!(
                "#!/bin/sh\nif [ \"$1\" = logic ]; then\n  if [ -e '{marker}' ]; then\n    echo $$ > '{held_pid}'\n    exec /bin/cat >/dev/null\n  fi\n  : > '{marker}'\nfi\nBBOXY_TEST_CUSTODY=memory exec '{binary}' \"$@\"\n",
                marker = root.path().join("logic-started").display(),
                held_pid = root.path().join("held-replacement-pid").display(),
                binary = binary.to_string_lossy().replace('\'', "'\\''")
            ),
        )
        .await
        .unwrap();
        tokio::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))
            .await
            .unwrap();
        let session = Session::start(cockpit::Config {
            executable,
            socket: root.path().join("run/mux.sock"),
            run_dir: root.path().join("run"),
            repository: repository.clone(),
            shell_config: Some(shell_config),
        })
        .await
        .unwrap();
        let supervisor_pid = session.supervisor.id().unwrap();
        let logic_pid = session.logic.id().unwrap();
        let pids = format!("owned supervisor={supervisor_pid} logic={logic_pid}\n");
        tokio::fs::write(root.path().join("owned-pids.log"), &pids)
            .await
            .unwrap();
        eprint!("{pids}");
        let context = Arc::new(DeviceContext::new().await.unwrap());
        let dirty = bbt_rt::DirtySet::new(1, Arc::new(NoWake));
        let updates = Arc::new(Latest::new(dirty.source(0)));
        let (sender, commands) = command_channel();
        let (render, rendering) = oneshot::channel();
        render
            .send(RenderContext {
                context,
                scale: 1.,
                theme: bbt_ui_model::tokens::Theme::Dark,
            })
            .ok()
            .unwrap();
        let (lost_sender, mut lost_receiver) = mpsc::unbounded_channel();
        let task_updates = updates.clone();
        let task = tokio::spawn(async move {
            run(
                task_updates,
                commands,
                Logger::new(),
                session,
                repository,
                rendering,
                |reason| {
                    let _ = lost_sender.send(reason);
                },
            )
            .await
        });
        let mut current = Arc::new(Model::default());
        sender
            .send(UiMsg::Spawn {
                harness_kind: "zsh".into(),
                name: Some("generic shell".into()),
                location: Where::Local,
            })
            .await
            .unwrap();
        until(&updates, &mut current, |model| model.ui.seats.len() == 1).await;
        let (&seat, metadata) = current.ui.seats.first_key_value().unwrap();
        assert_eq!(metadata.name, "generic shell");
        assert_eq!(metadata.harness.0, "zsh");
        let view = bbt_proto::View {
            seat: Ulid(seat.0.0),
            generation: metadata.process_generation,
            view_generation: Ulid::nil(),
        };
        sender
            .send(UiMsg::Watched(vec![(seat, view)]))
            .await
            .unwrap();
        until(&updates, &mut current, |model| {
            model.screens.contains_key(&seat) && model.frames.contains_key(&seat)
        })
        .await;
        input(&sender, seat, view.generation, b"echo hi\r").await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "hi")
        })
        .await;
        eprintln!("PASS real IO: standalone hi row in Replica, prepared frame published");
        let expected: String = (0..256)
            .map(|i| "0123456789abcdef".as_bytes()[i % 16] as char)
            .collect();
        let probe = format!(
            "printf 'BURST_READY\\n'; IFS= read -r payload; if [ \"$payload\" = '{expected}' ]; then printf 'BURST_ORDER_OK\\n'; else printf 'BURST_ORDER_BAD\\n'; fi\r"
        );
        input(&sender, seat, view.generation, probe.as_bytes()).await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "BURST_READY")
        })
        .await;
        tokio::time::timeout(Duration::from_secs(15), async {
            for byte in expected.bytes() {
                input(&sender, seat, view.generation, &[byte]).await;
            }
            input(&sender, seat, view.generation, b"\r").await;
        })
        .await
        .expect("full-duplex input burst remained blocked");
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "BURST_ORDER_OK")
        })
        .await;
        eprintln!(
            "PASS full duplex: 256 distinct input IDs delivered all bytes in exact order while frames/replies were drained"
        );
        sender
            .send(UiMsg::Resize {
                seat,
                generation: view.generation,
                cols: 40,
                rows: 12,
            })
            .await
            .unwrap();
        until(&updates, &mut current, |model| {
            (model.screens[&seat].cols, model.screens[&seat].rows) == (40, 12)
        })
        .await;
        input(&sender, seat, view.generation, b"stty size\r").await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "12 40")
        })
        .await;
        input(
            &sender,
            seat,
            view.generation,
            b"printf 'REFLOW_abcdefghijklmnopqrstuvwxyz0123456789\\n'\r",
        )
        .await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .concat()
                .contains("REFLOW_abcdefghijklmnopqrstuvwxyz0123456789")
        })
        .await;
        sender
            .send(UiMsg::Resize {
                seat,
                generation: view.generation,
                cols: 20,
                rows: 12,
            })
            .await
            .unwrap();
        until(&updates, &mut current, |model| {
            model.screens[&seat].cols == 20
                && lines(&model.screens[&seat])
                    .concat()
                    .contains("REFLOW_abcdefghijklmnopqrstuvwxyz0123456789")
        })
        .await;
        assert!(
            lines(&current.screens[&seat])
                .iter()
                .all(|line| line.len() <= 20)
        );
        input(&sender, seat, view.generation, b"stty size\r").await;
        until(&updates, &mut current, |model| {
            lines(&model.screens[&seat])
                .iter()
                .any(|line| line.trim() == "12 20")
        })
        .await;
        eprintln!("PASS resize: PTY stty reports 12x40 then 12x20; prior output reflows");
        assert!(
            tokio::process::Command::new("/bin/kill")
                .args(["-KILL", &logic_pid.to_string()])
                .status()
                .await
                .unwrap()
                .success()
        );
        let reason = tokio::time::timeout(Duration::from_secs(10), lost_receiver.recv())
            .await
            .unwrap()
            .unwrap();
        if let Some(next) = updates.take() {
            current = next;
        }
        let last_screen = current.screens[&seat].clone();
        let last_frame = current.frames[&seat].clone();
        input(
            &sender,
            seat,
            view.generation,
            b"queued after backend loss\r",
        )
        .await;
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert!(
            !task.is_finished(),
            "backend loss must not stop the app's IO owner"
        );
        assert!(updates.take().is_none());
        assert!(Arc::ptr_eq(&last_screen, &current.screens[&seat]));
        assert!(Arc::ptr_eq(&last_frame, &current.frames[&seat]));
        // Fill both bounded queues while disconnected. Dropping the UI owner
        // must still wake shutdown when command receive is backpressured.
        for _ in 1..(COMMAND_CAPACITY * 2) {
            input(&sender, seat, view.generation, b"queued while lost\r").await;
        }
        assert!(reason.is_some());
        eprintln!("PASS logic SIGKILL: {reason:?}; retained Screen/frame; no restoration claimed");
        drop(sender);
        tokio::time::timeout(Duration::from_secs(10), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        eprintln!(
            "PASS orderly shutdown at full retained/channel capacity after backend loss, supervisor {supervisor_pid} reaped"
        );
    }
}
