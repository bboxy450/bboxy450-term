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
        match &model.plugin_registry {
            Some(entries) => {
                out.plugins = entries.iter().map(plugin).collect();
                out.plugin_controls = true;
                out.plugin_notice = Some("Production account registry. Trust pins one binary at its minimum tier with no additional grants; enable is separate. Run ctl plugin check before choosing a Knowledge provider. Setup, hooks and external signed writing remain unavailable.".into());
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
        if let Some(status) = model.knowledge_roots.as_deref() {
            super::knowledge::project(out, status);
        }
    }
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
    Plugin {
        measurements_unavailable: true,
        candidate: entry.candidate.id.clone(),
        actionable: matches!(
            state,
            State::Untrusted | State::NeedsRetrust | State::Disabled | State::Enabled
        ) && refusal.is_none(),
        name: std::path::Path::new(&entry.candidate.source)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned(),
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
        grant: approved.map(|_| "no additional grants".into()),
        version: shape.map(|s| s.version.clone()),
        auth: shape
            .and_then(|s| s.auth.as_ref())
            .map(|_| "setup unavailable".into()),
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
        trace: shape.is_some_and(|s| s.trace),
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
                    | PluginRequest::Check { candidate } => *candidate == plugin.candidate,
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
        assert_eq!(plugin.contract.ok, None);
        assert!(!out.plugin_notice.as_ref().unwrap().contains("examples"));
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
}
