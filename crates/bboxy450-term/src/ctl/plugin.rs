// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Account-owned plugin controls. `plugin trust` takes the operator's grants;
//! setup has no CLI in this cut.
use super::*;
use bbt_proto::gui::PluginRequest;

pub(super) const VERBS: &[Verb] = &[
    ("plugin", "list", "Plugin"),
    ("plugin", "trust", "Plugin"),
    ("plugin", "enable", "Plugin"),
    ("plugin", "disable", "Plugin"),
    ("plugin", "check", "Plugin"),
    ("plugin", "setup", "Plugin"),
];

macro_rules! plugin_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "plugin": [
            Plugin { request: bbt_proto::gui::PluginRequest, swarm: Option<Ulid> },
        ]] $($head)*);
    };
}
pub(super) use plugin_commands;

pub(super) fn parse(words: &[&str], cx: &ParseCx) -> Result<Option<Command>, String> {
    Ok(Some(Command::Plugin {
        swarm: cx.swarm,
        request: match *words {
            ["plugin", "list"] => PluginRequest::List,
            ["plugin", "trust", path] => {
                let path = std::path::absolute(path).map_err(|error| error.to_string())?;
                let (grants, tier) = approval(
                    cx.flags.get("grant").map(String::as_str),
                    cx.flags.get("tier").map(String::as_str),
                )?;
                PluginRequest::Trust {
                    path: path.to_str().ok_or("plugin path must be UTF-8")?.into(),
                    sha256: None,
                    grants,
                    tier,
                }
            }
            ["plugin", "enable", candidate] => PluginRequest::Enable {
                candidate: candidate.into(),
            },
            ["plugin", "disable", candidate] => PluginRequest::Disable {
                candidate: candidate.into(),
            },
            ["plugin", "check", candidate] => PluginRequest::Check {
                candidate: candidate.into(),
            },
            ["plugin", "setup", candidate] => PluginRequest::Setup {
                candidate: candidate.into(),
            },
            _ => return Ok(None),
        },
    }))
}

/// `[--tier T] [--grant payload_events[:<event>]] [--grant block:<fail_open|fail_closed>]
/// [--grant host_api:<method>] [--grant messages_to:<destination>]…`. No flag
/// keeps the contract minimum; the host refuses a grant the shape never requested.
type Approval = (
    Option<bbt_proto::gui::PluginGrants>,
    Option<bbt_proto::gui::PluginTier>,
);
fn approval(grant: Option<&str>, tier: Option<&str>) -> Result<Approval, String> {
    use bbt_proto::gui::{PluginFailurePolicy as FailurePolicy, PluginTier as Tier};
    let tier = tier
        .map(|value| match value {
            "read" => Ok(Tier::Read),
            "write" => Ok(Tier::Write),
            "export" => Ok(Tier::Export),
            "transport" => Ok(Tier::Transport),
            _ => Err(format!("unknown tier {value}")),
        })
        .transpose()?;
    let mut grants = bbt_proto::gui::PluginGrants::default();
    let values: Vec<&str> = grant
        .map(|all| all.split('\n').collect())
        .unwrap_or_default();
    for value in &values {
        match value.split_once(':') {
            None if *value == "payload_events" => {
                grants
                    .payload_events
                    .insert(bbt_proto::gui::PLUGIN_ALL_REQUESTED.into());
            }
            Some(("payload_events", event)) if !event.is_empty() => {
                grants.payload_events.insert(event.into());
            }
            Some(("block", "fail_open")) => grants.block = Some(FailurePolicy::FailOpen),
            Some(("block", "fail_closed")) => grants.block = Some(FailurePolicy::FailClosed),
            Some(("host_api", method)) if !method.is_empty() => {
                grants.host_api.insert(method.into());
            }
            Some(("messages_to", destination)) if !destination.is_empty() => {
                grants.messages_to.insert(destination.into());
            }
            _ => {
                return Err(format!(
                    "unknown grant {value}: payload_events[:<event>], block:<fail_open|fail_closed>, host_api:<method>, messages_to:<destination>"
                ));
            }
        }
    }
    Ok(((!values.is_empty()).then_some(grants), tier))
}

pub(super) fn run_verb(cx: VerbCx<'_>, command: Command) -> Result<(), String> {
    let VerbCx {
        mut client,
        out,
        noun,
        verb,
        id,
        human,
        ..
    } = cx;
    let Command::Plugin { request, swarm } = command else {
        return Err(no_flat_form());
    };
    let scope = client.workspace_scope(swarm)?;
    let op = client.mint();
    let entries = match client.call_workspace(
        op,
        GuiAction::Plugin {
            scope,
            id: op,
            request,
        },
        |reply| matches!(reply, GuiReply::Plugin {op: id,..} if *id == op),
    )? {
        GuiReply::Plugin { entries, .. } => entries,
        _ => unreachable!(),
    };
    let entries: Vec<_> = entries.into_iter().map(|entry| {
        json!({"state":entry.state(),"candidate":entry.candidate,"activation":entry.activation,"trust":entry.trust,"conformance":entry.conformance})
    }).collect();
    print_reply(
        out,
        noun,
        verb,
        id,
        json!({"entries":entries,"setup":"unavailable"}),
        human,
    )
}

#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "plugin list",
    "plugin trust /private/plugins/bboxy450-term-data_source-stub",
    "plugin enable candidate",
    "plugin disable candidate",
    "plugin check candidate",
    "plugin setup candidate",
];
