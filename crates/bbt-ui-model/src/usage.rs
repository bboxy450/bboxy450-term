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
    /// Machine: percent used. Provider: percent left.
    pub percent: Option<u8>,
    pub stale: bool,
    pub reason: Option<String>,
}
impl UsageMetric {
    pub fn text(&self, provider: bool) -> String {
        let number = self.percent.map_or_else(
            || "—".into(),
            |p| format!("{p}%{}", if provider { " left" } else { "" }),
        );
        let mut label = self.label.clone();
        if provider && let Some(first) = label.get_mut(..1) {
            first.make_ascii_uppercase();
        }
        format!("{label} {number}{}", if self.stale { " (stale)" } else { "" })
    }
    pub fn description(&self, provider: bool) -> String {
        let text = self.text(provider);
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
