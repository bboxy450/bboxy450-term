// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Host-side measurements of plugin calls and hook deliveries: a bounded
//! ring per plugin, filled only by real calls, read by the inventory listing.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use bbt_plugin::registry::{Outcome, Sample, Span, SpanSkip, TRACE_SPAN_NAME, TRACE_SPANS};
use serde_json::Value;

/// Samples kept per plugin; older ones are dropped first.
pub const RING: usize = 64;
/// Plugins tracked at once; the least recently measured is forgotten.
pub const PLUGINS: usize = 256;

struct Recorded {
    end: Instant,
    sample: Sample,
}

#[derive(Clone, Default)]
pub(crate) struct Measurements(Arc<Mutex<HashMap<String, VecDeque<Recorded>>>>);

impl std::fmt::Debug for Measurements {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Measurements")
    }
}

impl Measurements {
    pub(crate) fn record(&self, candidate: &str, end: Instant, sample: Sample) {
        let Ok(mut held) = self.0.lock() else {
            return;
        };
        if !held.contains_key(candidate) && held.len() >= PLUGINS {
            let oldest = held
                .iter()
                .min_by_key(|(_, ring)| ring.back().map(|r| r.end))
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                held.remove(&oldest);
            }
        }
        let ring = held.entry(candidate.to_owned()).or_default();
        if ring.len() == RING {
            ring.pop_front();
        }
        ring.push_back(Recorded { end, sample });
    }

    /// This plugin's samples, oldest first, with `ago_ms` relative to `now`.
    pub(crate) fn snapshot(&self, candidate: &str, now: Instant) -> Vec<Sample> {
        let Ok(held) = self.0.lock() else {
            return Vec::new();
        };
        held.get(candidate)
            .map(|ring| {
                ring.iter()
                    .map(|recorded| {
                        let mut sample = recorded.sample.clone();
                        sample.ago_ms = ms(now.saturating_duration_since(recorded.end));
                        sample
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Samples per plugin in an inventory listing.
pub const LISTED: usize = 32;
/// The listing's window; Settings plots the same minute.
pub const WINDOW_MS: f64 = 60_000.;

/// The listed part of a snapshot: the last minute, newest [`LISTED`], phase
/// spans kept only on the newest sample (the one a flame graph shows).
pub(crate) fn listed(samples: Vec<Sample>) -> Vec<Sample> {
    let mut samples: Vec<Sample> = samples
        .into_iter()
        .filter(|sample| sample.ago_ms <= WINDOW_MS)
        .collect();
    let excess = samples.len().saturating_sub(LISTED);
    samples.drain(..excess);
    let newest = samples.len().saturating_sub(1);
    for (index, sample) in samples.iter_mut().enumerate() {
        if index != newest {
            sample.spans.clear();
        }
    }
    samples
}

fn ms(duration: std::time::Duration) -> f64 {
    duration.as_secs_f64() * 1000.
}

/// The phases of one call as they happen; [`Trace::finish`] records it.
pub(crate) struct Trace {
    start: Instant,
    phase: Instant,
    method: String,
    hook: Option<String>,
    spans: Vec<Span>,
    span_skip: Option<SpanSkip>,
}

impl Trace {
    pub(crate) fn start(method: &str, hook: Option<String>) -> Self {
        let now = Instant::now();
        Self {
            start: now,
            phase: now,
            method: method.into(),
            hook,
            spans: Vec::new(),
            span_skip: None,
        }
    }

    fn skip_spans(&mut self, code: &str, count: usize) {
        let count = u32::try_from(count).unwrap_or(u32::MAX);
        match &mut self.span_skip {
            Some(skip) => skip.count = skip.count.saturating_add(count),
            None => {
                self.span_skip = Some(SpanSkip {
                    code: code.into(),
                    count,
                })
            }
        }
    }

    /// Merge the plugin's own spans from a traced call's result
    /// (`result._meta.bboxy450.spans`, offsets in ms from when the plugin
    /// took the call) under the `plugin` phase just marked. A span is kept
    /// only inside that phase's window; malformed, oversized or outside
    /// spans are dropped and counted as a named skip. Plugin spans never
    /// change the host's own timing.
    pub(crate) fn plugin_spans(&mut self, result: &Value) {
        let Some(raw) = result.pointer("/_meta/bboxy450/spans") else {
            return;
        };
        let Some(window) = self.spans.last().filter(|s| s.name == "plugin").cloned() else {
            return;
        };
        let Some(raw) = raw.as_array() else {
            return self.skip_spans("trace_spans_invalid", 1);
        };
        if raw.len() > TRACE_SPANS {
            return self.skip_spans("trace_spans_oversized", raw.len());
        }
        for span in raw {
            let name = span["name"]
                .as_str()
                .filter(|name| !name.is_empty() && name.chars().count() <= TRACE_SPAN_NAME);
            let (Some(name), Some(start), Some(length)) =
                (name, span["start_ms"].as_f64(), span["ms"].as_f64())
            else {
                self.skip_spans("trace_span_invalid", 1);
                continue;
            };
            if !(start.is_finite() && length.is_finite() && start >= 0. && length >= 0.) {
                self.skip_spans("trace_span_invalid", 1);
                continue;
            }
            if start + length > window.ms {
                self.skip_spans("trace_span_outside_window", 1);
                continue;
            }
            self.spans.push(Span {
                name: name.into(),
                start_ms: window.start_ms + start,
                ms: length,
                depth: 2,
            });
        }
    }

    /// Close the phase that began at the previous mark (or the start).
    pub(crate) fn mark(&mut self, name: &str) {
        let now = Instant::now();
        self.spans.push(Span {
            name: name.into(),
            start_ms: ms(self.phase.saturating_duration_since(self.start)),
            ms: ms(now.saturating_duration_since(self.phase)),
            depth: 1,
        });
        self.phase = now;
    }

    pub(crate) fn finish(
        mut self,
        measurements: &Measurements,
        candidate: &str,
        outcome: Outcome,
        code: Option<String>,
    ) {
        let end = Instant::now();
        let total = ms(end.saturating_duration_since(self.start));
        self.spans.insert(
            0,
            Span {
                name: self.method.clone(),
                start_ms: 0.,
                ms: total,
                depth: 0,
            },
        );
        measurements.record(
            candidate,
            end,
            Sample {
                ago_ms: 0.,
                method: self.method,
                hook: self.hook,
                ms: total,
                outcome,
                code,
                spans: self.spans,
                span_skip: self.span_skip,
            },
        );
    }
}

/// How a dispatched call ended: accepted, refused by the plugin itself,
/// skipped because the connection was busy, or failed by the host.
pub(crate) fn outcome<T>(
    result: &Result<T, crate::knowledge::data_source::CommandError>,
    refused_by_plugin: bool,
) -> (Outcome, Option<String>) {
    match result {
        Ok(_) => (Outcome::Ok, None),
        Err(reason) if refused_by_plugin => (Outcome::Refused, Some(reason.code.clone())),
        Err(reason) if reason.code == "provider_busy" => {
            (Outcome::Skipped, Some(reason.code.clone()))
        }
        Err(reason) => (Outcome::Failed, Some(reason.code.clone())),
    }
}

/// A delivery the host did not make, recorded with its named reason.
pub(crate) fn skipped(measurements: &Measurements, candidate: &str, hook: &str, code: &str) {
    measurements.record(
        candidate,
        Instant::now(),
        Sample {
            ago_ms: 0.,
            // The MCP tool a hook is delivered through.
            method: "hook".into(),
            hook: Some(hook.into()),
            ms: 0.,
            outcome: Outcome::Skipped,
            code: Some(code.into()),
            spans: Vec::new(),
            span_skip: None,
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rings_are_bounded_per_plugin_and_report_age() {
        let m = Measurements::default();
        let start = Instant::now();
        for i in 0..(RING + 5) {
            let mut trace = Trace::start(&format!("data_source_list{i}"), None);
            trace.mark("plugin");
            trace.finish(&m, "a", Outcome::Ok, None);
        }
        let samples = m.snapshot("a", Instant::now());
        assert_eq!(samples.len(), RING);
        assert_eq!(
            samples[0].method, "data_source_list5",
            "oldest dropped first"
        );
        assert!(samples.iter().all(|s| s.ago_ms >= 0. && s.ms >= 0.));
        assert_eq!(samples[0].spans[0].depth, 0);
        assert_eq!(samples[0].spans[1].name, "plugin");
        assert!(m.snapshot("b", start).is_empty(), "no data is no samples");
        skipped(&m, "b", "PreToolUse", "queue_full");
        let b = m.snapshot("b", Instant::now());
        assert_eq!(b[0].outcome, Outcome::Skipped);
        assert_eq!(b[0].code.as_deref(), Some("queue_full"));
    }

    #[test]
    fn a_listing_keeps_the_last_minute_newest_first_spans_on_the_newest_only() {
        let sample = |ago_ms: f64| Sample {
            ago_ms,
            method: "m".into(),
            hook: None,
            ms: 1.,
            outcome: Outcome::Ok,
            code: None,
            spans: vec![Span {
                name: "m".into(),
                start_ms: 0.,
                ms: 1.,
                depth: 0,
            }],
            span_skip: None,
        };
        let mut samples = vec![sample(WINDOW_MS + 1.)];
        samples.extend((0..LISTED + 3).rev().map(|i| sample(i as f64)));
        let kept = listed(samples);
        assert_eq!(kept.len(), LISTED);
        assert_eq!(kept.last().unwrap().ago_ms, 0.);
        assert!(kept[..LISTED - 1].iter().all(|s| s.spans.is_empty()));
        assert_eq!(kept.last().unwrap().spans.len(), 1);
        assert!(listed(Vec::new()).is_empty());
    }

    #[test]
    fn plugin_spans_merge_inside_the_plugin_phase_and_bad_ones_are_named_skips() {
        let m = Measurements::default();
        let mut trace = Trace::start("repo_graph_describe", None);
        trace.mark("validate");
        std::thread::sleep(std::time::Duration::from_millis(30));
        trace.mark("plugin");
        let window = trace.spans.last().unwrap().clone();
        trace.plugin_spans(&serde_json::json!({"_meta":{"bboxy450":{"spans":[
            {"name":"scan","start_ms":1.0,"ms":5.0},
            {"name":"","start_ms":0.0,"ms":1.0},
            {"name":"late","start_ms":0.0,"ms":window.ms + 100.0},
            {"name":"negative","start_ms":-1.0,"ms":1.0}
        ]}}}));
        trace.finish(&m, "p", Outcome::Ok, None);
        let sample = &m.snapshot("p", Instant::now())[0];
        let scan = sample.spans.iter().find(|s| s.name == "scan").unwrap();
        assert_eq!(scan.depth, 2);
        assert_eq!(
            scan.start_ms,
            window.start_ms + 1.0,
            "relative to the plugin phase"
        );
        assert_eq!(
            sample.span_skip,
            Some(SpanSkip {
                code: "trace_span_invalid".into(),
                count: 3
            })
        );
        let mut many = Trace::start("m", None);
        many.mark("plugin");
        let spans: Vec<Value> = (0..=TRACE_SPANS)
            .map(|_| serde_json::json!({"name":"x","start_ms":0.0,"ms":0.0}))
            .collect();
        many.plugin_spans(&serde_json::json!({"_meta":{"bboxy450":{"spans":spans}}}));
        assert_eq!(many.spans.len(), 1, "an oversized list is dropped whole");
        assert_eq!(
            many.span_skip.as_ref().unwrap().code,
            "trace_spans_oversized"
        );
        let mut none = Trace::start("m", None);
        none.mark("plugin");
        none.plugin_spans(&serde_json::json!({"structuredContent":{}}));
        assert!(none.span_skip.is_none() && none.spans.len() == 1);
    }

    #[test]
    fn the_least_recently_measured_plugin_is_forgotten_beyond_the_bound() {
        let m = Measurements::default();
        for i in 0..=PLUGINS {
            Trace::start("hook", None).finish(&m, &format!("p{i}"), Outcome::Ok, None);
        }
        assert!(m.snapshot("p0", Instant::now()).is_empty());
        assert_eq!(m.snapshot(&format!("p{PLUGINS}"), Instant::now()).len(), 1);
    }
}
