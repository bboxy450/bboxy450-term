// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Windowless R5 consumer; only the display-deadline park is outside frames.
use super::*;
use crate::{gpu::VerificationFrame, scene::Scene};

#[allow(
    clippy::disallowed_methods,
    reason = "offline GPU bootstrap completes before registering the R5 UI thread"
)]
fn bootstrap() -> Result<VerificationFrame, String> {
    bbt_rt::handle().block_on(VerificationFrame::new())
}

pub fn run(config: Config) -> Result<(), String> {
    let mut gpu = bootstrap()?;
    struct Wake(Arc<AtomicBool>);
    impl bbt_rt::Waker for Wake {
        fn wake(&self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let wake = Arc::new(AtomicBool::new(false));
    let inbox = bbt_rt::init_ui(Wake(wake.clone()));
    let updates = Arc::new(Latest::new(inbox.dirty.source(5)));
    let (context, rendering) = oneshot::channel();
    context
        .send(crate::live::RenderContext {
            context: gpu.context(),
            scale: 1.,
            theme: bbt_ui_model::tokens::Theme::Dark,
        })
        .ok()
        .ok_or("r5-headless-render-context")?;
    let poll = wake.clone();
    let mut controller = Native::start(
        config.clone(),
        updates.clone(),
        rendering,
        inbox.sender.clone(),
        move || {
            poll.store(true, Ordering::Release);
        },
    );
    let (completed, completions) = std::sync::mpsc::sync_channel(256);
    let mut scene = Scene::live();
    let mut model = Arc::new(Model::default());
    let mut frame = 0;
    let mut deadline = Instant::now();
    let start = Instant::now();
    loop {
        if start.elapsed() > Duration::from_secs(300) {
            controller.cancel();
            return Err("r5-headless-deadline".into());
        }
        // This models the display scheduler's idle wait, never an IO wait in
        // an encoded frame. It holds no UI state lock and consumes no input.
        std::thread::park_timeout(deadline.saturating_duration_since(Instant::now()));
        deadline = Instant::now() + Duration::from_secs_f64(1. / 120.);
        while let Ok((id, at)) = completions.try_recv() {
            controller.gpu_complete(id, at);
            c1_gpu(id, controller.input_ms, controller.progress.phase());
        }
        let pending = !inbox.messages.is_empty() || !inbox.output.is_empty();
        if C1_OVERLAP
            .get()
            .is_some_and(|shared| shared.aborted.load(Ordering::Acquire))
        {
            controller.cancel();
        }
        if let Some(report) = controller.poll(frame, 0, pending, 120.)? {
            return if report.failures.is_empty() {
                Ok(())
            } else {
                Err(format!("r5-gate-failed: {:?}", report.failures))
            };
        }
        let mut phases = Phases::default();
        let drain = Instant::now();
        let dirty = inbox
            .dirty
            .drain()
            .iter()
            .any(|source| !matches!(*source, 0 | 1));
        wake.store(false, Ordering::Release);
        controller.drain();
        while drain.elapsed() < Duration::from_millis(1) {
            let a = inbox.messages.try_recv();
            let b = inbox.output.pop();
            if a.is_none() && b.is_none() {
                break;
            }
        }
        inbox.messages.complete_drain();
        inbox.output.complete_drain();
        let changed = if let Some(next) = updates.take() {
            controller.model_received(&next);
            model = next;
            scene.set_model(&model, &model.ui.window.stage);
            controller.model_applied();
            true
        } else {
            false
        };
        phases.drain = drain.elapsed();
        if !dirty && !changed {
            continue;
        }
        let begin = Instant::now();
        let layouts = scene.layout([1600., 900.], false)?;
        let bounds = scene.pane_bounds(&layouts);
        phases.layout = begin.elapsed();
        let begin = Instant::now();
        let ops = scene.paint(&layouts, [1600., 900.], None, None);
        phases.draw = begin.elapsed();
        frame += 1;
        phases.encode = gpu.draw(ops, &model, &bounds, frame, completed.clone())?;
        let cpu = bbt_rt::fault::thread_cpu_time().map_err(|e| e.to_string())?;
        let elapsed = controller
            .last_cpu
            .map(|previous| cpu.saturating_sub(previous));
        controller.frame(frame, &model, &bounds, phases, cpu, 1.);
        c1_encoded(frame, elapsed, controller.progress.phase());
    }
}

// Private integration hook for the existing R5 Native/render/Core owner. It is
// opt-in, debug-only, memory-custodied and confined to a fresh fixture HOME/run.
// Only atomics and fixed slots are touched from the registered rendering owner.
const C1_FRAME_SLOTS: usize = 4096;
struct C1Overlap {
    active: AtomicBool,
    stopped: AtomicBool,
    aborted: AtomicBool,
    encoded_ids: [AtomicU64; C1_FRAME_SLOTS],
    encoded_cpu: [AtomicU64; C1_FRAME_SLOTS],
    completed_cpu: [AtomicU64; C1_FRAME_SLOTS],
    completed: AtomicU64,
    input_ns: AtomicU64,
    overflow: AtomicBool,
}
static C1_OVERLAP: std::sync::OnceLock<Arc<C1Overlap>> = std::sync::OnceLock::new();
pub(super) struct C1JoinGuard(Arc<C1Overlap>);
impl Drop for C1JoinGuard {
    fn drop(&mut self) {
        self.0.active.store(false, Ordering::Release);
        self.0.stopped.store(true, Ordering::Release);
    }
}
fn c1_encoded(frame: u64, cpu: Option<Duration>, phase: u8) {
    let Some(shared) = C1_OVERLAP.get() else {
        return;
    };
    if phase != MEASURE || !shared.active.load(Ordering::Acquire) {
        return;
    }
    let Some(cpu) = cpu else {
        return;
    };
    let slot = frame as usize % C1_FRAME_SLOTS;
    shared.encoded_cpu[slot].store(
        cpu.as_nanos().min(u64::MAX as u128) as u64,
        Ordering::Relaxed,
    );
    shared.encoded_ids[slot].store(frame, Ordering::Release);
}
fn c1_gpu(frame: u64, input: Option<f64>, phase: u8) {
    let Some(shared) = C1_OVERLAP.get() else {
        return;
    };
    if phase != MEASURE || !shared.active.load(Ordering::Acquire) {
        return;
    }
    let slot = frame as usize % C1_FRAME_SLOTS;
    if shared.encoded_ids[slot]
        .compare_exchange(frame, 0, Ordering::AcqRel, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    // Only the UI receiver writes completed; the evidence owner reads after
    // release. Capacity overflow fails the witness rather than dropping samples.
    let index = shared.completed.load(Ordering::Relaxed) as usize;
    if index >= C1_FRAME_SLOTS {
        shared.overflow.store(true, Ordering::Release);
        return;
    }
    shared.completed_cpu[index].store(
        shared.encoded_cpu[slot].load(Ordering::Relaxed),
        Ordering::Relaxed,
    );
    shared.completed.store(index as u64 + 1, Ordering::Release);
    if let Some(ms) = input {
        let ns = (ms * 1_000_000.).round().clamp(1., u64::MAX as f64) as u64;
        let _ = shared
            .input_ns
            .compare_exchange(0, ns, Ordering::Release, Ordering::Relaxed);
    }
}

/// Called once, after the existing memory-custody assertion and before exercise.
/// The returned guard belongs to the exact Core whose GUI endpoint is exported.
/// Ordinary R5 invocations execute no extra fixture work.
pub(super) async fn join_c1(
    config: &Config,
    client: bbt_mux::logic::LogicClient,
) -> Result<Option<C1JoinGuard>, String> {
    let Some(directory) = std::env::var_os("BBOXY_DREAM_C1_NATIVE_JOIN") else {
        return Ok(None);
    };
    if !cfg!(debug_assertions)
        || !config.real
        || !config.headless
        || !config.frame_gate
        || std::env::var("BBOXY_TEST_CUSTODY").as_deref() != Ok("memory")
    {
        return Err("c1-native-private-headless-memory-admission-required".into());
    }
    let endpoint = client.gui_endpoint().await.map_err(|e| e.to_string())?;
    let directory = PathBuf::from(directory);
    let run_dir = config.run_dir.clone();
    // This isolated bootstrap owns all filesystem, process-identity and receipt IO.
    // Its bounded handshake completes before R5's measurement interval begins.
    let state = tokio::task::spawn_blocking(move || {
        use std::{fs, io::Write, os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt}};
        let home = PathBuf::from(std::env::var_os("HOME").ok_or("c1-native-private-home-required")?);
        let home = home.canonicalize().map_err(|e| e.to_string())?;
        let root = home.parent().ok_or("c1-native-private-home-parent-required")?;
        if directory != root.join("run") || directory.exists()
            || !run_dir.starts_with(root) || run_dir.canonicalize().map_err(|e|e.to_string())? != run_dir
            || fs::symlink_metadata(root).map_err(|e|e.to_string())?.permissions().mode() & 0o077 != 0 {
            return Err("c1-native-fresh-private-run-required".into());
        }
        fs::DirBuilder::new().mode(0o700).create(&directory).map_err(|e|e.to_string())?;
        let repository = run_dir.join("repository");
        let git = std::process::Command::new("/usr/bin/git").args(["init","-q"]).arg(&repository)
            .env_clear().env("HOME",&home).env("PATH","/usr/bin:/bin").output().map_err(|e|e.to_string())?;
        if !git.status.success() { return Err("c1-native-private-source-repository-init".into()); }
        let write_new = |path: &std::path::Path, value: &serde_json::Value| -> Result<(),String> {
            let mut file = fs::OpenOptions::new().write(true).create_new(true).mode(0o600).open(path).map_err(|e|e.to_string())?;
            file.write_all(&serde_json::to_vec(value).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
            file.sync_all().map_err(|e|e.to_string())
        };
        let hex = endpoint.capability.iter().map(|v|format!("{v:02x}")).collect::<String>();
        write_new(&directory.join("gui.endpoint"), &serde_json::json!({"path":endpoint.path,"capability":hex}))?;
        write_new(&directory.join("c1-runtime-path.json"), &serde_json::json!({"path":run_dir.join("logic")}))?;
        let ready_path = directory.join("c1-ready.json");
        let end = Instant::now()+Duration::from_secs(60);
        let ready = loop {
            if directory.join("c1-abort").is_file() { return Err("c1-native-private-job-handshake-aborted".into()); }
            if Instant::now() >= end { return Err("c1-native-private-job-handshake-deadline".into()); }
            match fs::symlink_metadata(&ready_path) {
                Ok(meta) => {
                    if !meta.is_file() || meta.len()>4096 || meta.permissions().mode() & 0o077 != 0 {
                        return Err("c1-native-private-job-handshake-refused".into());
                    }
                    let value:serde_json::Value=serde_json::from_slice(&fs::read(&ready_path).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
                    break value;
                }
                Err(e) if e.kind()==std::io::ErrorKind::NotFound => std::thread::sleep(Duration::from_millis(10)),
                Err(e) => return Err(e.to_string()),
            }
        };
        let pid = ready["worker_pid"].as_u64().and_then(|v|u32::try_from(v).ok()).ok_or("c1-native-private-worker-pid-required")?;
        let stamp=ready["worker_start"].as_str().ok_or("c1-native-private-worker-start-required")?.to_owned();
        if bbt_mux::adapters::dream_c1::process_stamp(pid).map_err(|e|format!("{e:?}"))? != stamp {
            return Err("c1-native-private-worker-identity-changed".into());
        }
        let shared=Arc::new(C1Overlap {
            active:AtomicBool::new(true),stopped:AtomicBool::new(false),aborted:AtomicBool::new(false),
            encoded_ids:std::array::from_fn(|_|AtomicU64::new(0)),
            encoded_cpu:std::array::from_fn(|_|AtomicU64::new(0)),
            completed_cpu:std::array::from_fn(|_|AtomicU64::new(0)),
            completed:AtomicU64::new(0),input_ns:AtomicU64::new(0),overflow:AtomicBool::new(false),
        });
        C1_OVERLAP.set(shared.clone()).map_err(|_|"c1-native-only-one-private-render-owner")?;
        let monitor=shared.clone();
        std::thread::Builder::new()
            .name("c1-native-evidence".into())
            .spawn(move || {
                let result = (|| -> Result<(), String> {
                    let finish = directory.join("c1-finish-overlap");
                    let deadline = Instant::now() + Duration::from_secs(60);
                    let mut published = false;
                    while !monitor.stopped.load(Ordering::Acquire) {
                        if directory.join("c1-abort").is_file() {
                            monitor.active.store(false, Ordering::Release);
                            monitor.aborted.store(true, Ordering::Release);
                            return Ok(());
                        }
                        if Instant::now() >= deadline {
                            return Err("c1-native-overlap-deadline".into());
                        }
                        if !published {
                            // Nonallocating kernel identity lookup: no process spawn,
                            // hash, parse, filesystem work or locks on the UI owner.
                            if bbt_mux::adapters::dream_c1::process_stamp(pid).map_err(|e|format!("{e:?}"))? != stamp {
                                return Err("c1-native-worker-exited-during-measured-overlap".into());
                            }
                            let count = monitor.completed.load(Ordering::Acquire);
                            let progress = serde_json::json!({"gpu_frames":count,"worker_pid":pid,"same_logic_owner":true});
                            let temporary = directory.join("c1-overlap-progress.tmp");
                            {
                                let mut file = fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&temporary).map_err(|e|e.to_string())?;
                                file.write_all(&serde_json::to_vec(&progress).unwrap()).map_err(|e|e.to_string())?;
                            }
                            fs::rename(temporary, directory.join("c1-overlap-progress.json")).map_err(|e|e.to_string())?;
                        }
                        if !published && finish.is_file() {
                            monitor.active.store(false, Ordering::Release);
                            if bbt_mux::adapters::dream_c1::process_stamp(pid).map_err(|e|format!("{e:?}"))? != stamp {
                                return Err("c1-native-worker-incarnation-changed-at-overlap-cut".into());
                            }
                            // A rendering callback already admitted before the cut
                            // can publish its final fixed slot; no UI wait is added.
                            std::thread::sleep(Duration::from_millis(20));
                            let count = monitor.completed.load(Ordering::Acquire) as usize;
                            let mut cpu = (0..count.min(C1_FRAME_SLOTS))
                                .map(|i| monitor.completed_cpu[i].load(Ordering::Relaxed) as f64 / 1_000_000.)
                                .collect::<Vec<_>>();
                            cpu.sort_by(f64::total_cmp);
                            let p99 = cpu.get((cpu.len() * 99).div_ceil(100).saturating_sub(1)).copied();
                            let input = monitor.input_ns.load(Ordering::Acquire);
                            let value = serde_json::json!({
                                "schema":"bboxy.dream.c1.native-overlap/1","same_logic_owner":true,
                                "worker_pid":pid,"worker_start":stamp,"run_id":ready["run_id"],
                                "source_sha256":ready["source_sha256"],"gpu_frames":count,
                                "cpu_p99_ms":p99,"cpu_samples_ms":cpu,
                                "input_to_gpu_ms":(input>0).then_some(input as f64/1_000_000.),
                                "overflow":monitor.overflow.load(Ordering::Acquire)});
                            write_new(&directory.join("c1-overlap.json"), &value)?;
                            published = true;
                        }
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    if published { Ok(()) } else { Err("c1-native-owner-ended-before-overlap-settlement".into()) }
                })();
                monitor.active.store(false, Ordering::Release);
                if let Err(error) = result {
                    let _ = write_new(&directory.join("c1-overlap-error.json"), &serde_json::json!({"error":error}));
                }
            })
            .map_err(|e|e.to_string())?;
        Ok(C1JoinGuard(shared))
    }).await.map_err(|e|e.to_string())??;
    Ok(Some(state))
}
