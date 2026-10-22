//! Pair asynchronous Metal GPU timestamps with the same frame's CPU work.
use std::{collections::VecDeque, time::Duration};
pub const SAMPLES: usize = 1024;

pub struct GpuSample {
    pub frame: u64,
    pub time: Duration,
}
struct Sample {
    frame: u64,
    cpu: Duration,
    gpu: Option<Duration>,
}
#[derive(Default)]
pub struct Budget(VecDeque<Sample>);
impl Budget {
    pub fn cpu(&mut self, frame: u64, cpu: Duration) {
        if self.0.len() == SAMPLES {
            self.0.pop_front();
        }
        self.0.push_back(Sample {
            frame,
            cpu,
            gpu: None,
        });
    }
    pub fn gpu(&mut self, sample: GpuSample) {
        if let Some(frame) = self.0.iter_mut().rev().find(|f| f.frame == sample.frame) {
            frame.gpu = Some(sample.time);
        }
    }
    pub fn report(&self) -> String {
        let mut gpu = Vec::new();
        let mut total = Vec::new();
        for sample in &self.0 {
            if let Some(time) = sample.gpu {
                gpu.push(time);
                total.push(sample.cpu + time);
            }
        }
        gpu.sort_unstable();
        total.sort_unstable();
        let ms = |times: &[Duration], p: f64| {
            let index = ((times.len() as f64 * p).ceil() as usize).saturating_sub(1);
            times.get(index).copied().unwrap_or_default().as_secs_f64() * 1000.
        };
        let over = total
            .iter()
            .filter(|time| **time > Duration::from_micros(8333))
            .count();
        format!(
            "GPU timestamps: paired={} pending={} GPU_span_p50={:.3}ms GPU_span_p99={:.3}ms CPU+GPU_span_p50={:.3}ms p99={:.3}ms max={:.3}ms over_8.333ms={over}; CPU is redraw wall time (Fifo/CV drawable wait excluded); GPU span bounds render+uploads between serial-queue marker GPUStartTime/GPUEndTime (includes marker/queue gaps)",
            total.len(),
            self.0.len() - total.len(),
            ms(&gpu, 0.5),
            ms(&gpu, 0.99),
            ms(&total, 0.5),
            ms(&total, 0.99),
            ms(&total, 1.)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn late_gpu_completions_pair_by_frame_and_evicted_samples_are_ignored() {
        let mut budget = Budget::default();
        for frame in 0..=SAMPLES as u64 {
            budget.cpu(frame, Duration::from_millis(2));
        }
        budget.gpu(GpuSample {
            frame: 2,
            time: Duration::from_millis(3),
        });
        budget.gpu(GpuSample {
            frame: 0,
            time: Duration::from_secs(1),
        });
        assert_eq!(budget.0.len(), SAMPLES);
        assert!(budget.report().contains("paired=1 pending=1023"));
        assert!(budget.report().contains("CPU+GPU_span_p50=5.000ms"));
    }
}
