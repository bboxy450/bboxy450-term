// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Attention owns a bounded deadline; output/input still use the display link.
use crate::*;

pub const PULSE_SOURCE: usize = 1025;
pub const PULSE_INTERVAL: Duration = Duration::from_nanos(33_333_334);

#[derive(Default)]
pub struct Frames {
    pub full: u64,
    pub pulse: u64,
}

impl App {
    pub(super) fn gui_render_observation(&self) -> serde_json::Value {
        serde_json::json!({"gui_render": {
            "full_frames": self.gui_frames.full,
            "pulse_frames": self.gui_frames.pulse,
            "occluded": self.occluded,
        }})
    }

    pub(super) fn pace_after_frame(&mut self) -> Result<(), String> {
        let visible = !self.occluded && !self.clock.is_frozen();
        let continuous = visible
            && (self.animate
                || self.verify.continuous_redraw
                || self.scene.transient_animating(self.clock.now())
                || self.task_panel_animating()
                || self.rich_pending());
        if let Some(pacer) = &self.pacer {
            pacer.set_active(continuous)?;
        }
        self.attention_tick = (visible && !continuous && !self.attention_origins.is_empty())
            .then(|| Instant::now() + PULSE_INTERVAL);
        Ok(())
    }

    pub(super) fn poll_attention_frame(&mut self) {
        if self.occluded || self.clock.is_frozen() {
            self.attention_tick = None;
            return;
        }
        if self
            .attention_tick
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            self.attention_tick = None;
            self.attention_requested = true;
            self.request_redraw_sources(&[PULSE_SOURCE]);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn idle_redraw_controls_refuse_the_operator_account() {
        for flag in [
            "--idle-cpu-probe",
            "--idle-redraw-control",
            "--idle-full-scene-control",
            "--idle-occluded",
        ] {
            for args in [
                vec![flag.to_owned()],
                vec![flag.to_owned(), "--unattended".into()],
            ] {
                assert!(crate::Verify::from_args(&args).is_err());
            }
        }
    }
}
