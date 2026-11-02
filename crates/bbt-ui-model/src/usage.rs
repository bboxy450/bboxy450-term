// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Last-observed title-bar values. No IO, credential or runtime dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageMetric {
    pub label: String,
    /// Percent used (machine and provider alike).
    pub percent: Option<u8>,
    /// Provider pace: percent this window reaches by its reset (Tachyon).
    pub projected: Option<u16>,
    pub stale: bool,
    pub reason: Option<String>,
}
impl UsageMetric {
    pub fn text(&self, provider: bool) -> String {
        let number = self.percent.map_or_else(|| "—".into(), |p| format!("{p}%"));
        let mut label = self.label.clone();
        if provider && let Some(first) = label.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        format!("{label} {number}{}", if self.stale { " (stale)" } else { "" })
    }
    /// Tachyon: on pace to exhaust the window before it resets, with capacity left.
    pub fn pace_hot(&self) -> bool {
        self.percent.is_some_and(|p| p < 100) && self.projected.is_some_and(|p| p >= 100)
    }
    /// Tachyon band input: the used percent, lifted one band when pace-hot.
    pub fn band_percent(&self) -> Option<u8> {
        let p = self.percent?;
        Some(if !self.pace_hot() {
            p
        } else if p < 50 {
            50
        } else if p < 70 {
            70
        } else if p < 90 {
            90
        } else {
            p
        })
    }
    /// Tachyon `PaceFormat.caption`.
    pub fn pace_caption(&self) -> Option<String> {
        let p = self.percent?;
        if p >= 100 {
            return Some("limit reached".into());
        }
        let projected = self.projected?;
        Some(if projected >= 100 {
            "at this pace, limit before reset".into()
        } else {
            format!("at this pace, ~{projected}% by reset")
        })
    }
    pub fn description(&self, provider: bool) -> String {
        let mut text = self.text(provider);
        if provider && self.percent.is_some() {
            text.push_str(" used");
            if let Some(caption) = self.pace_caption() {
                text = format!("{text}, {caption}");
            }
        }
        self.reason
            .as_ref()
            .map_or(text.clone(), |reason| format!("{text}: {reason}"))
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UsageStrip {
    pub machine: Vec<UsageMetric>,
    /// Missing installations have no entry.
    pub providers: Vec<UsageMetric>,
}
impl UsageStrip {
    pub fn accessible_label(&self) -> String {
        self.lines().join(" · ")
    }
    /// One line for the machine, one for the accounts; empty groups are skipped.
    pub fn lines(&self) -> Vec<String> {
        [(false, &self.machine), (true, &self.providers)]
            .into_iter()
            .filter(|(_, metrics)| !metrics.is_empty())
            .map(|(provider, metrics)| {
                metrics
                    .iter()
                    .map(|m| m.description(provider))
                    .collect::<Vec<_>>()
                    .join(" · ")
            })
            .collect()
    }
}
