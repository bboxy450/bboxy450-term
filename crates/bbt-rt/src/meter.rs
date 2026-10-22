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
        (phases.cpu().max(phases.present_wait) > self.display_interval.mul_f64(1.5))
            .then(|| phases.slowest())
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
