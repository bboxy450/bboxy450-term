// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Host registry observations projected into Wren's Settings shapes; effects use GUI admission.
use bbt_ui::modules::settings::{
    Records,
    knowledge::{Knowledge, Root},
    plugins::Plugin,
};
use std::sync::LazyLock;

// Wren's signed 4edc61ba public reference shapes. The page labels every one as
// example data, including its timings and process-health words.
static REFERENCE: LazyLock<Vec<Plugin>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("plugin-reference.json"))
        .expect("pinned public plugin fixtures")
});
pub(super) fn project(out: &mut Records, model: Option<&crate::model::Model>) {
    if let Some(model) = model {
        let scope = match model.ui.window.stage.swarm {
            Some(s) => model.swarm_scope(s).ok().map(Some),
            None => Some(None),
        };
        match model
            .plugin_registry
            .as_ref()
            .filter(|_| Some(model.plugin_scope) == scope)
        {
            Some(entries) => {
                out.plugins = entries.iter().map(plugin).collect();
                out.plugin_controls = true;
                out.plugin_notice = Some("Workspace plugin registry. Trust pins one binary at its minimum tier; enable is separate. Run ctl plugin check before choosing a Knowledge provider.".into());
            }
            None => {
                out.plugins.clear();
                out.plugin_notice = Some("Production plugin registry snapshot pending.".into());
            }
        }
    } else {
        out.plugins = REFERENCE.clone();
        for plugin in &mut out.plugins {
            plugin.health.say = Some(plugin.health.say.as_ref().map_or_else(
                || "reference fixture".into(),
                |detail| format!("reference fixture · {detail}"),
            ));
        }
        out.plugin_notice = Some("Reference fixtures — these plugins and timings are examples. Plugin discovery, trust, enable and setup are unavailable in this build.".into());
    }
    let snapshot = model.and_then(|model| model.e2e.as_ref());
    let (answering, read) = match snapshot {
        Some(snapshot) if snapshot.faults.is_empty() => ("logic · in process", String::new()),
        Some(snapshot) => (
            "bundled read unavailable",
            format!(" {}", snapshot.faults.join(" · ")),
        ),
        None => (
            "bundled read snapshot pending",
            " The current read snapshot is not available yet.".into(),
        ),
    };
    let provider = "bboxy450-term-data_source-bboxy";
    out.knowledge = Knowledge {
        roots: vec![Root {
            name: ".bboxy".into(),
            path: ".bboxy/ (current repository)".into(),
            provider: provider.into(),
            // The current read snapshot contains acceptance records, not an
            // inventory of the entire root. Never present it as a root count.
            records: None,
            answering: answering.into(),
            digests: None,
            refused: Some(format!(
                "Provider switching is unavailable in this build; the bundled provider remains active.{read}"
            )),
            pending: None,
        }],
        offered: vec![provider.into()],
    };
    if let Some(model) = model {
        // A pending live snapshot cannot assert which provider is active.
        out.knowledge = Knowledge::default();
        let scope = match model.ui.window.stage.swarm {
            Some(s) => model.swarm_scope(s).ok().map(Some),
            None => Some(None),
        };
        if let Some(status) = model
            .knowledge_roots
            .as_deref()
            .filter(|_| Some(model.knowledge_scope) == scope)
        {
            super::knowledge::project(out, status);
        }
    }
}

/// Requested versus granted: every payload event, host API method and (with a
/// `before` hook) block policy the trusted shape can be granted, plus any
/// message destination already granted.
fn grant_rows(
    record: &bbt_proto::gui::PluginTrustRecord,
    shape: &bbt_proto::gui::PluginShape,
) -> Vec<bbt_ui::modules::settings::plugins::GrantRow> {
    use bbt_proto::gui::{PLUGIN_ALL_REQUESTED, PluginFailurePolicy, PluginGrants};
    use bbt_ui::modules::settings::plugins::GrantRow;
    let requested = PluginGrants::requested_by(shape);
    let granted = &record.grants;
    let mut rows = Vec::new();
    for event in &requested.payload_events {
        rows.push(GrantRow {
            name: format!("payload_events:{event}"),
            granted: granted.payload_events.contains(event)
                || granted.payload_events.contains(PLUGIN_ALL_REQUESTED),
        });
    }
    for method in &requested.host_api {
        rows.push(GrantRow {
            name: format!("host_api:{method}"),
            granted: granted.host_api.contains(method),
        });
    }
    if shape
        .hooks
        .iter()
        .any(|hook| hook.phase == bbt_proto::gui::PluginPhase::Before)
    {
        for (policy, name) in [
            (PluginFailurePolicy::FailOpen, "block:fail_open"),
            (PluginFailurePolicy::FailClosed, "block:fail_closed"),
        ] {
            rows.push(GrantRow {
                name: name.into(),
                granted: granted.block == Some(policy),
            });
        }
    }
    for destination in &granted.messages_to {
        rows.push(GrantRow {
            name: format!("messages_to:{destination}"),
            granted: true,
        });
    }
    rows
}

/// Apply one grant toggle; a revoked `*` payload grant becomes the explicit rest.
fn set_grant(
    grants: &mut bbt_proto::gui::PluginGrants,
    grant: &str,
    on: bool,
) -> Result<(), String> {
    use bbt_proto::gui::{PLUGIN_ALL_REQUESTED, PluginFailurePolicy};
    match grant.split_once(':') {
        Some(("payload_events", event)) => {
            grants.payload_events.remove(PLUGIN_ALL_REQUESTED);
            if on {
                grants.payload_events.insert(event.into());
            } else {
                grants.payload_events.remove(event);
            }
        }
        Some(("host_api", method)) => {
            if on {
                grants.host_api.insert(method.into());
            } else {
                grants.host_api.remove(method);
            }
        }
        Some(("block", policy)) => {
            let policy = match policy {
                "fail_open" => PluginFailurePolicy::FailOpen,
                "fail_closed" => PluginFailurePolicy::FailClosed,
                _ => return Err(format!("unknown block policy {policy}")),
            };
            grants.block = on.then_some(policy);
        }
        Some(("messages_to", destination)) => {
            if on {
                grants.messages_to.insert(destination.into());
            } else {
                grants.messages_to.remove(destination);
            }
        }
        _ => return Err(format!("unknown grant {grant}")),
    }
    Ok(())
}

/// The last recorded setup result for the current pin, in plain words.
fn setup_label(setup: Option<&bbt_proto::gui::PluginSetupResult>) -> String {
    use bbt_proto::gui::PluginSetupState as State;
    let Some(setup) = setup else {
        return "setup not run".into();
    };
    let mut label = match setup.state {
        State::LoggedIn => "logged in".to_owned(),
        State::LoggedOut => "not logged in".to_owned(),
        State::LoginRefused => format!(
            "login refused (exit {})",
            setup.login_exit.map_or("signal".into(), |c| c.to_string())
        ),
        State::Failed => format!(
            "setup failed ({})",
            setup.code.as_deref().unwrap_or("unknown")
        ),
    };
    if !setup.detail.is_empty() {
        label.push_str(&format!(" · {}", setup.detail));
    }
    label.push_str(&format!(" · {} ms", setup.elapsed_ms));
    label
}

/// The host's measured calls and hook deliveries, as Settings plots them.
fn measured_calls(
    entry: &bbt_proto::gui::PluginEntry,
) -> Vec<bbt_ui::modules::settings::plugins::Call> {
    use bbt_proto::gui::PluginOutcome as Outcome;
    use bbt_ui::modules::settings::plugins::{Call, Span};
    entry
        .measurements
        .iter()
        .map(|sample| Call {
            t: sample.ago_ms,
            method: sample.method.clone(),
            ms: sample.ms,
            hook: sample.hook.clone(),
            spans: sample
                .spans
                .iter()
                .map(|span| Span {
                    name: span.name.clone(),
                    start: span.start_ms,
                    ms: span.ms,
                    depth: span.depth,
                })
                .collect(),
            outcome: match sample.outcome {
                Outcome::Ok => None,
                Outcome::Refused => Some("refused".into()),
                Outcome::Failed => Some("failed".into()),
                Outcome::Skipped => Some("skipped".into()),
            },
            code: sample.code.clone(),
            spans_dropped: sample.span_skip.as_ref().map_or(0, |skip| skip.count),
            spans_dropped_code: sample.span_skip.as_ref().map(|skip| skip.code.clone()),
        })
        .collect()
}

fn plugin(entry: &bbt_proto::gui::PluginEntry) -> Plugin {
    use bbt_proto::gui::PluginState as State;
    use bbt_ui::modules::settings::plugins::{Contract, Health};
    let state = entry.state();
    let activation = entry.activation.as_ref();
    let bound = activation.is_some_and(|activation| {
        !activation.needs_retrust && entry.candidate.sha256.as_ref() == Some(&activation.sha256)
    });
    let approved = entry.trust.as_ref().filter(|_| bound);
    let inspection = activation
        .filter(|_| bound)
        .and_then(|activation| activation.inspection.as_ref());
    let shape = approved.and_then(|record| record.shape.as_ref());
    let refusal = entry
        .candidate
        .refusal
        .as_ref()
        .or_else(|| inspection.and_then(|i| i.refusal.as_ref()));
    let named = entry.candidate.name.as_ref();
    let tier = approved
        .map(|t| t.approved_tier)
        .or_else(|| named.map(|n| n.system.tier()));
    let tier = tier
        .and_then(|tier| serde_json::to_value(tier).ok())
        .and_then(|value| value.as_str().map(str::to_owned))
        .unwrap_or_else(|| "unavailable".into());
    let calls = measured_calls(entry);
    Plugin {
        // Only the host's real measurements; no data keeps the label.
        measurements_unavailable: calls.is_empty(),
        candidate: entry.candidate.id.clone(),
        actionable: matches!(
            state,
            State::Untrusted | State::NeedsRetrust | State::Disabled | State::Enabled
        ) && refusal.is_none(),
        name: shape.map(|shape| shape.name.clone()).unwrap_or_else(|| {
            std::path::Path::new(&entry.candidate.source)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned()
        }),
        contract: Contract {
            name: named.map_or("unknown", |n| n.system.name()).into(),
            version: named.map_or("unknown", |n| n.system.version()).into(),
            // Only an explicit isolated check makes a method-conformance claim.
            ok: refusal
                .map(|_| false)
                .or_else(|| entry.conformance.as_ref().map(|c| c.ok)),
            failing: refusal.map(|r| r.code.clone()).or_else(|| {
                entry
                    .conformance
                    .as_ref()
                    .and_then(|c| c.refusal.as_ref())
                    .map(|r| r.code.clone())
            }),
        },
        tier,
        local: true,
        trust: if matches!(state, State::Untrusted | State::NeedsRetrust) {
            "untrusted"
        } else if approved.is_some() {
            "trusted"
        } else {
            "untrusted"
        }
        .into(),
        repo_local: entry.candidate.scope.name() == "repository",
        grant: approved
            .filter(|record| record.grants == Default::default())
            .map(|_| "no additional grants".into()),
        grants: approved
            .zip(shape)
            .map(|(record, shape)| grant_rows(record, shape))
            .unwrap_or_default(),
        version: shape.map(|s| s.version.clone()),
        // The host's recorded setup of this pin, never an assumption.
        auth: shape
            .and_then(|s| s.auth.as_ref())
            .map(|_| setup_label(entry.setup.as_ref())),
        health: Health {
            state: match state {
                State::Refused => "degraded",
                State::Untrusted => "never run",
                State::NeedsRetrust => "needs re-trust",
                State::Unchecked => "descriptor unchecked",
                State::Disabled => "disabled",
                State::Enabled => "enabled · not started",
            }
            .into(),
            say: Some(
                refusal
                    .map(|r| format!("{}: {}", r.code, r.message))
                    .unwrap_or_else(|| {
                        inspection.map_or_else(
                            || "no descriptor or runtime health observation".into(),
                            |i| {
                                format!(
                                    "descriptor {} ms; runtime health not queried",
                                    i.elapsed_ms
                                )
                            },
                        )
                    }),
            ),
        },
        enabled: state == State::Enabled,
        // Host phase spans are measured for every call; a flame graph needs one.
        trace: shape.is_some_and(|s| s.trace) || calls.iter().any(|call| !call.spans.is_empty()),
        calls,
        alerts: if state == State::NeedsRetrust {
            vec!["Binary changed; explicit trust of the current digest is required.".into()]
        } else {
            vec![]
        },
        ..Default::default()
    }
}

/// Translate a UI intent only from the currently observed candidate; the host rechecks authority.
pub fn request(
    model: &crate::model::Model,
    intent: bbt_ui::modules::settings::controller::PluginIntent,
) -> Result<bbt_proto::gui::PluginRequest, String> {
    use bbt_proto::gui::PluginRequest;
    use bbt_ui::modules::settings::controller::PluginIntent;
    let request = match intent {
        PluginIntent::Trust(candidate) => {
            let entry = model
                .plugin_registry
                .as_ref()
                .and_then(|entries| entries.iter().find(|e| e.candidate.id == candidate))
                .ok_or("Plugin is no longer in the current inventory")?;
            PluginRequest::Trust {
                path: entry.candidate.source.clone(),
                sha256: entry.candidate.sha256.clone(),
                grants: None,
                tier: None,
            }
        }
        PluginIntent::Grant {
            candidate,
            grant,
            on,
        } => {
            let entry = model
                .plugin_registry
                .as_ref()
                .and_then(|entries| entries.iter().find(|e| e.candidate.id == candidate))
                .ok_or("Plugin is no longer in the current inventory")?;
            let record = entry
                .trust
                .as_ref()
                .ok_or("Trust the plugin before changing its grants")?;
            let mut grants = record.grants.clone();
            set_grant(&mut grants, &grant, on)?;
            PluginRequest::Trust {
                path: entry.candidate.source.clone(),
                sha256: Some(record.sha256.clone()),
                grants: Some(grants),
                tier: None,
            }
        }
        PluginIntent::Enabled {
            candidate,
            enabled: true,
        } => PluginRequest::Enable { candidate },
        PluginIntent::Enabled {
            candidate,
            enabled: false,
        } => PluginRequest::Disable { candidate },
        PluginIntent::Setup(candidate) => PluginRequest::Setup { candidate },
    };
    Ok(request)
}

#[derive(Default)]
pub struct Requests(Vec<Pending>);
struct Pending {
    op: bbt_mux::Ulid,
    request: bbt_proto::gui::PluginRequest,
    unknown: bool,
    serial: u64,
}
impl Requests {
    /// Unknown effects retain their ID; other plugins and inventory remain independent.
    pub fn issue(
        &mut self,
        model: &crate::model::Model,
        request: bbt_proto::gui::PluginRequest,
    ) -> Result<Option<crate::cockpit::UiMsg>, String> {
        use crate::messaging::Reply;
        self.0.retain_mut(|pending| {
            let Some(row) = model
                .operation_results
                .rows()
                .find(|row| row.op == pending.op)
            else {
                return true;
            };
            if row.serial <= pending.serial {
                return true;
            }
            match &row.reply {
                Reply::Unknown(_) if pending.request.mutation() => {
                    pending.unknown = true;
                    true
                }
                Reply::Refused(reason) if pending.unknown && reason.starts_with("held:") => true,
                _ => false,
            }
        });
        let op = if let Some(pending) = self.0.iter_mut().find(|p| p.request == request) {
            if !pending.unknown {
                return Ok(None);
            }
            pending.unknown = false;
            pending.serial = model
                .operation_results
                .rows()
                .find(|r| r.op == pending.op)
                .map_or(0, |r| r.serial);
            pending.op
        } else {
            if self.0.len() >= 32 {
                return Err("plugin_control_window_full: resolve the retained operations before adding more".into());
            }
            let op = bbt_mux::Ulid::new();
            self.0.push(Pending {
                op,
                request: request.clone(),
                unknown: false,
                serial: 0,
            });
            op
        };
        Ok(Some(crate::cockpit::UiMsg::PluginControl {
            id: op,
            request,
        }))
    }

    pub fn decorate(&self, records: &mut Records, model: &crate::model::Model) {
        use bbt_proto::gui::PluginRequest;
        for plugin in &mut records.plugins {
            for pending in &self.0 {
                let selected = match &pending.request {
                    PluginRequest::Trust { path, .. } => {
                        model.plugin_registry.as_ref().is_some_and(|entries| {
                            entries.iter().any(|entry| {
                                entry.candidate.id == plugin.candidate
                                    && entry.candidate.source == *path
                            })
                        })
                    }
                    PluginRequest::Enable { candidate }
                    | PluginRequest::Disable { candidate }
                    | PluginRequest::Check { candidate }
                    | PluginRequest::Setup { candidate } => *candidate == plugin.candidate,
                    PluginRequest::List => false,
                };
                if !selected {
                    continue;
                }
                let row = model
                    .operation_results
                    .rows()
                    .find(|row| row.op == pending.op && row.serial > pending.serial);
                let verb = match &pending.request {
                    PluginRequest::Trust { .. } => "trust",
                    PluginRequest::Enable { .. } => "enable",
                    PluginRequest::Disable { .. } => "disable",
                    PluginRequest::Check { .. } => "check",
                    PluginRequest::Setup { .. } => "setup",
                    PluginRequest::List => unreachable!(),
                };
                match row.map(|row| &row.reply) {
                    None => {
                        plugin.actionable = false;
                        plugin.pending = Some(bbt_ui::modules::settings::plugins::IntentPending {
                            verb: verb.into(),
                        });
                    }
                    Some(crate::messaging::Reply::Refused(reason)) => {
                        plugin.refused = Some(bbt_ui::modules::settings::plugins::IntentRefused {
                            verb: verb.into(),
                            reason: reason.clone(),
                        });
                    }
                    Some(crate::messaging::Reply::Unknown(reason)) => {
                        plugin.alerts.push(format!(
                            "{verb} outcome unknown · {} · {reason}",
                            pending.op
                        ));
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Inventory and mutation completions can cross. An older list cannot erase a newer activation.
pub fn merge(
    previous: &[bbt_proto::gui::PluginEntry],
    mut next: Vec<bbt_proto::gui::PluginEntry>,
    all: bool,
) -> Vec<bbt_proto::gui::PluginEntry> {
    for old in previous {
        if let Some(entry) = next
            .iter_mut()
            .find(|entry| entry.candidate.id == old.candidate.id)
        {
            let generation = |entry: &bbt_proto::gui::PluginEntry| {
                entry.activation.as_ref().map_or(0, |a| a.generation)
            };
            if generation(old) > generation(entry) {
                *entry = old.clone();
            }
        } else if !all || old.activation.is_some() {
            next.push(old.clone());
        }
    }
    next.sort_by(|a, b| a.candidate.source.cmp(&b.candidate.source));
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::gui::{PluginEntry, PluginRequest};
    use serde_json::json;
    fn entry(generation: Option<u64>) -> PluginEntry {
        let name = json!({"system":"repo_graph","provider":"stub"});
        serde_json::from_value(json!({"candidate":{"id":"candidate","source":"/private/plugins/bboxy450-term-repo_graph-stub","scope":"repository","name":name,"sha256":"a".repeat(64),"refusal":null},
        "activation":generation.map(|generation| json!({"candidate":"candidate","source":"/private/plugins/bboxy450-term-repo_graph-stub","scope":"repository","name":name,"sha256":"a".repeat(64),"generation":generation,"desired_enabled":false,"needs_retrust":false,"inspection":null})),"trust":null})).unwrap()
    }
    #[test]
    fn production_projection_uses_only_host_facts_and_no_reference_measurements() {
        let mut model = crate::model::Model::default();
        let mut out = Records::default();
        project(&mut out, Some(&model));
        assert!(out.plugins.is_empty());
        assert!(!out.plugin_controls);
        model.plugin_registry = Some(std::sync::Arc::new(vec![entry(None)]));
        project(&mut out, Some(&model));
        assert!(out.plugin_controls);
        assert_eq!(out.plugins.len(), 1);
        let plugin = &out.plugins[0];
        assert_eq!(plugin.candidate, "candidate");
        assert_eq!(plugin.trust, "untrusted");
        assert!(!plugin.enabled);
        assert!(plugin.calls.is_empty());
        assert!(plugin.measurements_unavailable, "no data keeps the label");
        assert_eq!(plugin.contract.ok, None);
        assert!(!out.plugin_notice.as_ref().unwrap().contains("examples"));
    }
    // keeps: #57 — Settings shows the host's recorded setup of this pin, and
    // the setup intent becomes a Setup request for the same candidate.
    #[test]
    fn recorded_setup_results_are_shown_in_plain_words() {
        use bbt_proto::gui::{PluginSetupResult, PluginSetupState};
        let result = |state, login_exit, code: Option<&str>| PluginSetupResult {
            sha256: "a".repeat(64),
            state,
            code: code.map(str::to_owned),
            login_exit,
            detail: "fixture account".into(),
            elapsed_ms: 12,
        };
        assert_eq!(setup_label(None), "setup not run");
        assert_eq!(
            setup_label(Some(&result(PluginSetupState::LoggedIn, Some(0), None))),
            "logged in · fixture account · 12 ms"
        );
        assert!(
            setup_label(Some(&result(PluginSetupState::LoginRefused, Some(2), None)))
                .starts_with("login refused (exit 2)")
        );
        assert!(
            setup_label(Some(&result(
                PluginSetupState::Failed,
                None,
                Some("login_deadline_exceeded")
            )))
            .starts_with("setup failed (login_deadline_exceeded)")
        );
        let model = crate::model::Model::default();
        assert_eq!(
            request(
                &model,
                bbt_ui::modules::settings::controller::PluginIntent::Setup("candidate".into())
            )
            .unwrap(),
            PluginRequest::Setup {
                candidate: "candidate".into()
            }
        );
    }
    // keeps: #61 — only the host's measured samples become timeline calls.
    #[test]
    fn host_measurements_become_calls_with_named_outcomes_and_spans() {
        let mut measured = entry(Some(1));
        measured.measurements = serde_json::from_value(json!([
            {"ago_ms":2000.0,"method":"hook","hook":"PreToolUse","ms":0.0,"outcome":"skipped","code":"stale_generation"},
            {"ago_ms":500.0,"method":"repo_graph_describe","ms":12.5,"outcome":"ok",
             "span_skip":{"code":"trace_span_invalid","count":1},
             "spans":[{"name":"repo_graph_describe","start_ms":0.0,"ms":12.5,"depth":0},
                      {"name":"plugin","start_ms":1.0,"ms":10.0,"depth":1}]}
        ]))
        .unwrap();
        let model = crate::model::Model {
            plugin_registry: Some(std::sync::Arc::new(vec![measured])),
            ..Default::default()
        };
        let mut out = Records::default();
        project(&mut out, Some(&model));
        let plugin = &out.plugins[0];
        assert!(!plugin.measurements_unavailable);
        assert!(plugin.trace, "host spans enable the flame graph");
        assert_eq!(plugin.calls.len(), 2);
        assert_eq!(plugin.calls[0].hook.as_deref(), Some("PreToolUse"));
        assert_eq!(plugin.calls[0].outcome.as_deref(), Some("skipped"));
        assert_eq!(plugin.calls[0].code.as_deref(), Some("stale_generation"));
        assert_eq!(plugin.calls[1].outcome, None);
        assert_eq!(plugin.calls[1].t, 500.0);
        assert_eq!(plugin.calls[1].spans[1].name, "plugin");
        assert_eq!(plugin.calls[1].spans[1].start, 1.0);
        assert_eq!(plugin.calls[1].spans_dropped, 1);
        assert_eq!(
            plugin.calls[1].spans_dropped_code.as_deref(),
            Some("trace_span_invalid")
        );
    }
    // keeps: plugins §3b/§3c operator grants — the card lists what the shape
    // requests with the current grant, and a toggle re-trusts the same digest
    // with exactly that grant changed.
    #[test]
    fn plugin_card_shows_requested_versus_granted_and_toggles_re_trust() {
        use bbt_ui::modules::settings::controller::PluginIntent;
        let mut value = serde_json::to_value(entry(Some(2))).unwrap();
        value["trust"] = json!({"sha256":"a".repeat(64),"source":"/private/plugins/bboxy450-term-repo_graph-stub",
            "copy":"/private/copy","data_dir":"/private/data","name":{"system":"repo_graph","provider":"stub"},
            "scope":"repository","approved_tier":"export",
            "grants":{"payload_events":["PreToolUse"]},
            "shape":{"name":"stub","version":"1.0","shape_version":"1.0","health_command":"health","systems":[],
                "hooks":[{"event":"PreToolUse","phase":"before","budget_ms":500}],"host_api":["messages.send"]}});
        let entry: PluginEntry = serde_json::from_value(value).unwrap();
        let rows = plugin(&entry).grants;
        let shown: Vec<_> = rows.iter().map(|r| (r.name.as_str(), r.granted)).collect();
        assert_eq!(
            shown,
            [
                ("payload_events:PreToolUse", true),
                ("host_api:messages.send", false),
                ("block:fail_open", false),
                ("block:fail_closed", false),
            ]
        );
        let model = crate::model::Model {
            plugin_registry: Some(std::sync::Arc::new(vec![entry])),
            ..Default::default()
        };
        for (grant, on, check) in [
            ("block:fail_closed", true, 0),
            ("payload_events:PreToolUse", false, 1),
            ("host_api:messages.send", true, 2),
        ] {
            let PluginRequest::Trust {
                sha256,
                grants: Some(grants),
                tier: None,
                ..
            } = request(
                &model,
                PluginIntent::Grant {
                    candidate: "candidate".into(),
                    grant: grant.into(),
                    on,
                },
            )
            .unwrap()
            else {
                panic!("{grant}: not a re-trust with grants");
            };
            assert_eq!(sha256.as_deref(), Some(&*"a".repeat(64)));
            match check {
                0 => assert_eq!(
                    grants.block,
                    Some(bbt_proto::gui::PluginFailurePolicy::FailClosed)
                ),
                1 => assert!(grants.payload_events.is_empty()),
                _ => assert!(grants.host_api.contains("messages.send")),
            }
        }
    }
    #[test]
    fn stale_inventory_cannot_erase_a_more_recent_activation() {
        let current = entry(Some(3));
        assert_eq!(
            merge(std::slice::from_ref(&current), vec![entry(None)], true),
            vec![current.clone()]
        );
        assert_eq!(
            merge(std::slice::from_ref(&current), vec![], true),
            vec![current]
        );
        let fresh = entry(Some(4));
        assert_eq!(
            merge(&[entry(Some(3))], vec![fresh.clone()], true),
            vec![fresh]
        );
        assert!(merge(&[entry(None)], vec![], true).is_empty());
    }
    #[test]
    fn repairing_a_binary_does_not_keep_the_old_descriptor_refusal_as_current() {
        let mut value = serde_json::to_value(entry(Some(1))).unwrap();
        value["candidate"]["sha256"] = json!("b".repeat(64));
        value["activation"]["needs_retrust"] = json!(true);
        value["activation"]["inspection"] = json!({"shape":null,"refusal":{"code":"invalid_shape_json","message":"old binary"},"elapsed_ms":1,"stderr":"","stderr_truncated":false});
        let entry: PluginEntry = serde_json::from_value(value).unwrap();
        let view = plugin(&entry);
        assert!(view.actionable);
        assert_eq!(view.trust, "untrusted");
        assert_eq!(view.contract.ok, None);
        assert!(!view.health.say.unwrap().contains("old binary"));
    }
    #[test]
    fn plugin_intents_show_their_outcome_without_changing_observed_values_or_health() {
        let mut model = crate::model::Model {
            plugin_registry: Some(std::sync::Arc::new(vec![entry(None)])),
            ..Default::default()
        };
        let mut requests = Requests::default();
        let request = PluginRequest::Trust {
            path: entry(None).candidate.source,
            sha256: Some("a".repeat(64)),
            grants: None,
            tier: None,
        };
        let Some(crate::cockpit::UiMsg::PluginControl { id, .. }) =
            requests.issue(&model, request).unwrap()
        else {
            panic!()
        };
        let mut observed = Records::default();
        project(&mut observed, Some(&model));
        let before = serde_json::to_value(&observed.plugins[0]).unwrap();
        let mut pending = observed.clone();
        requests.decorate(&mut pending, &model);
        assert_eq!(pending.plugins[0].pending.as_ref().unwrap().verb, "trust");
        assert!(!pending.plugins[0].actionable);
        assert_eq!(pending.plugins[0].trust, observed.plugins[0].trust);
        assert_eq!(pending.plugins[0].enabled, observed.plugins[0].enabled);
        assert_eq!(
            serde_json::to_value(&pending.plugins[0].health).unwrap(),
            before["health"]
        );

        std::sync::Arc::make_mut(&mut model.operation_results).push(
            id,
            crate::messaging::Reply::Refused("plugin_changed_since_observed".into()),
        );
        let mut refused = observed.clone();
        requests.decorate(&mut refused, &model);
        let refusal = refused.plugins[0].refused.as_ref().unwrap();
        assert_eq!(refusal.verb, "trust");
        assert_eq!(refusal.reason, "plugin_changed_since_observed");
        assert!(refused.plugins[0].pending.is_none());
        assert_eq!(refused.plugins[0].trust, observed.plugins[0].trust);
        assert_eq!(refused.plugins[0].enabled, observed.plugins[0].enabled);
        assert_eq!(
            serde_json::to_value(&refused.plugins[0].health).unwrap(),
            before["health"]
        );

        std::sync::Arc::make_mut(&mut model.operation_results).push(
            id,
            crate::messaging::Reply::Unknown("transport lost".into()),
        );
        let mut unknown = observed;
        requests.decorate(&mut unknown, &model);
        assert!(unknown.plugins[0].refused.is_none());
        assert!(unknown.plugins[0].alerts[0].contains("trust outcome unknown"));
        assert_eq!(
            serde_json::to_value(&unknown.plugins[0].health).unwrap(),
            before["health"]
        );
    }
    #[test]
    fn unknown_plugin_effect_reuses_its_id_and_other_plugins_remain_independent() {
        let mut model = crate::model::Model::default();
        let mut requests = Requests::default();
        let first = PluginRequest::Enable {
            candidate: "a".into(),
        };
        let Some(crate::cockpit::UiMsg::PluginControl { id, .. }) =
            requests.issue(&model, first.clone()).unwrap()
        else {
            panic!()
        };
        assert!(requests.issue(&model, first.clone()).unwrap().is_none());
        std::sync::Arc::make_mut(&mut model.operation_results).requested(id);
        std::sync::Arc::make_mut(&mut model.operation_results).disconnected("lost reply");
        let Some(crate::cockpit::UiMsg::PluginControl { id: retry, .. }) =
            requests.issue(&model, first.clone()).unwrap()
        else {
            panic!()
        };
        assert_eq!(id, retry);
        assert!(requests.issue(&model, first.clone()).unwrap().is_none());
        let other = requests
            .issue(
                &model,
                PluginRequest::Enable {
                    candidate: "b".into(),
                },
            )
            .unwrap();
        assert!(
            matches!(other,Some(crate::cockpit::UiMsg::PluginControl {id:other,..}) if other!=id)
        );
        std::sync::Arc::make_mut(&mut model.operation_results)
            .push(id, crate::messaging::Reply::Plugins);
        assert!(
            matches!(requests.issue(&model,first).unwrap(),Some(crate::cockpit::UiMsg::PluginControl {id:next,..}) if next!=id)
        );
    }
    // keeps: workspace context — old provider and plugin snapshots disappear on a switch.
    #[test]
    fn settings_plugin_and_root_snapshots_are_fenced_by_swarm_and_revision() {
        use bbt_proto::gui::{KnowledgeProvider, KnowledgeRoot, KnowledgeStatus, SwarmScope};
        let a = bbt_ui_model::SwarmId::from_u128(1);
        let b = bbt_ui_model::SwarmId::from_u128(2);
        let mut model = crate::model::Model {
            swarm_metadata_loaded: true,
            ..Default::default()
        };
        model
            .ui
            .window
            .sidebar
            .swarms
            .push(bbt_ui_model::Swarm::new(a, "A", vec![]));
        model
            .ui
            .window
            .sidebar
            .swarms
            .push(bbt_ui_model::Swarm::new(b, "B", vec![]));
        model.ui.window.stage.swarm = Some(a);
        model.plugin_scope = Some(SwarmScope {
            swarm: bbt_mux::Ulid(1),
            revision: 0,
        });
        model.knowledge_scope = model.plugin_scope;
        model.plugin_registry = Some(std::sync::Arc::new(vec![entry(None)]));
        model.knowledge_roots = Some(std::sync::Arc::new(KnowledgeStatus {
            roots: vec![KnowledgeRoot {
                id: "a".into(),
                path: "/private/a/.bboxy".into(),
                provider: KnowledgeProvider {
                    id: "source-a".into(),
                    name: "A".into(),
                    sha256: None,
                },
                revision: 1,
                records: Some(1),
                process: None,
                refusal: None,
            }],
            offered: vec![],
        }));
        let mut visible = Records::default();
        project(&mut visible, Some(&model));
        assert_eq!(visible.plugins.len(), 2); // Selected source plus the bundled provider row.
        assert_eq!(visible.knowledge.roots.len(), 1);
        model.ui.window.stage.swarm = Some(b);
        let mut switched = Records::default();
        project(&mut switched, Some(&model));
        assert!(switched.plugins.is_empty());
        assert!(switched.knowledge.roots.is_empty());
        model.ui.window.stage.swarm = Some(a);
        model.swarm_metadata_pending.insert(a);
        let mut pending = Records::default();
        project(&mut pending, Some(&model));
        assert!(pending.plugins.is_empty());
        assert!(pending.knowledge.roots.is_empty());
    }
}
