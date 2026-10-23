use super::*;
use bboxy450_term::{
    grids::Grids,
    model::{self, PanePreparer},
};
use bbt_mux::{
    LaunchSpec, Supervisor, Ulid,
    bus::{Consumer, Delivery},
    logic::{Logic, LogicConfig},
    store::events::Draft,
};
use bbt_proto::{GuiAction, GuiReply, GuiRequest, SeatRecord, Snapshot, Update, View};
use std::{collections::VecDeque, ffi::OsString, path::Path};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::UnixStream,
};

struct Core {
    logic: Logic,
    supervisor: Supervisor,
}
impl Core {
    #[allow(
        clippy::disallowed_methods,
        reason = "R5 bootstrap file IO runs on the blocking pool, never the UI"
    )]
    async fn open(config: &Config) -> Result<Self, String> {
        let root = config.run_dir.clone();
        bbt_rt::handle()
            .spawn_blocking(move || {
                bbt_rt::assert_not_ui_thread();
                std::fs::create_dir(&root).map_err(|e| format!("r5-fresh-run-dir: {e}"))?;
                let repository = root.join("repository");
                std::fs::create_dir_all(repository.join(".bboxy")).map_err(|e| e.to_string())?;
                let supervisor = Supervisor::start(bbt_mux::Config {
                    journal_directory: Some(root.join("journal")),
                    ..Default::default()
                })
                .map_err(|e| e.to_string())?;
                let connection = supervisor.connect().map_err(|e| e.to_string())?;
                let logic = Logic::start_custodied(
                    connection,
                    LogicConfig {
                        run_dir: root.join("logic"),
                        repository,
                        ring: Default::default(),
                    },
                    bbt_rt::handle(),
                )
                .map_err(|e| e.to_string())?;
                Ok(Self { logic, supervisor })
            })
            .await
            .map_err(|e| e.to_string())?
    }
    async fn close(self) -> Result<(), String> {
        bbt_rt::handle()
            .spawn_blocking(move || {
                let a = self.logic.shutdown().map_err(|e| e.to_string());
                let b = self.supervisor.shutdown().map_err(|e| e.to_string());
                a.and(b)
            })
            .await
            .map_err(|e| e.to_string())?
    }
}

pub async fn run(
    config: Config,
    updates: Arc<Latest<Arc<Model>>>,
    rendering: oneshot::Receiver<crate::live::RenderContext>,
    messages: UiSender,
    progress: Arc<Progress>,
    commands: mpsc::Receiver<Control>,
    mut stopped: oneshot::Receiver<()>,
) -> Result<Report, String> {
    let host_load_before = bbt_rt::fault::host_load_average().map_err(|e| e.to_string())?;
    eprintln!("R5 host load before: {host_load_before:?}");
    if config.frame_gate && host_load_before[0] > 8. {
        let reason = format!(
            "r5-timing-run-discarded: starting loadavg {} exceeds8",
            host_load_before[0]
        );
        tokio::fs::write(
            &config.report,
            serde_json::to_vec_pretty(&serde_json::json!({
                "discarded": true, "reason": reason, "host_load_before": host_load_before,
            }))
            .map_err(|e| e.to_string())?,
        )
        .await
        .map_err(|e| e.to_string())?;
        return Err(reason);
    }
    let mut result=async {
        disk_guard(&config.run_dir, config.real).await?;
        if config.real {
            let output=tokio::process::Command::new("sysctl").args(["-n","kern.tty.ptmx_max"]).output().await.map_err(|e|e.to_string())?;
            let maximum=String::from_utf8_lossy(&output.stdout).trim().parse::<usize>().map_err(|_|"r5-pty-ceiling-unavailable")?.saturating_sub(32);
            if config.seats>maximum {return Err(format!("r5-pty-capacity: requested{} exceeds host ceiling{maximum} with32margin",config.seats));}
        }
        let core=Core::open(&config).await?;
        let identity=core.logic.client.identity_status().await.map_err(|e|e.to_string())?;
        if identity.custody!=Some(bbt_mux::identity::Custody::Memory) {core.close().await?;return Err("r5-custody-witness-not-memory".into());}
        eprintln!("R5 identity witness: custody={:?} label={:?}",identity.custody,identity.custody_label);
        let outcome=tokio::select! {
            result=exercise(&config,&core,updates,rendering,messages,progress.clone(),commands)=>result,
            _=&mut stopped=>Err("r5-cancelled".into()),
        };
        let mut background_reaped=true;
        if let Some(task)=progress.background.get() {
            bbt_rt::cancel(task);
            let end=Instant::now()+Duration::from_secs(5);
            while task.status().is_none() && Instant::now()<end {tokio::time::sleep(Duration::from_millis(10)).await;}
            background_reaped=task.status().is_some();
        }
        let cleanup=core.close().await;
        let _=tokio::fs::remove_dir_all(&config.run_dir).await;
        cleanup?;
        if !background_reaped {return Err("r5-background-command-not-reaped".into());}
        outcome
    }.await;
    if let Ok(report) = &mut result {
        report.host_load_before = host_load_before;
        report.host_load_after = bbt_rt::fault::host_load_average().map_err(|e| e.to_string())?;
        let json = serde_json::to_vec_pretty(report).map_err(|e| e.to_string())?;
        tokio::fs::write(&config.report, json)
            .await
            .map_err(|e| e.to_string())?;
        println!("{}", report.gate_line(&config));
    }
    result
}

async fn disk_guard(path: &Path, real: bool) -> Result<(), String> {
    let parent = if path.exists() {
        path
    } else {
        path.parent().ok_or("r5-run-parent")?
    };
    let output = tokio::process::Command::new("df")
        .arg("-Pk")
        .arg(parent)
        .env("LC_ALL", "C")
        .output()
        .await
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("r5-disk-probe-failed".into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let free = text
        .lines()
        .last()
        .and_then(|l| l.split_whitespace().nth(3))
        .and_then(|s| s.parse::<u64>().ok())
        .ok_or("r5-disk-probe-invalid")?
        * 1024;
    // Real output is journaled; the synthetic fixture writes no PTY history.
    let minimum = if real {
        10 * 1024 * 1024 * 1024
    } else {
        512 * 1024 * 1024
    };
    if free < minimum {
        return Err(format!(
            "r5-disk-guard: {free}bytes below required{minimum}"
        ));
    }
    Ok(())
}

struct Wire {
    out: mpsc::Sender<GuiAction>,
    incoming: mpsc::Receiver<Result<GuiReply, String>>,
    reader: tokio::task::JoinHandle<()>,
    writer: tokio::task::JoinHandle<()>,
}
impl Drop for Wire {
    fn drop(&mut self) {
        self.reader.abort();
        self.writer.abort();
    }
}
impl Wire {
    async fn open(core: &Core) -> Result<Self, String> {
        let endpoint = core
            .logic
            .client
            .gui_endpoint()
            .await
            .map_err(|e| e.to_string())?;
        let stream = UnixStream::connect(endpoint.path)
            .await
            .map_err(|e| e.to_string())?;
        let (read, mut write) = stream.into_split();
        let (out, mut requests) = mpsc::channel(64);
        let (reply, incoming) = mpsc::channel(8);
        let reader = tokio::spawn(async move {
            let mut read = BufReader::new(read);
            loop {
                let parsed = match bbt_mux::seat_socket::read_line(&mut read).await {
                    Ok(Some(line)) => {
                        serde_json::from_slice::<GuiReply>(&line).map_err(|e| e.to_string())
                    }
                    Ok(None) => Err("r5-gui-eof".into()),
                    Err(e) => Err(e.to_string()),
                };
                let end = parsed.is_err();
                if reply.send(parsed).await.is_err() || end {
                    break;
                }
            }
        });
        let writer = tokio::spawn(async move {
            while let Some(action) = requests.recv().await {
                let request = GuiRequest {
                    version: 1,
                    capability: endpoint.capability,
                    action,
                };
                let Ok(mut bytes) = serde_json::to_vec(&request) else {
                    break;
                };
                bytes.push(b'\n');
                if write.write_all(&bytes).await.is_err() {
                    break;
                }
            }
        });
        Ok(Self {
            out,
            incoming,
            reader,
            writer,
        })
    }
}

fn publish(
    records: &[SeatRecord],
    grids: &Grids,
    previous: &mut Arc<Model>,
    preparer: &mut PanePreparer,
    updates: &Latest<Arc<Model>>,
) -> Result<(), String> {
    let mut next = model::build(records, grids, previous);
    // A fixture grid keeps the probe's entire row visible. The production
    // membership builder's one-row fallback would make 50 sliver panes.
    for swarm in &mut next.ui.window.sidebar.swarms {
        if swarm.id == bbt_ui_model::model::SwarmId::from_u128(1) {
            for tab in &mut swarm.tabs {
                tab.tree = fixture_grid(&tab.seats);
            }
        }
    }
    preparer.prepare(&mut next).map_err(|e| e.to_string())?;
    *previous = Arc::new(next);
    updates.publish(previous.clone());
    Ok(())
}

fn fixture_grid(seats: &[SeatId]) -> bbt_ui_model::model::SplitNode {
    use bbt_ui_model::model::{Axis, SplitNode};
    fn join(nodes: &[SplitNode], axis: Axis) -> SplitNode {
        if nodes.len() == 1 {
            return nodes[0].clone();
        }
        let half = nodes.len() / 2;
        SplitNode::split(
            axis,
            half as f32 / nodes.len() as f32,
            join(&nodes[..half], axis),
            join(&nodes[half..], axis),
        )
    }
    let columns = ((seats.len() as f64 * 1.5).sqrt().ceil() as usize).max(1);
    let rows: Vec<_> = seats
        .chunks(columns)
        .map(|row| {
            join(
                &row.iter().copied().map(SplitNode::Leaf).collect::<Vec<_>>(),
                Axis::Row,
            )
        })
        .collect();
    join(&rows, Axis::Column)
}

async fn exercise(
    config: &Config,
    core: &Core,
    updates: Arc<Latest<Arc<Model>>>,
    rendering: oneshot::Receiver<crate::live::RenderContext>,
    messages: UiSender,
    progress: Arc<Progress>,
    mut commands: mpsc::Receiver<Control>,
) -> Result<Report, String> {
    let rendering = rendering.await.map_err(|_| "r5-renderer-stopped")?;
    let mut options = bbt_render::cells::terminal::Options::default();
    options.font.scale = rendering.scale.round().max(1.) as u32;
    let mut preparer =
        PanePreparer::with_options(rendering.context, options).map_err(|e| e.to_string())?;
    let mut records = Vec::with_capacity(config.seats);
    let mut pids = Vec::new();
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    for i in 0..config.seats {
        if config.real && i % 25 == 0 {
            disk_guard(&config.run_dir, true).await?;
        }
        let mut spec = LaunchSpec::new(executable.to_string_lossy(), &config.run_dir);
        spec.args = vec![
            "--r5-producer".into(),
            config.run_dir.to_string_lossy().into_owned(),
        ];
        spec.size.cols = 24;
        spec.size.rows = 8;
        let id = if config.real {
            spec.seat
        } else {
            Ulid::from(i as u128 + 1)
        };
        let record = SeatRecord::new(
            id,
            Ulid::from(if i < config.watched { 1 } else { 2 }),
            format!(
                "{}-{i}",
                if config.real {
                    "r5-pty"
                } else {
                    "synthetic-state"
                }
            ),
            "zsh",
        );
        if config.real {
            let info = core
                .logic
                .client
                .spawn(record.clone(), spec)
                .await
                .map_err(|e| format!("r5-spawn-{i}: {e}"))?;
            pids.push(info.pid);
            eprintln!("R5 owned producer seat={} pid={}", record.id, info.pid);
            tokio::fs::write(
                config.run_dir.join("owned-pids.json"),
                serde_json::to_vec(&pids).map_err(|e| e.to_string())?,
            )
            .await
            .map_err(|e| e.to_string())?;
        }
        records.push(record);
    }
    progress.rows.store(records.len() as u64, Ordering::Release);
    if !config.real {
        struct FixtureWake(UiSender);
        impl bbt_rt::Waker for FixtureWake {
            fn wake(&self) {
                let _ = self.0.try_send(bbt_rt::UiMsg::Changed { source: 7 });
            }
        }
        let dirty = bbt_rt::DirtySet::new(config.seats, Arc::new(FixtureWake(messages.clone())));
        let values = (0..config.seats)
            .map(|id| Latest::new(dirty.source(id)))
            .collect();
        let _ = progress.synthetic.set(SyntheticStates {
            dirty,
            values,
            consumed: AtomicU64::new(0),
        });
    }
    let panes: Vec<_> = records
        .iter()
        .take(config.watched)
        .map(|r| {
            (
                SeatId::from_u128(r.id.0),
                View {
                    seat: r.id,
                    generation: 1,
                    view_generation: Ulid::new(),
                },
            )
        })
        .collect();
    let mut grids = Grids::new();
    let watches = grids.set_watched(&panes);
    let mut wire = if config.real {
        Some(Wire::open(core).await?)
    } else {
        None
    };
    let mut outgoing: VecDeque<_> = watches.into();
    let mut terms: Vec<_> = (0..config.watched)
        .map(|_| bbt_core::Term::new(24, 8, 0))
        .collect();
    let mut previous = Arc::new(Model::default());
    Arc::make_mut(&mut previous).ui.window.theme = rendering.theme;
    let mut sequence = 0;
    let mut warm = None;
    let mut measured = None;
    let mut observed_seconds = 0.;
    let mut side = None;
    let mut pending_probe = None;
    let mut stop_sent = false;
    let mut stopped_at = None;
    let mut last_disk = Instant::now();
    let start = Instant::now();
    let mut tick = tokio::time::interval(Duration::from_millis(5));
    let mut sides = SideStats::default();
    loop {
        if start.elapsed() > Duration::from_secs(240) {
            return Err("r5-setup-or-settle-timeout".into());
        }
        if stopped_at.is_some_and(|at: Instant| at.elapsed() > Duration::from_secs(20))
            && progress.phase() < IDLE
        {
            return Err("r5-unsettled-after-producers-stopped".into());
        }
        if config.real && last_disk.elapsed() >= Duration::from_secs(1) {
            disk_guard(&config.run_dir, true).await?;
            last_disk = Instant::now();
        }
        if previous.frames.len() == config.watched && warm.is_none() {
            tokio::fs::write(config.run_dir.join("go"), b"run")
                .await
                .map_err(|e| e.to_string())?;
            progress.set_phase(WARM);
            warm = Some(Instant::now());
        }
        if warm.is_some_and(|t| t.elapsed() >= Duration::from_secs(2)) && measured.is_none() {
            progress.set_phase(MEASURE);
            measured = Some(Instant::now());
            let client = core.logic.client.clone();
            let p = progress.clone();
            let sender = messages.clone();
            let exe = executable.clone();
            let secs = config.seconds;
            side = Some(AbortTask(tokio::spawn(async move {
                side_loads(client, p, sender, exe, secs).await
            })));
        }
        if measured.is_some_and(|t| t.elapsed().as_secs_f64() >= config.seconds) && !stop_sent {
            stop_sent = true;
            stopped_at = Some(Instant::now());
            tokio::fs::remove_file(config.run_dir.join("go"))
                .await
                .map_err(|e| e.to_string())?;
            // Producers stop before the idle witness. Their last snapshot is
            // an explicit PAUSED row, so a quiet interval cannot hide backlog.
            if let Some(task) = side.take() {
                sides = task.finish().await?;
            }
        }
        if stop_sent
            && previous.screens.len() == config.watched
            && previous
                .screens
                .values()
                .all(|screen| contains(screen, "R5PAUSED"))
            && progress.phase() == MEASURE
        {
            observed_seconds = measured.unwrap().elapsed().as_secs_f64();
            progress.set_phase(SETTLE);
        }
        let mut writer_blocked = false;
        if let Some(wire) = &wire {
            while let Some(action) = outgoing.pop_front() {
                match wire.out.try_send(action) {
                    Ok(()) => {}
                    Err(mpsc::error::TrySendError::Full(action)) => {
                        outgoing.push_front(action);
                        writer_blocked = true;
                        break;
                    }
                    Err(_) => return Err("r5-gui-writer-closed".into()),
                }
            }
        }
        let capacity = wire.as_ref().map(|w| w.out.clone());
        tokio::select! {
            biased;
            command=commands.recv()=>match command {
                Some(Control::Probe(id))=>{
                    pending_probe=Some(id);progress.probe.store(id,Ordering::Release);
                    if config.real {outgoing.push_back(GuiAction::Input {seat:panes[0].0,generation:1,id:Ulid::new(),bytes:format!("R5P{id:08X}\n").into_bytes()});}
                }
                Some(Control::Finish(metrics))=>{
                    let (failures,timing_failures)=assess(config,&metrics,&sides);
                    return Ok(Report {
                        kind:if config.real {"real-pty"} else {"synthetic-render-state"},
                        rows:records.len(), real_ptys:pids.len(), watched:config.watched,
                        seconds:config.seconds, observed_seconds, metrics,
                        stdout_bytes:sides.stdout_bytes,
                        bus_published:sides.bus_published, bus_delivered:sides.bus_delivered,
                        bus_hz:sides.bus_hz, bus_seconds:sides.bus_seconds,
                        fault_read_ms:sides.fault_ms,
                        synthetic_states_consumed:progress.synthetic.get().map_or(0,|s|s.consumed.load(Ordering::Relaxed)),
                        owned_producer_pids:pids.clone(),
                        background_pid:progress.background.get().and_then(bbt_rt::TaskId::pid),
                        host_load_before:[0.;3], host_load_after:[0.;3],
                        timing_gated:config.frame_gate, timing_failures, failures,
                        unproven:vec!["1000 real PTYs on this host","120Hz native panel",
                            "physical keystroke/scanout/photon",
                            "exhaustive framework/OS lock and IO tracing","quinn non-answering peer"],
                    });
                }
                None=>return Err("r5-ui-commands-closed".into()),
            },
            reply=async {wire.as_mut().unwrap().incoming.recv().await},if wire.is_some()=>{
                let reply=reply.ok_or("r5-gui-reader-closed")??;
                match reply {
                    GuiReply::Update {seat,update}=>{
                        let (view,sequence)=match &update {Update::Snapshot {snapshot}=>(snapshot.view,snapshot.sequence),Update::Delta {delta}=>(delta.view,delta.sequence)};
                        if grids.apply(seat,&update).map_err(|e|e.to_string())?.is_some(){publish(&records,&grids,&mut previous,&mut preparer,&updates)?;}
                        progress.updates.fetch_add(1,Ordering::Relaxed);
                        outgoing.push_back(GuiAction::Applied {seat,view,sequence});
                    }
                    GuiReply::Error {reason,..}=>return Err(format!("r5-gui-error: {reason}")),
                    _=>{},
                }
            },
            _=async {drop(capacity.unwrap().reserve_owned().await);},if writer_blocked=>{},
            _=tick.tick()=>{
                if !config.real && progress.phase()<SETTLE {
                    sequence+=1;
                    if let Some(states)=progress.synthetic.get(){for value in &states.values{value.publish(sequence);}}
                    for (i,((seat,view),term)) in panes.iter().zip(&mut terms).enumerate() {
                        let marker=if stop_sent{"R5PAUSED".to_owned()}else if i==0 && pending_probe.is_some(){format!("R5P{:08X}{}",pending_probe.unwrap(),if sequence.is_multiple_of(2){'|'}else{'/'})}else{format!("R5 {:04X}",sequence & 0xffff)};
                        term.advance(format!("\x1b[H\x1b[2J{marker}\r\nsynthetic {i} revision {sequence}\r\n").as_bytes());
                        grids.apply(*seat,&Update::Snapshot {snapshot:Box::new(Snapshot {view:*view,sequence,screen:bbt_proto::Screen::from_term(term,String::new(),None,None)})}).map_err(|e|e.to_string())?;
                    }
                    progress.updates.fetch_add(config.seats as u64,Ordering::Relaxed);
                    publish(&records,&grids,&mut previous,&mut preparer,&updates)?;
                }
            },
        }
    }
}

#[derive(Default)]
struct SideStats {
    stdout_bytes: u64,
    bus_published: u64,
    bus_delivered: u64,
    bus_hz: f64,
    bus_seconds: f64,
    fault_ms: f64,
}

fn assess(
    config: &Config,
    metrics: &Measurements,
    sides: &SideStats,
) -> (Vec<String>, Vec<String>) {
    let mut failures = Vec::new();
    if metrics.encoded_changes.len() != config.watched
        || metrics.encoded_changes.values().any(|n| *n < 2)
    {
        failures.push("non-advancing-watched-pane".into());
    }
    if metrics.idle_frames != 0 || metrics.idle_callbacks != 0 {
        failures.push("idle-frame-or-callback".into());
    }
    if metrics.guard_violations != 0 {
        failures.push("ui-blocking-guard-violation".into());
    }
    if sides.stdout_bytes != 1_000_000_000 {
        failures.push("stdout-not1GB".into());
    }
    if sides.fault_ms < 2000. {
        failures.push("file-read-not-stalled2s".into());
    }
    if metrics.probe_clipped {
        failures.push("probe-glyphs-clipped-by-pane".into());
    }
    let mut timing = Vec::new();
    if metrics.frames < 30 {
        timing.push("insufficient-measured-frames".into());
    }
    if metrics.cpu_p99_ms > 4. {
        timing.push("ui-cpu-p99-over4ms".into());
    }
    if metrics
        .input_to_gpu_complete_ms
        .is_none_or(|ms| ms > metrics.input_budget_ms)
    {
        timing.push("input-to-gpu-over-two-display-intervals".into());
    }
    if sides.bus_hz < 10_000. {
        timing.push("bus-below10k-events-per-second".into());
    }
    if config.budget.is_some_and(|b| metrics.frame_work_p99_ms > b) {
        timing.push("frame-budget-exceeded".into());
    }
    if config.frame_gate {
        failures.extend(timing.clone());
    }
    (failures, timing)
}
struct AbortTask(tokio::task::JoinHandle<Result<SideStats, String>>);
impl Drop for AbortTask {
    fn drop(&mut self) {
        self.0.abort();
    }
}
impl AbortTask {
    async fn finish(mut self) -> Result<SideStats, String> {
        (&mut self.0).await.map_err(|e| e.to_string())?
    }
}
async fn side_loads(
    client: bbt_mux::logic::LogicClient,
    progress: Arc<Progress>,
    messages: UiSender,
    executable: PathBuf,
    seconds: f64,
) -> Result<SideStats, String> {
    let task = bbt_rt::spawn_command(
        vec![executable.into_os_string(), OsString::from("--r5-stdout")],
        std::env::temp_dir(),
        vec![],
    );
    let _ = progress.background.set(task.clone());
    let spawn_deadline = Instant::now() + Duration::from_secs(5);
    while task.pid().is_none() && task.status().is_none() {
        if Instant::now() > spawn_deadline {
            return Err("r5-background-spawn-timeout".into());
        }
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    if let Some(pid) = task.pid() {
        eprintln!("R5 owned background stdout pid={pid}");
    }
    let read = tokio::spawn(bbt_rt::fault::stalled_file_read(Duration::from_secs(2)));
    let consumer = client
        .bus
        .subscribe(Consumer::gui("seat/r5/output"))
        .await
        .map_err(|e| e.to_string())?;
    let start = Instant::now();
    let mut published = 0;
    let mut delivered = 0;
    while start.elapsed().as_secs_f64() < seconds {
        let target = (start.elapsed().as_secs_f64() * 10_000.) as u64 + 128;
        while published < target && start.elapsed().as_secs_f64() < seconds {
            client
                .bus
                .publish(
                    "r5",
                    Draft::new(
                        "seat/r5/output",
                        "r5-fixture",
                        serde_json::json!({"revision":published}),
                    ),
                )
                .await
                .map_err(|e| e.to_string())?;
            published += 1;
        }
        for item in client
            .bus
            .poll(&consumer, 256)
            .await
            .map_err(|e| e.to_string())?
        {
            if matches!(item, Delivery::Event { .. }) {
                delivered += 1;
            }
        }
        progress.bus_events.store(published, Ordering::Release);
        progress.bus_delivered.store(delivered, Ordering::Release);
        let _ = messages.try_send(bbt_rt::UiMsg::Changed { source: 6 });
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let bus_seconds = start.elapsed().as_secs_f64();
    let bus_hz = published as f64 / bus_seconds;
    let fault = read
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    progress
        .fault_ns
        .store(fault.elapsed.as_nanos() as u64, Ordering::Release);
    let deadline = Instant::now() + Duration::from_secs(10);
    while task.status().is_none() {
        if Instant::now() > deadline {
            bbt_rt::cancel(&task);
            return Err("r5-stdout-timeout".into());
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let status = task.status().unwrap();
    if status.code != Some(0) || status.error.is_some() {
        return Err(format!("r5-stdout-failed: {status:?}"));
    }
    progress
        .stdout_bytes
        .store(status.stdout_bytes, Ordering::Release);
    client
        .bus
        .unsubscribe(&consumer)
        .await
        .map_err(|e| e.to_string())?;
    Ok(SideStats {
        stdout_bytes: status.stdout_bytes,
        bus_published: published,
        bus_delivered: delivered,
        bus_hz,
        bus_seconds,
        fault_ms: fault.elapsed.as_secs_f64() * 1000.,
    })
}

#[allow(
    clippy::disallowed_methods,
    reason = "R5 helper child processes do blocking fixture IO; they never register a UI thread"
)]
pub fn private_entry(args: &[String]) -> Option<Result<(), String>> {
    use std::io::{BufRead, Write};
    if args.first().is_some_and(|s| s == "--r5-stdout") {
        return Some((|| {
            let mut out = std::io::stdout().lock();
            let block = vec![b'x'; 64 * 1024];
            let mut left = 1_000_000_000;
            while left > 0 {
                let n = left.min(block.len());
                out.write_all(&block[..n]).map_err(|e| e.to_string())?;
                left -= n;
            }
            Ok(())
        })());
    }
    if !args.first().is_some_and(|s| s == "--r5-producer") {
        return None;
    }
    Some((|| {
        let root = PathBuf::from(args.get(1).ok_or("r5-producer-root-required")?);
        let probe = Arc::new(AtomicU64::new(0));
        let input = probe.clone();
        std::thread::spawn(move || {
            for line in std::io::stdin().lock().lines().map_while(Result::ok) {
                if let Some(id) = line
                    .strip_prefix("R5P")
                    .and_then(|s| u64::from_str_radix(s.trim(), 16).ok())
                {
                    input.store(id, Ordering::Release);
                }
            }
        });
        let mut stdout = std::io::stdout().lock();
        let block = "y\r\n".repeat(2048);
        let mut sequence = 0u64;
        let mut last = Instant::now() - Duration::from_secs(1);
        let mut running = false;
        let mut bytes = 0u64;
        stdout
            .write_all(b"\x1b[H\x1b[2J\x1b[2;rR5READY\r\n")
            .map_err(|e| e.to_string())?;
        loop {
            if last.elapsed() >= Duration::from_millis(100) {
                running = root.join("go").exists();
                last = Instant::now();
                if !running {
                    stdout
                        .write_all(b"\x1b7\x1b[1;1HR5PAUSED\x1b[K\x1b8")
                        .map_err(|e| e.to_string())?;
                }
            }
            if running {
                sequence += 1;
                let id = probe.load(Ordering::Acquire);
                let header = if id == 0 {
                    format!("R5 {:04X}", sequence & 0xffff)
                } else {
                    format!(
                        "R5P{id:08X}{}",
                        if sequence.is_multiple_of(2) { '|' } else { '/' }
                    )
                };
                let prefix = format!("\x1b7\x1b[1;1H{header}\x1b[K\x1b8");
                stdout
                    .write_all(prefix.as_bytes())
                    .and_then(|_| stdout.write_all(block.as_bytes()))
                    .map_err(|e| e.to_string())?;
                bytes += (prefix.len() + block.len()) as u64;
            } else {
                stdout.flush().map_err(|e| e.to_string())?;
                std::thread::sleep(Duration::from_millis(20));
            }
            if sequence.is_multiple_of(4096) && sequence > 0 {
                let _ = std::fs::write(
                    root.join(format!("producer-{}.json", std::process::id())),
                    format!(
                        "{{\"pid\":{},\"written_bytes\":{bytes}}}",
                        std::process::id()
                    ),
                );
            }
        }
    })())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_row_fits_every_fifteen_and_fifty_pane_fixture() {
        for count in [15, 50] {
            let seats: Vec<_> = (1..=count).map(SeatId::from_u128).collect();
            let tree = fixture_grid(&seats);
            let solved = bbt_ui_model::layout::solve_split(
                &tree,
                bbt_ui_model::layout::Rect::new(0., 0., 1280., 860.),
                4.,
            );
            assert_eq!(solved.panes.len(), count as usize);
            for (_, rect) in solved.panes {
                assert!(rect.w >= 120. && rect.h >= 100., "{count}: {rect:?}");
            }
        }
    }

    #[test]
    fn functional_ci_reports_timing_failures_and_the_slow_switch_enforces_them() {
        let mut config = Config {
            real: false,
            headless: true,
            frame_gate: false,
            seats: 1000,
            watched: 15,
            seconds: 3.,
            idle: 1.,
            report: PathBuf::new(),
            run_dir: PathBuf::new(),
            budget: Some(4.),
        };
        let metrics = Measurements {
            frames: 30,
            cpu_p99_ms: 5.,
            frame_work_p99_ms: 5.,
            input_to_gpu_complete_ms: Some(30.),
            input_budget_ms: 2000. / 120.,
            encoded_changes: (0..15).map(|id| (id.to_string(), 2)).collect(),
            ..Default::default()
        };
        let sides = SideStats {
            stdout_bytes: 1_000_000_000,
            fault_ms: 2000.,
            bus_hz: 9999.,
            ..Default::default()
        };
        let (functional, timing) = assess(&config, &metrics, &sides);
        assert!(functional.is_empty());
        assert_eq!(timing.len(), 4);
        config.frame_gate = true;
        let (gated, same_timing) = assess(&config, &metrics, &sides);
        assert_eq!(same_timing, timing);
        assert_eq!(gated, timing);
    }
}
