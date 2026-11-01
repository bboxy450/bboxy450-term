// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed contract traits: implement one per contract with the request and
//! result types of `bboxy450-plugin`, and the SDK decodes arguments and
//! encodes results for you.
//!
//! - One contract: implement [`Base`] and the contract trait, then
//!   `run::<AsTaskSystem<Mine>>()` (likewise [`AsDataSource`],
//!   [`AsEventSink`], [`AsDecisionModel`], [`AsRepoGraph`], [`AsSession`],
//!   [`AsProvider`]).
//! - Several contracts in one binary: implement [`Plugin`] yourself and route
//!   each `call` with [`serve_task_system`], [`serve_event_sink`], … .
//!
//! Arguments that do not decode refuse `invalid_arguments`; a method the
//! trait does not know refuses `unknown_method`.

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::{AuthStatus, Contract, Ctx, HookEvent, HookReply, Init, Manifest, Plugin, Refusal};
use bbt_plugin::{data_source as ds, decision_model as dm, event_sink as es, provider as pv};
use bbt_plugin::{repo_graph as rg, session as ss, task_system as ts, transport as tr};

/// Everything but the contract methods: the descriptor, root binding and the
/// optional tool, hook, health and auth entry points (all with
/// [`Plugin`]'s defaults).
pub trait Base: Sized + 'static {
    /// The descriptor. The adapter adds its contract if it is missing.
    fn manifest() -> Manifest;
    /// Bind the root (see [`Plugin::initialize`]).
    fn initialize(init: Init, ctx: &Ctx) -> Result<Self, Refusal>;
    /// See [`Plugin::call_tool`].
    fn call_tool(&mut self, name: &str, _args: Value, _ctx: &Ctx) -> Result<Value, Refusal> {
        Err(Refusal::new(
            "unknown_tool",
            format!("this plugin does not serve tool {name}"),
        ))
    }
    /// See [`Plugin::on_hook`].
    fn on_hook(&mut self, _event: &HookEvent, _ctx: &Ctx) -> Result<HookReply, Refusal> {
        Ok(HookReply::Allow)
    }
    /// See [`Plugin::health`].
    fn health(&mut self, _ctx: &Ctx) -> Result<Value, Refusal> {
        Ok(serde_json::json!({"healthy": true}))
    }
    /// See [`Plugin::auth_status`].
    fn auth_status() -> Result<AuthStatus, Refusal> {
        Ok(AuthStatus::LoggedIn(String::new()))
    }
    /// See [`Plugin::auth_login`].
    fn auth_login() -> Result<(), Refusal> {
        Err(Refusal::new(
            "login_unavailable",
            "this plugin declares no login",
        ))
    }
}

fn typed<A: DeserializeOwned, R: Serialize>(
    args: Value,
    f: impl FnOnce(A) -> Result<R, Refusal>,
) -> Result<Value, Refusal> {
    let result = f(crate::args(args)?)?;
    serde_json::to_value(result).map_err(|e| Refusal::new("internal_error", e.to_string()))
}

fn unknown(contract: Contract, method: &str) -> Refusal {
    Refusal::new(
        "unknown_method",
        format!("{} has no method {method}", contract.name()),
    )
}

/// `data_source` 1.0: documents and their links under one root.
pub trait DataSource {
    /// A slug, path, ID or `#N` to one reference; `ambiguous_locator` with
    /// `candidates` when several match.
    fn resolve(&mut self, req: ds::ResolveRequest, ctx: &Ctx) -> Result<ds::RecordRef, Refusal>;
    /// One record, lossless (frontmatter and unrendered body).
    fn fetch(&mut self, req: ds::FetchRequest, ctx: &Ctx) -> Result<ds::Record, Refusal>;
    /// A ranked page of hits; `next` continues the same query.
    fn search(
        &mut self,
        req: ds::SearchRequest,
        ctx: &Ctx,
    ) -> Result<ds::Page<ds::SearchHit>, Refusal>;
    /// A page of references matching the filter.
    fn list(&mut self, req: ds::ListRequest, ctx: &Ctx)
    -> Result<ds::Page<ds::RecordRef>, Refusal>;
    /// Both directions of a record's links.
    fn links(&mut self, req: ds::FetchRequest, ctx: &Ctx) -> Result<ds::Links, Refusal>;
    /// A page of the document graph.
    fn graph(&mut self, req: ds::GraphRequest, ctx: &Ctx) -> Result<ds::GraphPage, Refusal>;
    /// Store a record (create with `path`, update with `ref` and `etag`).
    fn write(&mut self, req: ds::WriteRequest, ctx: &Ctx) -> Result<ds::WriteResult, Refusal>;
    /// References changed after `cursor`.
    fn changes_since(&mut self, req: ds::ChangesRequest, ctx: &Ctx)
    -> Result<ds::Changes, Refusal>;
    /// Optional `commit` (declare it with `Manifest::method`); refuses by default.
    fn commit(&mut self, _req: ds::CommitRequest, _ctx: &Ctx) -> Result<ds::CommitResult, Refusal> {
        Err(unknown(Contract::DataSource, "commit"))
    }
}

/// Route one `data_source` call to `plugin`.
pub fn serve_data_source<T: DataSource>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "resolve" => typed(args, |r| plugin.resolve(r, ctx)),
        "fetch" => typed(args, |r| plugin.fetch(r, ctx)),
        "search" => typed(args, |r| plugin.search(r, ctx)),
        "list" => typed(args, |r| plugin.list(r, ctx)),
        "links" => typed(args, |r| plugin.links(r, ctx)),
        "graph" => typed(args, |r| plugin.graph(r, ctx)),
        "write" => typed(args, |r| plugin.write(r, ctx)),
        "changes_since" => typed(args, |r| plugin.changes_since(r, ctx)),
        "commit" => typed(args, |r| plugin.commit(r, ctx)),
        _ => Err(unknown(Contract::DataSource, method)),
    }
}

/// `task_system` 1.0: tasks and their host-signed lifecycle. Every write
/// must return the request's `signed_record` byte for byte.
pub trait TaskSystem {
    fn list(&mut self, req: ts::ListRequest, ctx: &Ctx) -> Result<ts::TaskPage, Refusal>;
    fn status(&mut self, req: ts::StatusRequest, ctx: &Ctx) -> Result<ts::TaskRecord, Refusal>;
    fn changes_since(&mut self, req: ts::ChangesRequest, ctx: &Ctx)
    -> Result<ts::Changes, Refusal>;
    fn delegate(
        &mut self,
        req: ts::DelegateRequest,
        ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal>;
    fn reassign(
        &mut self,
        req: ts::ReassignRequest,
        ctx: &Ctx,
    ) -> Result<ts::TaskWriteResult, Refusal>;
    fn report(&mut self, req: ts::ReportRequest, ctx: &Ctx)
    -> Result<ts::TaskWriteResult, Refusal>;
    fn accept(&mut self, req: ts::AcceptRequest, ctx: &Ctx)
    -> Result<ts::TaskWriteResult, Refusal>;
    fn reject(&mut self, req: ts::RejectRequest, ctx: &Ctx)
    -> Result<ts::TaskWriteResult, Refusal>;
}

/// Route one `task_system` call to `plugin`.
pub fn serve_task_system<T: TaskSystem>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "list" => typed(args, |r| plugin.list(r, ctx)),
        "status" => typed(args, |r| plugin.status(r, ctx)),
        "changes_since" => typed(args, |r| plugin.changes_since(r, ctx)),
        "delegate" => typed(args, |r| plugin.delegate(r, ctx)),
        "reassign" => typed(args, |r| plugin.reassign(r, ctx)),
        "report" => typed(args, |r| plugin.report(r, ctx)),
        "accept" => typed(args, |r| plugin.accept(r, ctx)),
        "reject" => typed(args, |r| plugin.reject(r, ctx)),
        _ => Err(unknown(Contract::TaskSystem, method)),
    }
}

/// `event_sink` 1.0: host-only fan-out of signed envelopes. Deduplicate by
/// `envelope.id` and answer with that same `id`.
pub trait EventSink {
    fn deliver(&mut self, req: es::DeliverRequest, ctx: &Ctx)
    -> Result<es::DeliverResult, Refusal>;
}

/// Route one `event_sink` call to `plugin`.
pub fn serve_event_sink<T: EventSink>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "deliver" => typed(args, |r| plugin.deliver(r, ctx)),
        _ => Err(unknown(Contract::EventSink, method)),
    }
}

/// `decision_model` 1.0: choose one named criterion (oracle seats only).
/// `choice` and every `probabilities` key must name a supplied criterion.
pub trait DecisionModel {
    fn choose(&mut self, req: dm::ChooseRequest, ctx: &Ctx) -> Result<dm::ChooseResult, Refusal>;
}

/// Route one `decision_model` call to `plugin`.
pub fn serve_decision_model<T: DecisionModel>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "choose" => typed(args, |r| plugin.choose(r, ctx)),
        _ => Err(unknown(Contract::DecisionModel, method)),
    }
}

/// `repo_graph` 1.0: repositories under a root and their relations.
pub trait RepoGraph {
    fn describe(
        &mut self,
        req: rg::DescribeRequest,
        ctx: &Ctx,
    ) -> Result<rg::DescribeResult, Refusal>;
    fn changes_since(&mut self, req: rg::ChangesRequest, ctx: &Ctx)
    -> Result<rg::Changes, Refusal>;
}

/// Route one `repo_graph` call to `plugin`.
pub fn serve_repo_graph<T: RepoGraph>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "describe" => typed(args, |r| plugin.describe(r, ctx)),
        "changes_since" => typed(args, |r| plugin.changes_since(r, ctx)),
        _ => Err(unknown(Contract::RepoGraph, method)),
    }
}

/// `transport` 1.0: wrap an inner argv with the declared launch fields.
pub trait Transport {
    fn wrap(&mut self, req: tr::WrapRequest, ctx: &Ctx) -> Result<tr::WrapResult, Refusal>;
}
pub fn serve_transport<T: Transport>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "wrap" => typed(args, |r| plugin.wrap(r, ctx)),
        _ => Err(unknown(Contract::Transport, method)),
    }
}

/// `session` 0.1: describe external harness sessions.
pub trait Session {
    fn describe(
        &mut self,
        req: ss::DescribeRequest,
        ctx: &Ctx,
    ) -> Result<ss::DescribeResult, Refusal>;
}

/// Route one `session` call to `plugin`.
pub fn serve_session<T: Session>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "describe" => typed(args, |r| plugin.describe(r, ctx)),
        _ => Err(unknown(Contract::Session, method)),
    }
}

/// `provider` 0.1: describe hosts seats may run on.
pub trait Provider {
    fn describe(
        &mut self,
        req: pv::DescribeRequest,
        ctx: &Ctx,
    ) -> Result<pv::DescribeResult, Refusal>;
}

/// Route one `provider` call to `plugin`.
pub fn serve_provider<T: Provider>(
    plugin: &mut T,
    method: &str,
    args: Value,
    ctx: &Ctx,
) -> Result<Value, Refusal> {
    match method {
        "describe" => typed(args, |r| plugin.describe(r, ctx)),
        _ => Err(unknown(Contract::Provider, method)),
    }
}

macro_rules! adapter {
    ($(#[$doc:meta])* $name:ident, $trait:ident, $contract:expr, $serve:ident) => {
        $(#[$doc])*
        pub struct $name<T>(pub T);

        impl<T: Base + $trait> Plugin for $name<T> {
            fn manifest() -> Manifest {
                let manifest = T::manifest();
                if manifest.contracts.iter().any(|s| s.contract == $contract) {
                    manifest
                } else {
                    manifest.contract($contract)
                }
            }
            fn initialize(init: Init, ctx: &Ctx) -> Result<Self, Refusal> {
                T::initialize(init, ctx).map(Self)
            }
            fn call(
                &mut self,
                contract: Contract,
                method: &str,
                args: Value,
                ctx: &Ctx,
            ) -> Result<Value, Refusal> {
                if contract != $contract {
                    return Err(unknown(contract, method));
                }
                $serve(&mut self.0, method, args, ctx)
            }
            fn call_tool(&mut self, name: &str, args: Value, ctx: &Ctx) -> Result<Value, Refusal> {
                self.0.call_tool(name, args, ctx)
            }
            fn on_hook(&mut self, event: &HookEvent, ctx: &Ctx) -> Result<HookReply, Refusal> {
                self.0.on_hook(event, ctx)
            }
            fn health(&mut self, ctx: &Ctx) -> Result<Value, Refusal> {
                self.0.health(ctx)
            }
            fn auth_status() -> Result<AuthStatus, Refusal> {
                T::auth_status()
            }
            fn auth_login() -> Result<(), Refusal> {
                T::auth_login()
            }
        }
    };
}

adapter!(
    /// Serve a [`DataSource`] as a whole plugin.
    AsDataSource, DataSource, Contract::DataSource, serve_data_source
);
adapter!(
    /// Serve a [`TaskSystem`] as a whole plugin.
    AsTaskSystem, TaskSystem, Contract::TaskSystem, serve_task_system
);
adapter!(
    /// Serve an [`EventSink`] as a whole plugin.
    AsEventSink, EventSink, Contract::EventSink, serve_event_sink
);
adapter!(
    /// Serve a [`DecisionModel`] as a whole plugin.
    AsDecisionModel, DecisionModel, Contract::DecisionModel, serve_decision_model
);
adapter!(
    /// Serve a [`RepoGraph`] as a whole plugin.
    AsRepoGraph, RepoGraph, Contract::RepoGraph, serve_repo_graph
);
adapter!(
    /// Serve a [`Session`] as a whole plugin.
    AsSession, Session, Contract::Session, serve_session
);
adapter!(
    /// Serve a [`Provider`] as a whole plugin.
    AsProvider, Provider, Contract::Provider, serve_provider
);

adapter!(
    /// Serve a [`Transport`] as a whole plugin.
    AsTransport, Transport, Contract::Transport, serve_transport
);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    struct Echo;
    impl Base for Echo {
        fn manifest() -> Manifest {
            Manifest::new("bboxy450-term-repo_graph-echo", "Echo", "Test")
        }
        fn initialize(_: Init, _: &Ctx) -> Result<Self, Refusal> {
            Ok(Echo)
        }
    }
    impl RepoGraph for Echo {
        fn describe(
            &mut self,
            req: rg::DescribeRequest,
            _: &Ctx,
        ) -> Result<rg::DescribeResult, Refusal> {
            Ok(rg::DescribeResult {
                repos: vec![rg::Repo {
                    path: req.root,
                    kind: rg::RepoKind::Git,
                    name: "r".into(),
                }],
                edges: Vec::new(),
            })
        }
        fn changes_since(
            &mut self,
            req: rg::ChangesRequest,
            _: &Ctx,
        ) -> Result<rg::Changes, Refusal> {
            Ok(rg::Changes {
                changed: Vec::new(),
                next: req.cursor,
            })
        }
    }

    // keeps: P8 — typed traits decode arguments, encode results and refuse
    // by name; the adapter declares its contract.
    #[test]
    fn typed_contract_calls_decode_encode_and_refuse() {
        let ctx = Ctx::detached(Duration::from_secs(1));
        let mut plugin = AsRepoGraph(Echo);
        assert_eq!(
            plugin
                .call(
                    Contract::RepoGraph,
                    "describe",
                    json!({"root": "/r", "extra": 1}),
                    &ctx
                )
                .unwrap(),
            json!({"repos": [{"path": "/r", "kind": "git", "name": "r"}], "edges": []})
        );
        assert_eq!(
            plugin
                .call(Contract::RepoGraph, "describe", json!({}), &ctx)
                .unwrap_err()
                .code,
            "invalid_arguments"
        );
        assert_eq!(
            plugin
                .call(Contract::RepoGraph, "nope", json!({}), &ctx)
                .unwrap_err()
                .code,
            "unknown_method"
        );
        assert_eq!(
            plugin
                .call(Contract::Session, "describe", json!({}), &ctx)
                .unwrap_err()
                .code,
            "unknown_method"
        );
        let manifest = <AsRepoGraph<Echo> as Plugin>::manifest();
        assert_eq!(manifest.contracts.len(), 1);
        assert_eq!(manifest.contracts[0].contract, Contract::RepoGraph);
        manifest.validate().unwrap();
    }
}
