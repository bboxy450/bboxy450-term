// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded P1 fixture: synthetic sidebar metadata, actual History owners and GPU frames.
#![allow(clippy::disallowed_methods)] // Isolated CLI fixture, own worker and scheduler.
use crate::{gpu::VerificationFrame, scene::Scene};
use bboxy450_term::{
    grids::Grids,
    model::{self, Model, PanePreparer},
    scrollback::{History, Request},
};
use bbt_mux::{Ulid, registry::SeatRecord};
use bbt_proto::{GuiAction, SeatId, Snapshot, Update, View, gui::HistoryRecord};
use bbt_rt::{HitchMeter, Phases};
use std::{
    collections::BTreeMap,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};

struct Sample {
    model: Arc<Model>,
    heap: usize,
    pages: u64,
}
fn worker(
    context: Arc<bbt_render::device::DeviceContext>,
    commands: mpsc::Receiver<u64>,
    replies: mpsc::SyncSender<Result<Sample, String>>,
) {
    let result = (|| -> Result<(), String> {
        let records: Vec<_> = (1..=1000)
            .map(|i| {
                SeatRecord::new(
                    Ulid(i),
                    Ulid(if i <= 50 { 1 } else { 2 }),
                    format!("P1 row {i}"),
                    "zsh",
                )
            })
            .collect();
        let term = bbt_core::Term::new_with_spare_rows(24, 8, 0, 0);
        let screen = Arc::new(bbt_proto::Screen::from_term(
            &term,
            String::new(),
            None,
            None,
        ));
        let mut grids = Grids::new();
        let views: Vec<_> = (1..=50)
            .map(|i| {
                (
                    SeatId::from_u128(i),
                    View {
                        seat: Ulid(i),
                        generation: 1,
                        view_generation: Ulid::new(),
                    },
                )
            })
            .collect();
        grids.set_watched(&views);
        for (seat, view) in &views {
            grids
                .apply(
                    *seat,
                    &Update::Snapshot {
                        snapshot: Box::new(Snapshot {
                            view: *view,
                            sequence: 1,
                            screen: screen.as_ref().clone(),
                        }),
                    },
                )
                .map_err(|e| e.to_string())?;
        }
        let bytes: Vec<u8> = (0..800)
            .flat_map(|n| format!("P1 row {n:04}\r\n").into_bytes())
            .collect();
        let mut history = History::default();
        let mut displays = BTreeMap::new();
        let mut previous = Model::default();
        let mut preparer =
            PanePreparer::with_options(context, Default::default()).map_err(|e| e.to_string())?;
        for row in commands {
            for (seat, view) in &views {
                history
                    .request(*seat, *view, 1, screen.clone(), Request::row(row))
                    .map_err(str::to_owned)?;
            }
            let mut turns = 0;
            loop {
                turns += 1;
                if turns > 100_000 {
                    return Err("p1_history_did_not_settle".into());
                }
                if let Some(GuiAction::GetHistory { seat, from, to }) =
                    history.next_action(Instant::now())
                {
                    if (from, to) != (1, 1) {
                        return Err("p1_history_cursor_changed".into());
                    }
                    history
                        .accept(
                            seat,
                            vec![HistoryRecord::Bytes {
                                sequence: 1,
                                bytes: bytes.clone(),
                            }],
                            None,
                        )
                        .map_err(str::to_owned)?;
                }
                if let Some((seat, display)) = history.step() {
                    displays.insert(
                        seat,
                        display
                            .map_err(str::to_owned)?
                            .ok_or("p1_history_returned_live")?,
                    );
                }
                if !history.has_work()
                    && displays.len() == 50
                    && displays.values().all(|d| d.top_row == row)
                {
                    break;
                }
            }
            if displays.len() != 50 || displays.values().any(|d| d.top_row != row) {
                return Err("p1_history_projection_incomplete".into());
            }
            let metrics = history.metrics(&displays);
            let heap = history.retained_heap_bytes().ok_or("p1_heap_unavailable")?;
            if heap > 50 * bboxy450_term::scrollback::HEAP_BUDGET
                || metrics["panes"].as_array().unwrap().iter().any(|p| {
                    p["retained_history_bytes"].as_u64().unwrap()
                        > p["budget_bytes"].as_u64().unwrap()
                })
            {
                return Err("p1_history_heap_over_budget".into());
            }
            let pages = metrics["panes"]
                .as_array()
                .unwrap()
                .iter()
                .map(|p| p["pages_fetched"].as_u64().unwrap())
                .sum();
            let mut next = model::build(&records, &grids, &previous);
            for swarm in &mut next.ui.window.sidebar.swarms {
                if swarm.id == bbt_ui_model::SwarmId::from_u128(1) {
                    for tab in &mut swarm.tabs {
                        tab.tree = grid(&tab.seats);
                    }
                }
            }
            next.history = displays.clone();
            preparer.prepare(&mut next).map_err(|e| e.to_string())?;
            previous = next.clone();
            if replies
                .send(Ok(Sample {
                    model: Arc::new(next),
                    heap,
                    pages,
                }))
                .is_err()
            {
                return Ok(());
            }
        }
        Ok(())
    })();
    if let Err(reason) = result {
        let _ = replies.send(Err(reason));
    }
}
// Adapted from bboxy450-term src/stall/load.rs::fixture_grid: same visible-pane geometry.
// https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
fn grid(seats: &[SeatId]) -> bbt_ui_model::model::SplitNode {
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
    let rows: Vec<_> = seats
        .chunks(9)
        .map(|row| {
            join(
                &row.iter().copied().map(SplitNode::Leaf).collect::<Vec<_>>(),
                Axis::Row,
            )
        })
        .collect();
    join(&rows, Axis::Column)
}

pub fn run() -> Result<serde_json::Value, String> {
    if !cfg!(debug_assertions) || std::env::var("BBOXY_TEST_CUSTODY").as_deref() != Ok("memory") {
        return Err("p1_private_fixture_required".into());
    }
    let mut gpu = bbt_rt::handle().block_on(VerificationFrame::new())?;
    let gpu_errors = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let errors = gpu_errors.clone();
    gpu.context()
        .device
        .on_uncaptured_error(Arc::new(move |error| {
            if errors.fetch_add(1, std::sync::atomic::Ordering::Relaxed) < 4 {
                eprintln!("P1 GPU validation: {error}");
            }
        }));
    gpu.context()
        .device
        .set_device_lost_callback(|reason, message| {
            eprintln!("P1 device lost: {reason:?}: {message}");
        });
    let (send, commands) = mpsc::sync_channel(1);
    let (replies, receive) = mpsc::sync_channel(1);
    let context = gpu.context();
    let owner = std::thread::spawn(move || worker(context, commands, replies));
    send.send(0).map_err(|e| e.to_string())?;
    let mut sample = receive
        .recv_timeout(Duration::from_secs(30))
        .map_err(|e| e.to_string())??;
    if sample.model.ui.seats.len() != 1000 || sample.model.history.len() != 50 {
        return Err("p1_fixture_dimensions_unavailable".into());
    }
    let mut scene = Scene::live();
    let mut meter = HitchMeter::new(120.);
    let (complete, completed) = mpsc::sync_channel(64);
    let mut frame = 0;
    let mut max_heap = sample.heap;
    let mut hitches = 0;
    let mut paging_frames = 0;
    let mut phase_samples = Vec::new();
    let mut max_render_passes = 0;
    let mut gpu_completed = 0;
    let mut deadline = Instant::now();
    bbt_rt::register_ui_thread();
    // Warm the actual pane/chrome paths before the measured idle and paging interval.
    for round in 0..10 {
        if round >= 2 {
            send.try_send((round - 1) * 64).map_err(|e| e.to_string())?;
            let until = Instant::now() + Duration::from_secs(10);
            loop {
                match receive.try_recv() {
                    Ok(next) => {
                        sample = next?;
                        break;
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        return Err("p1_history_worker_stopped".into());
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
                if Instant::now() >= until {
                    return Err("p1_paging_timeout".into());
                }
                std::thread::park_timeout(Duration::from_secs_f64(1. / 120.));
            }
        }
        std::thread::park_timeout(deadline.saturating_duration_since(Instant::now()));
        deadline = Instant::now() + Duration::from_secs_f64(1. / 120.);
        let start = Instant::now();
        if round == 2 && std::env::var("BBOXY_P1_BROKEN").as_deref() == Ok("1") {
            std::thread::sleep(Duration::from_millis(20));
        }
        scene.set_model(&sample.model, &sample.model.ui.window.stage);
        let layouts = scene.layout([1600., 900.], false)?;
        let bounds = scene.pane_bounds(&layouts);
        if bounds.iter().filter(|(_, r)| r.w > 0. && r.h > 0.).count() != 50 {
            return Err("p1_watched_panes_not_visible".into());
        }
        let layout = start.elapsed();
        let start = Instant::now();
        // Match the native composition: chrome first, terminal cells, then
        // live rims/HUD. All three remain inside the measured frame phases.
        let (ops, mut overlay) =
            bbt_ui::components::split_rims(scene.paint(&layouts, [1600., 900.], None, None));
        let hud = crate::perf_hud(meter.perf_record(Instant::now(), 0.));
        let hud_ops = scene.overlay([1600., 900.], Some(&hud));
        if hud_ops.is_empty() {
            return Err("p1_hud_unpainted".into());
        }
        overlay.extend(hud_ops);
        let draw = start.elapsed();
        frame += 1;
        let encode = gpu.draw_layers_notifying(
            ops,
            crate::gpu::Panes {
                model: &sample.model,
                bounds: &bounds,
                focused: sample.model.ui.window.stage.focused,
                overlay,
            },
            frame,
            450.,
            complete.clone(),
            || {},
        )?;
        let phases = Phases {
            layout,
            draw,
            encode,
            ..Default::default()
        };
        max_render_passes = max_render_passes.max(bbt_render::passes::take().passes());
        if round >= 2 {
            phase_samples.push([
                layout.as_secs_f64() * 1000.,
                draw.as_secs_f64() * 1000.,
                encode.as_secs_f64() * 1000.,
            ]);
            paging_frames += 1;
            hitches += usize::from(meter.record(Instant::now(), phases).is_some());
        }
        max_heap = max_heap.max(sample.heap);
        while completed.try_recv().is_ok() {
            gpu_completed += 1;
        }
    }
    let idle_frames = meter.total_frames();
    let idle_hitches = hitches;
    let idle = Instant::now();
    let mut ticks = 0;
    while idle.elapsed() < Duration::from_secs(1) {
        std::thread::park_timeout(Duration::from_secs_f64(1. / 120.));
        ticks += 1;
        while completed.try_recv().is_ok() {
            gpu_completed += 1;
        }
        if receive.try_recv().is_ok() {
            return Err("p1_idle_model_update".into());
        }
    }
    drop(send);
    // All commands have completed; this join does no IO/frame work.
    owner.join().map_err(|_| "p1_owner_panicked")?;
    let record = meter.perf_record(Instant::now(), 0.);
    let gpu_errors = gpu_errors.load(std::sync::atomic::Ordering::Relaxed);
    Ok(
        serde_json::json!({"basis":"synthetic sidebar metadata; actual 50 GUI History owners, pane preparation, native Scene/HUD and offscreen GPU; not a 1000-PTY or display-cadence claim",
        "gpu_validation_errors":gpu_errors,"rows":sample.model.ui.seats.len(),"watched":sample.model.history.len(),"configured_hz":120,
        "paging_frames":paging_frames,"paging_hitches":hitches,"idle_hitches":hitches-idle_hitches,"idle_frames":meter.total_frames()-idle_frames,
        "idle_ticks":ticks,"gpu_completed":gpu_completed,"submitted_frames":frame,"cpu_p99_ms":record.cpu_p99,
        "retained_heap_bytes":max_heap,"heap_budget_bytes":50*bboxy450_term::scrollback::HEAP_BUDGET,
        "pages_fetched":sample.pages,"max_render_passes":max_render_passes,"phase_samples_ms_layout_draw_encode":phase_samples,"passed":gpu_errors==0 && hitches==0 && gpu_completed==frame && ticks>=60 && paging_frames==8 && meter.total_frames()==idle_frames}),
    )
}
