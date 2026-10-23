//! Memory-only frame samples. Logging and drawing belong to the app.
use std::collections::VecDeque;
use std::time::{Duration, Instant};

pub const FRAME_SAMPLES: usize = 1024;

#[derive(Clone, Copy, Debug, Default)]
pub struct Phases {
    pub drain: Duration,
    pub layout: Duration,
    pub draw: Duration,
    pub encode: Duration,
    pub present_wait: Duration,
}
impl Phases {
    pub fn cpu(self) -> Duration {
        self.drain + self.layout + self.draw + self.encode
    }
    pub fn slowest(self) -> (&'static str, Duration) {
        [
            ("drain", self.drain),
            ("layout", self.layout),
            ("draw", self.draw),
            ("encode", self.encode),
            ("present_wait", self.present_wait),
        ]
        .into_iter()
        .max_by_key(|(_, time)| *time)
        .unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FrameSample {
    pub interval: Option<Duration>,
    pub phases: Phases,
}

pub struct HitchMeter {
    samples: VecDeque<FrameSample>,
    previous: Option<Instant>,
    total: u64,
    display_interval: Duration,
}

/// Frames the perf overlay shows (lab/web/perf.html, C.perfHud).
pub const HUD_FRAMES: usize = 120;
/// An interval longer than this many display intervals ends an idle stretch.
pub const IDLE_INTERVALS: u32 = 4;

/// One frame of the overlay's sparkline: its phases in ms, or an idle stub.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum HudFrame {
    Busy {
        phases: [f64; 5],
        /// The phase that overran, when this frame was late.
        hitch: Option<&'static str>,
    },
    Idle,
}

/// The perf overlay's record, in the lab's names (C.perfHud): ms values are f64.
#[derive(Clone, Debug, PartialEq)]
pub struct PerfRecord {
    pub hz: u32,
    /// Median frame interval over the window (idle gaps excluded).
    pub interval_ms: f64,
    /// The display interval: a frame is late past 1.5 x this.
    pub budget_ms: f64,
    pub cpu_p50: f64,
    pub cpu_p99: f64,
    /// Mean GPU active time per frame, supplied by the host.
    pub gpu_ms: f64,
    /// Per-phase p50 over the window: drain, layout, draw, encode, present_wait.
    pub phases: [f64; 5],
    /// Oldest first, at most HUD_FRAMES.
    pub frames: Vec<HudFrame>,
    /// The latest late frame in the window: (phase, its ms, lifetime frame number).
    pub hitch: Option<(&'static str, f64, u64)>,
    /// Late frames in the window, by the meter's rule.
    pub late: u32,
    /// Time since the last frame, once it exceeds the idle threshold.
    pub idle_ms: Option<f64>,
}

pub const PHASE_NAMES: [&str; 5] = ["drain", "layout", "draw", "encode", "present_wait"];

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.
}

fn phase_ms(p: Phases) -> [f64; 5] {
    [
        ms(p.drain),
        ms(p.layout),
        ms(p.draw),
        ms(p.encode),
        ms(p.present_wait),
    ]
}

fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.;
    }
    values.sort_by(f64::total_cmp);
    let index = ((values.len() as f64 * p).ceil() as usize).saturating_sub(1);
    values[index.min(values.len() - 1)]
}
impl HitchMeter {
    pub fn new(hz: f64) -> Self {
        assert!(hz.is_finite() && hz > 0.);
        Self {
            samples: VecDeque::with_capacity(FRAME_SAMPLES),
            previous: None,
            total: 0,
            display_interval: Duration::from_secs_f64(1. / hz),
        }
    }
    /// Idle intervals remain visible in samples, but do not count as CPU hitches.
    pub fn record(&mut self, now: Instant, phases: Phases) -> Option<(&'static str, Duration)> {
        let interval = self
            .previous
            .replace(now)
            .map(|previous| now.saturating_duration_since(previous));
        if self.samples.len() == FRAME_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(FrameSample { interval, phases });
        self.total += 1;
        self.late(phases).then(|| phases.slowest())
    }
    fn late(&self, phases: Phases) -> bool {
        phases.cpu().max(phases.present_wait) > self.display_interval.mul_f64(1.5)
    }
    /// The overlay's record over the last HUD_FRAMES samples. `now` decides
    /// idleness since the last frame; `gpu_ms` is the host's GPU mean.
    pub fn perf_record(&self, now: Instant, gpu_ms: f64) -> PerfRecord {
        let idle_after = self.display_interval * IDLE_INTERVALS;
        let skip = self.samples.len().saturating_sub(HUD_FRAMES);
        let first_frame = self.total - self.samples.len() as u64 + 1;
        let mut frames = Vec::with_capacity(HUD_FRAMES + 8);
        let (mut intervals, mut cpu) = (Vec::new(), Vec::new());
        let mut phases: [Vec<f64>; 5] = Default::default();
        let (mut hitch, mut late) = (None, 0);
        for (i, sample) in self.samples.iter().enumerate().skip(skip) {
            match sample.interval {
                Some(gap) if gap > idle_after => frames.push(HudFrame::Idle),
                Some(gap) => intervals.push(ms(gap)),
                None => {}
            }
            let p = phase_ms(sample.phases);
            for (k, v) in p.iter().enumerate() {
                phases[k].push(*v);
            }
            cpu.push(ms(sample.phases.cpu()));
            let over = self.late(sample.phases).then(|| sample.phases.slowest());
            if let Some((phase, time)) = over {
                late += 1;
                hitch = Some((phase, ms(time), first_frame + i as u64));
            }
            frames.push(HudFrame::Busy {
                phases: p,
                hitch: over.map(|(phase, _)| phase),
            });
        }
        // Idle stubs can push the oldest frames out of view.
        let excess = frames.len().saturating_sub(HUD_FRAMES);
        frames.drain(..excess);
        let budget_ms = ms(self.display_interval);
        let interval_ms = if intervals.is_empty() {
            budget_ms
        } else {
            percentile(&mut intervals, 0.5)
        };
        let idle_ms = self
            .previous
            .map(|last| now.saturating_duration_since(last))
            .filter(|gap| *gap > idle_after)
            .map(ms);
        PerfRecord {
            hz: (1000. / interval_ms).round() as u32,
            interval_ms,
            budget_ms,
            cpu_p50: percentile(&mut cpu.clone(), 0.5),
            cpu_p99: percentile(&mut cpu, 0.99),
            gpu_ms,
            phases: phases.map(|mut v| percentile(&mut v, 0.5)),
            frames,
            hitch,
            late,
            idle_ms,
        }
    }
    pub fn total_frames(&self) -> u64 {
        self.total
    }
    pub fn samples(&self) -> &VecDeque<FrameSample> {
        &self.samples
    }
    pub fn cpu_percentile(&self, percentile: f64) -> Duration {
        if self.samples.is_empty() {
            return Duration::ZERO;
        }
        let mut times: Vec<_> = self
            .samples
            .iter()
            .map(|sample| sample.phases.cpu())
            .collect();
        times.sort_unstable();
        let index =
            ((times.len() as f64 * percentile.clamp(0., 1.)).ceil() as usize).saturating_sub(1);
        times[index.min(times.len() - 1)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn busy(layout_ms: u64) -> Phases {
        Phases {
            layout: Duration::from_millis(layout_ms),
            encode: Duration::from_millis(1),
            ..Default::default()
        }
    }
    #[test]
    fn perf_record_windows_percentiles_and_late_frames() {
        let mut meter = HitchMeter::new(120.);
        let start = Instant::now();
        let mut t = start;
        for i in 0..150u64 {
            t += Duration::from_micros(8333);
            // Every 50th frame overruns layout (14 ms + 1 ms > 1.5 x 8.33).
            meter.record(t, busy(if i % 50 == 49 { 14 } else { 2 }));
        }
        let r = meter.perf_record(t, 2.9);
        assert_eq!(r.frames.len(), HUD_FRAMES);
        assert_eq!(r.hz, 120);
        assert!((r.budget_ms - 8.333).abs() < 0.001);
        assert!((r.interval_ms - 8.333).abs() < 0.001);
        assert_eq!(r.gpu_ms, 2.9);
        assert_eq!(r.phases[1], 2.);
        assert_eq!(r.cpu_p50, 3.);
        // The window holds lifetime frames 31..=150: frames 50, 100 and 150 are late.
        assert_eq!(r.late, 3);
        assert_eq!(r.hitch, Some(("layout", 14., 150)));
        assert_eq!(r.idle_ms, None);
        let HudFrame::Busy { hitch, .. } = r.frames[HUD_FRAMES - 1] else {
            panic!("last frame is busy")
        };
        assert_eq!(hitch, Some("layout"));
    }
    #[test]
    fn idle_gaps_draw_as_stubs_never_count_and_report_time_since() {
        let mut meter = HitchMeter::new(60.);
        let start = Instant::now();
        meter.record(start, busy(2));
        meter.record(start + Duration::from_millis(16), busy(2));
        // A 2 s pause, then one frame: the gap is a stub, not a late frame.
        let woke = start + Duration::from_millis(2016);
        meter.record(woke, busy(2));
        let r = meter.perf_record(woke + Duration::from_millis(1500), 0.);
        assert_eq!(r.frames.len(), 4);
        assert_eq!(r.frames[2], HudFrame::Idle);
        assert_eq!(r.late, 0);
        assert_eq!(r.hz, 63, "the median busy interval (16 ms), idle excluded");
        assert_eq!(r.idle_ms, Some(1500.));
        // Within the idle threshold nothing is idle.
        assert_eq!(
            meter
                .perf_record(woke + Duration::from_millis(20), 0.)
                .idle_ms,
            None
        );
    }
    #[test]
    fn bounded_samples_keep_lifetime_count_and_phase_hitches() {
        let mut meter = HitchMeter::new(120.);
        let start = Instant::now();
        for i in 0..1030 {
            meter.record(
                start + Duration::from_millis(i * 10),
                Phases {
                    layout: Duration::from_millis(2),
                    ..Default::default()
                },
            );
        }
        assert_eq!(meter.samples().len(), 1024);
        assert_eq!(meter.total_frames(), 1030);
        assert_eq!(meter.cpu_percentile(0.99), Duration::from_millis(2));
        assert_eq!(
            meter.record(
                start + Duration::from_secs(20),
                Phases {
                    draw: Duration::from_millis(15),
                    ..Default::default()
                }
            ),
            Some(("draw", Duration::from_millis(15)))
        );
        assert!(
            meter
                .record(start + Duration::from_secs(40), Phases::default())
                .is_none()
        );
    }
}
