// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Public startup observations, without process capabilities or UI-thread IO.
use std::{path::PathBuf, sync::Arc, time::Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Phase {
    StartingSupervisor,
    AdoptingSupervisor(u32),
    StartingLogic,
    VerifyingHistory {
        stream: String,
        verified: u64,
        total: u64,
    },
    AttachingGui,
    Reconnecting {
        attempt: u32,
        reason: String,
    },
    RefreshingLogic(String),
    Failed(String),
}
pub type Progress = Arc<dyn Fn(Phase) + Send + Sync>;

/// Additive stdout record; the existing LogicReady record is unchanged.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct ProgressLine {
    pub startup_progress: HistoryProgress,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct HistoryProgress {
    pub stream: String,
    pub verified: u64,
    pub total: u64,
}
impl HistoryProgress {
    pub(super) fn advances(&self, previous: Option<&Self>) -> bool {
        previous.is_none_or(|last| last.stream != self.stream || self.verified > last.verified)
    }
}

#[derive(Debug)]
pub struct Modal {
    pub visible: bool,
    pub phase: Phase,
    pub since: Instant,
    pub error: Option<String>,
    pub run_dir: Option<PathBuf>,
    pub reconnect: Option<(u32, String)>,
    pub refresh_reason: Option<String>,
}
impl Modal {
    pub fn new(now: Instant, visible: bool) -> Self {
        Self {
            visible,
            phase: Phase::StartingSupervisor,
            since: now,
            error: None,
            run_dir: None,
            reconnect: None,
            refresh_reason: None,
        }
    }
    pub fn update(&mut self, phase: Phase, now: Instant) {
        if let Phase::Failed(reason) = phase {
            self.fail(reason);
            return;
        }
        // A failure is sticky until the operator chooses Retry.
        if self.error.is_some() {
            return;
        }
        if !self.visible || std::mem::discriminant(&self.phase) != std::mem::discriminant(&phase) {
            self.since = now;
        }
        if let Phase::Reconnecting { attempt, reason } = &phase {
            self.reconnect = Some((*attempt, reason.clone()));
        }
        if let Phase::RefreshingLogic(reason) = &phase {
            self.refresh_reason = Some(reason.clone());
        }
        self.phase = phase;
        self.visible = true;
    }
    pub fn fail(&mut self, reason: String) {
        self.visible = true;
        self.error = Some(reason);
    }
    pub fn attached(&mut self) {
        if self.error.is_none() {
            self.visible = false;
            self.reconnect = None;
            self.refresh_reason = None;
        }
    }
    pub fn retry(&mut self, now: Instant) {
        self.error = None;
        self.reconnect = None;
        self.refresh_reason = None;
        self.visible = false;
        self.update(
            Phase::Reconnecting {
                attempt: 1,
                reason: "Retry requested".into(),
            },
            now,
        );
    }
    pub fn label(&self) -> String {
        match &self.phase {
            Phase::StartingSupervisor => "Starting supervisor".into(),
            Phase::AdoptingSupervisor(pid) => format!("Adopting supervisor (pid {pid})"),
            Phase::StartingLogic => "Starting logic".into(),
            Phase::VerifyingHistory {
                stream,
                verified,
                total,
            } => format!("Logic verifying history {verified}/{total} · {stream}"),
            Phase::AttachingGui => "Attaching GUI".into(),
            Phase::Reconnecting { attempt, reason } => {
                format!("Reconnecting (attempt {attempt}) · {reason}")
            }
            Phase::RefreshingLogic(reason) => format!("Refreshing logic · {reason}"),
            Phase::Failed(reason) => reason.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn startup_modal_tracks_progress_failure_retry_and_reattach() {
        let start = Instant::now();
        let mut modal = Modal::new(start, true);
        assert_eq!(modal.label(), "Starting supervisor");
        modal.update(
            Phase::AdoptingSupervisor(42),
            start + Duration::from_secs(1),
        );
        assert!(modal.label().contains("pid 42"));
        let progress = |verified| Phase::VerifyingHistory {
            stream: "account".into(),
            verified,
            total: 100,
        };
        modal.update(progress(0), start + Duration::from_secs(2));
        modal.update(progress(25), start + Duration::from_secs(5));
        assert!(modal.label().contains("25/100"));
        assert_eq!(modal.since, start + Duration::from_secs(2));
        modal.fail("history signature failed".into());
        modal.update(Phase::AttachingGui, start + Duration::from_secs(6));
        modal.attached();
        assert!(modal.visible);
        assert_eq!(modal.error.as_deref(), Some("history signature failed"));
        modal.retry(start + Duration::from_secs(7));
        assert!(modal.error.is_none());
        modal.update(Phase::AttachingGui, start + Duration::from_secs(8));
        modal.attached();
        assert!(!modal.visible);
        modal.update(
            Phase::Reconnecting {
                attempt: 2,
                reason: "GUI connection closed".into(),
            },
            start + Duration::from_secs(9),
        );
        assert!(modal.visible);
        assert!(modal.label().contains("attempt 2"));
        modal.update(
            Phase::RefreshingLogic("schema changed".into()),
            start + Duration::from_secs(10),
        );
        assert_eq!(modal.reconnect, Some((2, "GUI connection closed".into())));
        assert!(modal.label().contains("schema changed"));
        assert_eq!(modal.refresh_reason.as_deref(), Some("schema changed"));

        let wire = r#"{"startup_progress":{"stream":"account","verified":25,"total":100}}"#;
        let record: ProgressLine = serde_json::from_str(wire).unwrap();
        assert!(
            !record
                .startup_progress
                .advances(Some(&record.startup_progress))
        );
        let mut later = record.startup_progress.clone();
        later.verified += 1;
        assert!(later.advances(Some(&record.startup_progress)));
        later.verified = 0;
        assert!(!later.advances(Some(&record.startup_progress)));
    }
}
