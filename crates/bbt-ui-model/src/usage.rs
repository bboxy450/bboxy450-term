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
        format!(
            "{} {number}{}",
            self.label,
            if self.stale { " (stale)" } else { "" }
        )
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
        self.machine
            .iter()
            .map(|m| m.description(false))
            .chain(self.providers.iter().map(|m| m.description(true)))
            .collect::<Vec<_>>()
            .join(" · ")
    }
}
