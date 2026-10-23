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
        }
        let pending = !inbox.messages.is_empty() || !inbox.output.is_empty();
        if let Some(report) = controller.poll(frame, 0, pending, 120.)? {
            return if report.failures.is_empty() {
                Ok(())
            } else {
                Err(format!("r5-gate-failed: {:?}", report.failures))
            };
        }
        let mut phases = Phases::default();
        let drain = Instant::now();
        let dirty = !inbox.dirty.drain().is_empty();
        wake.store(false, Ordering::Release);
        controller.drain();
        while drain.elapsed() < Duration::from_millis(1) {
            let a = inbox.messages.try_recv();
            let b = inbox.output.pop();
            if a.is_none() && b.is_none() {
                break;
            }
        }
        let changed = if let Some(next) = updates.take() {
            model = next;
            scene.set_model(&model, &model.ui.window.stage);
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
        controller.frame(frame, &model, &bounds, phases, cpu);
    }
}
