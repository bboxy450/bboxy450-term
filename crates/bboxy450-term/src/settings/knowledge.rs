// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Root observations and intents. The UI never opens a plugin or changes a TOML provider.
use crate::{cockpit::UiMsg, messaging::Reply, model::Model};
use bbt_proto::gui::{KnowledgeRequest, KnowledgeStatus};
use bbt_ui::modules::settings::{
    Records,
    controller::KnowledgeIntent,
    knowledge::{Knowledge, Root},
};

const BUNDLED: &str = "bboxy450-term-data_source-bboxy";
pub(super) fn project(out: &mut Records, status: &KnowledgeStatus) {
    out.knowledge_controls = true;
    out.knowledge = Knowledge {
        roots: status
            .roots
            .iter()
            .map(|root| Root {
                name: ".bboxy".into(),
                path: ".bboxy/ (current repository)".into(),
                provider: root.provider.name.clone(),
                records: root.records,
                answering: if root.refusal.is_some() {
                    "provider read unavailable".into()
                } else if let Some(pid) = root.process {
                    format!("{} · pid {pid}, over stdio", root.provider.name)
                } else if root.provider.id == BUNDLED {
                    "logic · in process".into()
                } else {
                    "provider has not answered yet".into()
                },
                digests: None,
                refused: root.refusal.clone(),
                pending: None,
            })
            .collect(),
        offered: status
            .offered
            .iter()
            .map(|provider| provider.name.clone())
            .collect(),
    };
    for root in &status.roots {
        if let Some(plugin) = out
            .plugins
            .iter_mut()
            .find(|p| p.candidate == root.provider.id)
        {
            plugin.root = Some(".bboxy".into());
            plugin.health.say=Some(root.refusal.clone().unwrap_or_else(|| match root.process {
                Some(pid)=>format!("Read-only root provider · last response from process {pid}; setup and external signed writing unavailable"),
                None=>"Root selected; no stdio response observed".into(),
            }));
        }
    }
    let mounted = status.roots.iter().any(|root| root.provider.id == BUNDLED);
    out.plugins.insert(
        0,
        bbt_ui::modules::settings::plugins::Plugin {
            measurements_unavailable: true,
            name: BUNDLED.into(),
            bundled: true,
            candidate: BUNDLED.into(),
            tier: "export".into(),
            local: true,
            trust: "trusted".into(),
            enabled: true,
            root: mounted.then(|| ".bboxy".into()),
            contract: bbt_ui::modules::settings::plugins::Contract {
                name: "data_source".into(),
                version: "1.0".into(),
                ok: None,
                failing: None,
            },
            health: bbt_ui::modules::settings::plugins::Health {
                state: "bundled".into(),
                say: Some("In-process default; host-signed writes available when selected.".into()),
            },
            ..Default::default()
        },
    );
}
pub fn request(model: &Model, intent: KnowledgeIntent) -> Result<KnowledgeRequest, String> {
    let status = model
        .knowledge_roots
        .as_ref()
        .ok_or("Knowledge root snapshot unavailable")?;
    let (name, provider) = match intent {
        KnowledgeIntent::Select { root, provider } => (root, Some(provider)),
        KnowledgeIntent::Reset { root } => (root, None),
    };
    if name != ".bboxy" {
        return Err("Unknown root".into());
    }
    let [root] = status.roots.as_slice() else {
        return Err("Current repository root unavailable".into());
    };
    let Some(provider) = provider else {
        return Ok(KnowledgeRequest::Reset {
            root: root.id.clone(),
        });
    };
    let provider = status
        .offered
        .iter()
        .find(|p| p.name == provider)
        .ok_or("Provider is no longer offered; refresh Knowledge")?;
    Ok(KnowledgeRequest::Select {
        root: root.id.clone(),
        provider: provider.id.clone(),
        revision: root.revision,
        sha256: provider.sha256.clone(),
    })
}
#[derive(Default)]
pub struct Requests(Vec<Pending>);
struct Pending {
    op: bbt_mux::Ulid,
    request: KnowledgeRequest,
    unknown: bool,
    serial: u64,
    uncertain: bool,
}
impl Requests {
    pub fn issue(
        &mut self,
        model: &Model,
        request: KnowledgeRequest,
    ) -> Result<Option<UiMsg>, String> {
        self.0.retain_mut(|pending| {
            let Some(row) = model
                .operation_results
                .rows()
                .find(|r| r.op == pending.op && r.serial > pending.serial)
            else {
                return true;
            };
            if matches!(row.reply, Reply::Unknown(_)) && pending.request.mutation() {
                pending.unknown = true;
                pending.uncertain = true;
                true
            } else if pending.uncertain
                && matches!(&row.reply,Reply::Refused(reason) if reason.starts_with("held:"))
            {
                pending.unknown = true;
                true
            } else {
                false
            }
        });
        let id = if let Some(pending) = self.0.iter_mut().find(|p| p.request == request) {
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
                return Err(
                    "knowledge_control_window_full: resolve retained operations first".into(),
                );
            }
            let op = bbt_mux::Ulid::new();
            self.0.push(Pending {
                op,
                request: request.clone(),
                unknown: false,
                serial: 0,
                uncertain: false,
            });
            op
        };
        Ok(Some(UiMsg::KnowledgeControl { id, request }))
    }
    pub fn decorate(&self, records: &mut Records, model: &Model) {
        for pending in self.0.iter().filter(|p| p.request.mutation()) {
            let reply = model
                .operation_results
                .rows()
                .find(|r| r.op == pending.op && r.serial > pending.serial)
                .map(|r| &r.reply);
            let to = match &pending.request {
                KnowledgeRequest::Select { provider, .. } => model
                    .knowledge_roots
                    .as_ref()
                    .and_then(|s| s.offered.iter().find(|p| p.id == *provider))
                    .map(|p| p.name.clone())
                    .unwrap_or_else(|| provider.clone()),
                KnowledgeRequest::Reset { .. } => BUNDLED.into(),
                KnowledgeRequest::Status => continue,
            };
            for root in &mut records.knowledge.roots {
                match reply {
                    None => {
                        root.pending =
                            Some(bbt_ui::modules::settings::knowledge::Pending { to: to.clone() })
                    }
                    Some(Reply::Refused(reason)) => {
                        root.refused = Some(format!(
                            "swap to {to} {} · {reason}; observed provider: {}",
                            if pending.uncertain {
                                "outcome unknown"
                            } else {
                                "refused"
                            },
                            root.provider
                        ))
                    }
                    Some(Reply::Unknown(reason)) => {
                        root.refused = Some(format!(
                            "swap to {to} outcome unknown · {reason}; retry retains the operation ID; observed provider: {}",
                            root.provider
                        ))
                    }
                    _ => {}
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use bbt_proto::gui::{KnowledgeProvider, KnowledgeRoot};
    use std::sync::Arc;
    fn model() -> Model {
        let mut model = Model::default();
        let bundled = KnowledgeProvider {
            id: BUNDLED.into(),
            name: BUNDLED.into(),
            sha256: None,
        };
        let external = KnowledgeProvider {
            id: "candidate".into(),
            name: "trusted stub".into(),
            sha256: Some("a".repeat(64)),
        };
        model.knowledge_roots = Some(Arc::new(KnowledgeStatus {
            roots: vec![KnowledgeRoot {
                id: "host-root".into(),
                path: "/private/root".into(),
                provider: bundled.clone(),
                revision: 7,
                records: Some(2),
                process: None,
                refusal: None,
            }],
            offered: vec![bundled, external],
        }));
        model
    }
    // keeps: knowledge-base §3 host-owned roots and pin-bound intents, no optimistic UI/TOML state.
    #[test]
    fn native_picker_emits_the_observed_pin_and_root_revision_without_mutating_the_view() {
        let model = model();
        let status = model.knowledge_roots.as_ref().unwrap();
        let mut records = Records::default();
        project(&mut records, status);
        let mut controller = bbt_ui::modules::settings::controller::Controller::new("Knowledge");
        let before = records.knowledge.roots[0].provider.clone();
        use bbt_ui::modules::settings::controller::{Action, Field};
        assert!(
            controller
                .handle(
                    Action::Toggle(Field::KnowledgeProvider(".bboxy".into())),
                    &mut records,
                    &Records::default()
                )
                .is_none()
        );
        let intent = controller.take_knowledge().unwrap();
        assert_eq!(records.knowledge.roots[0].provider, before);
        assert_eq!(
            request(&model, intent).unwrap(),
            KnowledgeRequest::Select {
                root: "host-root".into(),
                provider: "candidate".into(),
                revision: 7,
                sha256: Some("a".repeat(64))
            }
        );
        controller.handle(
            Action::Reset(Field::KnowledgeProvider(".bboxy".into())),
            &mut records,
            &Records::default(),
        );
        assert_eq!(
            request(&model, controller.take_knowledge().unwrap()).unwrap(),
            KnowledgeRequest::Reset {
                root: "host-root".into()
            }
        );
        assert_eq!(status.roots[0].revision, 7);
    }
    // keeps: ctl-cli op replay; an unknown provider effect keeps its ID and displayed provider.
    #[test]
    fn unknown_selection_reuses_the_operation_id_and_ignores_its_old_reply_serial() {
        let mut model = model();
        let mut requests = Requests::default();
        let request = KnowledgeRequest::Reset {
            root: "host-root".into(),
        };
        let Some(UiMsg::KnowledgeControl { id, .. }) =
            requests.issue(&model, request.clone()).unwrap()
        else {
            panic!()
        };
        Arc::make_mut(&mut model.operation_results).push(id, Reply::Unknown("no result".into()));
        let Some(UiMsg::KnowledgeControl { id: retry, .. }) =
            requests.issue(&model, request.clone()).unwrap()
        else {
            panic!()
        };
        assert_eq!(retry, id);
        assert!(requests.issue(&model, request.clone()).unwrap().is_none());
        let mut records = Records::default();
        project(&mut records, model.knowledge_roots.as_ref().unwrap());
        requests.decorate(&mut records, &model);
        assert_eq!(records.knowledge.roots[0].provider, BUNDLED);
        Arc::make_mut(&mut model.operation_results).push(id, Reply::KnowledgeControl);
        let Some(UiMsg::KnowledgeControl { id: next, .. }) =
            requests.issue(&model, request).unwrap()
        else {
            panic!()
        };
        assert_ne!(next, id);
    }
}
