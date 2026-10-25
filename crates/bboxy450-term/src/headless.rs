//! The real GUI consumer and renderer without winit, AppKit or an NSWindow.
//! Only a model change, app request or GPU completion wakes this owner.
use crate::{UiEvent, gpu::VerificationFrame, live, log::Logger};
use bboxy450_term::model::Model;
use bbt_rt::Latest;
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const SIZE: [f32; 2] = [1600., 900.];
const STARTUP: Duration = Duration::from_secs(30);

#[allow(
    clippy::disallowed_methods,
    reason = "offscreen device bootstrap runs before registering the UI owner"
)]
fn bootstrap() -> Result<VerificationFrame, String> {
    bbt_rt::handle().block_on(VerificationFrame::new())
}

struct Wake {
    thread: std::thread::Thread,
    callbacks: Arc<AtomicU64>,
}
struct PendingCapture {
    ask: live::CaptureAsk,
    after: u64,
}
impl bbt_rt::Waker for Wake {
    fn wake(&self) {
        self.callbacks.fetch_add(1, Ordering::Relaxed);
        self.thread.unpark();
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    let record_notifications = crate::notification_recording(args)?;
    let value = |flag| args.windows(2).find(|v| v[0] == flag).map(|v| v[1].clone());
    let run_dir = value("--run-dir").map(std::path::PathBuf::from);
    let smoke = args.iter().any(|v| v == "--smoke");
    let ready_line = args.iter().any(|v| v == "--app-ready");
    let capture_path = value("--capture").map(std::path::PathBuf::from);
    let test_account = bboxy450_term::cockpit::Config::test_account_from_args(args)
        .map_err(|error| error.to_string())?;
    let unattended = test_account.is_some();
    let verify = crate::Verify::from_args(args)?;
    let mut gpu = bootstrap()?;
    let thread = std::thread::current();
    let callbacks = Arc::new(AtomicU64::new(0));
    let overflow = Arc::new(AtomicBool::new(false));
    let inbox = bbt_rt::init_ui(Wake {
        thread: thread.clone(),
        callbacks: callbacks.clone(),
    });
    let updates = Arc::new(Latest::new(inbox.dirty.source(5)));
    let (sender, events) = mpsc::sync_channel(64);
    let sink: Arc<dyn Fn(UiEvent) + Send + Sync> = {
        let thread = thread.clone();
        let overflow = overflow.clone();
        let callbacks = callbacks.clone();
        Arc::new(move |event| {
            callbacks.fetch_add(1, Ordering::Relaxed);
            if sender.try_send(event).is_err() {
                overflow.store(true, Ordering::Release);
            }
            thread.unpark();
        })
    };
    let logger = Logger::new();
    let (context, rendering) = tokio::sync::oneshot::channel();
    context
        .send(live::RenderContext {
            context: gpu.context(),
            scale: 1.,
            theme: verify.theme.unwrap_or(bbt_ui_model::tokens::Theme::Dark),
        })
        .map_err(|_| "headless render context closed")?;
    let commands = live::start_with_sink(
        updates.clone(),
        sink.clone(),
        logger.clone(),
        run_dir,
        args.iter().any(|v| v == "--ctl"),
        rendering,
    );
    let signals = sink.clone();
    bbt_rt::handle().spawn(async move {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut stop) = signal(SignalKind::terminate()) else {
            return;
        };
        let Ok(mut interrupt) = signal(SignalKind::interrupt()) else {
            return;
        };
        tokio::select! { _ = stop.recv() => {}, _ = interrupt.recv() => {} }
        signals(UiEvent::App(live::AppEvent::Stop {
            id: bbt_mux::Ulid::nil(),
        }));
    });
    let mut app = crate::App::owner(
        crate::UiProxy::Headless(sink.clone()),
        inbox,
        logger.clone(),
        updates.clone(),
        args,
        verify,
    );
    app.live_commands = Some(commands);
    if record_notifications {
        app.install_recording()?;
    }
    let settings_path = value("--settings-config")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            test_account
                .as_ref()
                .map(|account| account.join("config.toml"))
        })
        .map_or_else(bboxy450_term::settings::default_path, Ok)?;
    let settings_events = sink.clone();
    app.preferences_io = Some(bboxy450_term::settings::Io::start(
        settings_path,
        bbt_rt::handle(),
        move |snapshot| settings_events(UiEvent::Settings(snapshot)),
    ));
    let (completed, completions) = mpsc::sync_channel::<(u64, Instant)>(64);
    let mut model = Arc::new(Model::default());
    let mut captures = VecDeque::new();
    let mut submitted = 0;
    let mut rendered = 0;
    let mut ui_frame = None;
    let mut rendered_at = None;
    let mut backend = false;
    let mut generation = 0;
    let mut custody = "unknown".to_string();
    let mut dirty = false;
    // The offscreen window's logical size; `ctl app resize` changes it.
    let mut size = SIZE;
    let mut ready_sent = false;
    let mut closing = false;
    let mut closing_at: Option<Instant> = None;
    let mut finished = false;
    let mut capacity_wait = false;
    let mut result = Ok(());
    let mut idle = crate::IdleQuiescence::default();
    let mut idle_epoch = None;
    let mut next_idle = Instant::now() + Duration::from_secs(1);
    let started = Instant::now();
    logger.write("headless GUI starting: windows=0, real Session, offscreen renderer");
    let pumped = (|| -> Result<(), String> {
        loop {
            if overflow.swap(false, Ordering::AcqRel) && !closing {
                result = Err("headless event queue full".into());
                app.live_commands.take();
                closing = true;
            }
            while let Ok(event) = events.try_recv() {
                match event {
                    UiEvent::Wake => capacity_wait = false,
                    UiEvent::CommandsReady(ready) => app.commands_ready(ready),
                    UiEvent::BackendInfo {
                        generation: value,
                        custody_backend,
                    } => {
                        generation = value;
                        custody = custody_backend;
                    }
                    UiEvent::BackendReady => backend = true,
                    UiEvent::BackendLost(reason) => {
                        backend = false;
                        logger.write(format!("backend lost: {reason}"));
                    }
                    UiEvent::CaptureFor(ask) => {
                        if captures.len() < 64 {
                            captures.push_back(PendingCapture {
                                ask,
                                after: submitted + 1,
                            });
                            dirty = true;
                        } else {
                            result = Err("headless capture queue full".into());
                            app.live_commands.take();
                            closing = true;
                        }
                    }
                    UiEvent::Settings(snapshot) => {
                        if app.preferences.accept(&snapshot) {
                            app.apply_preferences();
                            dirty = true;
                        }
                    }
                    UiEvent::Notification(bboxy450_desktop_notifications::Response::Clicked(
                        id,
                    )) => app.notification_clicked(id),
                    UiEvent::Notification(bboxy450_desktop_notifications::Response::Dismissed(
                        _,
                    )) => {}
                    UiEvent::NotificationReady => {
                        if let Some(worker) = &mut app.notifications {
                            worker.flush();
                        }
                    }
                    UiEvent::NotificationError(reason) => logger.write(reason),
                    UiEvent::UiFor {
                        id,
                        request,
                        prior_spawns,
                    } => {
                        dirty |= app.begin_ui(id, request, size, submitted, prior_spawns);
                    }
                    UiEvent::UiDeadline(id) => app.ui_deadline(id),
                    UiEvent::FocusFor(ask) => {
                        app.begin_focus(ask);
                    }
                    UiEvent::FocusDeadline(id) => {
                        if let Some(answer) = app.ctl_focus.expire(id) {
                            app.answer_focus(answer);
                        }
                    }
                    UiEvent::CloseFor { id, seat, undo } => {
                        app.begin_close(id, seat, undo, size, submitted);
                    }
                    UiEvent::Capture => {
                        if let Some(path) = &capture_path {
                            captures.push_back(PendingCapture {
                                ask: live::CaptureAsk {
                                    id: bbt_mux::Ulid::new(),
                                    path: path.clone(),
                                    seat: None,
                                },
                                after: submitted + 1,
                            });
                            dirty = true;
                        }
                    }
                    UiEvent::App(live::AppEvent::Ask { id, request }) => {
                        if let bbt_proto::gui::AppRequest::Resize { cols, rows } = request {
                            size = crate::cell_grid_size(&model, cols, rows, 1.);
                            gpu.resize([size[0] as u32, size[1] as u32]);
                            dirty = true;
                        }
                        let mut status = crate::app_status(
                            "headless",
                            unattended,
                            backend,
                            rendered > 0,
                            &model,
                            generation,
                            rendered,
                            0,
                        );
                        status.custody_backend = custody.clone();
                        if request == bbt_proto::gui::AppRequest::Notifications {
                            status.notifications = app.recording_status();
                        }
                        crate::queue_pending(
                            &mut app.pending_commands,
                            live::UiMsg::AppReply { id, status },
                        )
                        .map_err(|_| "headless lifecycle admission full")?;
                    }
                    UiEvent::App(live::AppEvent::Stop { id }) => {
                        logger.write(format!("headless stop {id}: orderly Session shutdown"));
                        app.pending_commands.clear();
                        app.live_commands.take();
                        closing = true;
                    }
                    UiEvent::BackendStopped(stopped) => {
                        if result.is_ok() {
                            result = stopped;
                        }
                        finished = true;
                        logger.finish_with_sink(
                            format!("headless GUI stopped: {result:?}"),
                            sink.clone(),
                        );
                    }
                    UiEvent::Quit if finished => return result,
                    _ => {}
                }
            }
            while let Ok((frame, at)) = completions.try_recv() {
                rendered = rendered.max(frame);
                rendered_at = Some(
                    crate::now_ms()
                        .saturating_sub(at.elapsed().as_millis().try_into().unwrap_or(u64::MAX)),
                );
                if ui_frame.as_ref().is_some_and(|(id, _)| *id <= frame) {
                    let (id, layout) = ui_frame.take().expect("completed UI frame");
                    app.present_ui(id, layout);
                }
            }
            let sources = app.drain();
            if sources.iter().any(|source| !matches!(*source, 0 | 1)) {
                dirty = true;
            }
            dirty |= app.redraw_pending.replace(false);
            app.redraw_queued.set(false);
            if let Some(next) = &app.live_state
                && !Arc::ptr_eq(next, &model)
            {
                model = next.clone();
                dirty = true;
            }
            if !closing {
                dirty |= app.poll_focus_frame(rendered, submitted + 1);
                dirty |= app.poll_ui_frame(submitted);
                // A copy is submitted before any later draw, so its metadata names
                // the exact completed texture being read back, even under a flood.
                if submitted == rendered && rendered > 0 {
                    while captures
                        .front()
                        .is_some_and(|queued| queued.after <= rendered)
                    {
                        let ask = captures.pop_front().expect("capture front checked").ask;
                        let crop = match ask.seat {
                            None => Ok(None),
                            Some(seat) => app
                                .pane_bounds
                                .iter()
                                .find(|(id, _)| *id == seat)
                                .map(|(_, r)| {
                                    Some(crate::capture::Crop {
                                        x: r.x.max(0.) as u32,
                                        y: r.y.max(0.) as u32,
                                        width: r.w.max(1.) as u32,
                                        height: r.h.max(1.) as u32,
                                    })
                                })
                                .ok_or_else(|| {
                                    format!(
                                        "not_on_screen: seat {seat} has no pane on screen in frame {rendered}"
                                    )
                                }),
                        };
                        let send = app.live_commands.as_ref().unwrap().sender_clone();
                        let frame = bbt_proto::gui::CaptureFrame {
                            offscreen: true,
                            frame_seq: rendered,
                            rendered_at_ms: rendered_at,
                            ..Default::default()
                        };
                        match crop {
                            Ok(crop) => {
                                gpu.capture(ask.path, crop, logger.clone(), move |answer| {
                                    bbt_rt::handle().spawn(async move {
                                        let _ = send
                                            .send(live::UiMsg::Captured {
                                                id: ask.id,
                                                error: answer.err(),
                                                frame,
                                            })
                                            .await;
                                    });
                                })
                            }
                            Err(error) => {
                                crate::queue_pending(
                                    &mut app.pending_commands,
                                    live::UiMsg::Captured {
                                        id: ask.id,
                                        error: Some(error),
                                        frame,
                                    },
                                )
                                .map_err(|_| "headless capture admission full")?;
                            }
                        }
                    }
                }
                if dirty && submitted == rendered {
                    app.project_scene(true);
                    let layouts = app.solve_ui(size)?;
                    app.focus_layout_ready();
                    let (next, changed) = crate::pane_controls(
                        &model,
                        &app.pane_bounds,
                        &app.watched,
                        &app.pane_sizes,
                        app.scene.focused(),
                        1.,
                        app.verify.grid_size,
                    );
                    if next != app.watched {
                        app.watched = next.clone();
                        crate::queue_pending(&mut app.pending_commands, live::UiMsg::Watched(next))
                            .map_err(|_| "headless watch admission full")?;
                    }
                    app.pane_sizes
                        .retain(|seat, _| app.pane_bounds.iter().any(|(id, _)| id == seat));
                    for (seat, (generation, cols, rows)) in changed {
                        app.pane_sizes.insert(seat, (generation, cols, rows));
                        crate::queue_pending(
                            &mut app.pending_commands,
                            live::UiMsg::Resize {
                                seat,
                                generation,
                                cols,
                                rows,
                            },
                        )
                        .map_err(|_| "headless resize admission full")?;
                    }
                    submitted += 1;
                    let wake = thread.clone();
                    let callbacks = callbacks.clone();
                    let mut base = app.scene.paint(&layouts, size, None, None);
                    if let Some(page) = &app.local_page_layout {
                        base.extend(page.paint());
                    }
                    let foreground = app.scene.foreground_with_document(
                        size,
                        app.launcher_layout.as_ref(),
                        None,
                        app.conversation_layout.as_ref(),
                        app.document_layout.as_ref(),
                    );
                    let layout = app.ui_observe.then(|| app.ui_layout(size, &layouts));
                    gpu.draw_layers_notifying(
                        base,
                        foreground,
                        &model,
                        &app.pane_bounds,
                        submitted,
                        app.clock_frame.time_ms,
                        completed.clone(),
                        move || {
                            callbacks.fetch_add(1, Ordering::Relaxed);
                            wake.unpark();
                        },
                    )?;
                    ui_frame = layout.map(|layout| (submitted, layout));
                    dirty = false;
                }
                if let Some(send) = &app.live_commands {
                    while let Some(message) = app.pending_commands.pop_front() {
                        match send.try_send(message) {
                            Ok(()) => {}
                            Err(tokio::sync::mpsc::error::TrySendError::Full(message)) => {
                                app.pending_commands.push_front(message);
                                break;
                            }
                            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                                result = Err("headless IO owner closed".into());
                                closing = true;
                                break;
                            }
                        }
                    }
                }
                if rendered > 0 && backend && app.live_state.is_some() && !ready_sent {
                    ready_sent = true;
                    let mut status = crate::app_status(
                        "headless", unattended, backend, true, &model, generation, rendered, 0,
                    );
                    status.custody_backend = custody.clone();
                    logger.write(format!(
                        "headless render ready: generation={generation} frame={rendered} windows=0"
                    ));
                    if ready_line {
                        crate::write_app_ready(status);
                    }
                    if capture_path.is_some() {
                        let wake = sink.clone();
                        let delay = app.verify.capture_after.unwrap_or(Duration::ZERO);
                        bbt_rt::handle().spawn(async move {
                            tokio::time::sleep(delay).await;
                            wake(UiEvent::Capture);
                        });
                    }
                }
                if !ready_sent && started.elapsed() >= STARTUP {
                    result = Err("headless render readiness timed out".into());
                    app.live_commands.take();
                    closing = true;
                }
                if smoke && started.elapsed() >= STARTUP && !closing {
                    result = Err(idle.deadline_error());
                    app.live_commands.take();
                    closing = true;
                }
                if smoke && Instant::now() >= next_idle && ready_sent {
                    let counts = (submitted, callbacks.load(Ordering::Relaxed));
                    if let Some(epoch) = idle_epoch.take() {
                        let (start, frames, wakes) = idle.finish(epoch).expect("own idle epoch");
                        let passed = backend && counts == (frames, wakes);
                        logger.write(format!(
                            "idle {}: frames={} callbacks={} seconds={:.3} windows=0 mode=headless",
                            if passed { "passed" } else { "FAILED" },
                            counts.0 - frames,
                            counts.1 - wakes,
                            start.elapsed().as_secs_f64()
                        ));
                        if !passed {
                            result = Err("headless idle probe produced work".into());
                        }
                        app.live_commands.take();
                        closing = true;
                    } else if idle.ready(
                        true,
                        dirty
                            || submitted != rendered
                            || !app.pending_commands.is_empty()
                            || !captures.is_empty(),
                        counts,
                    ) {
                        idle_epoch = Some(idle.begin(Instant::now(), counts));
                        next_idle = Instant::now() + Duration::from_secs(10);
                        logger.write("idle probe begins: mode=headless windows=0");
                    } else {
                        next_idle = Instant::now() + Duration::from_secs(1);
                    }
                }
            }
            if !app.pending_commands.is_empty() && !closing && !capacity_wait {
                // Capacity is signalled by IO, not polled. Keep replies and controls
                // in their original bounded admission order.
                let send = app.live_commands.as_ref().unwrap().sender_clone();
                let wake = sink.clone();
                capacity_wait = true;
                bbt_rt::handle().spawn(async move {
                    if let Ok(permit) = send.reserve_owned().await {
                        drop(permit);
                        wake(UiEvent::Wake);
                    }
                });
            }
            if closing
                && !finished
                && closing_at.get_or_insert_with(Instant::now).elapsed() >= Duration::from_secs(30)
            {
                return Err("headless orderly shutdown timed out".into());
            }
            let deadline = if closing && !finished {
                Some(*closing_at.get_or_insert_with(Instant::now) + Duration::from_secs(30))
            } else if !closing && smoke {
                Some(next_idle)
            } else if !ready_sent && !closing {
                Some(started + STARTUP)
            } else {
                None
            };
            if let Some(deadline) = deadline {
                std::thread::park_timeout(deadline.saturating_duration_since(Instant::now()));
            } else {
                std::thread::park();
            }
        }
    })();
    // Rendering/admission errors also close through the IO owner. Returning
    // directly here would strand the two children after a GUI-side failure.
    if !finished {
        app.live_commands.take();
        let deadline = closing_at.unwrap_or_else(Instant::now) + Duration::from_secs(30);
        loop {
            while let Ok(event) = events.try_recv() {
                if let UiEvent::BackendStopped(stopped) = event {
                    return pumped.and(stopped);
                }
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "{}; orderly shutdown timed out",
                    pumped.err().unwrap_or_default()
                ));
            }
            std::thread::park_timeout(deadline.saturating_duration_since(Instant::now()));
        }
    }
    pumped
}
