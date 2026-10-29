// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Account-owned plugin controls. Setup and grant widening have no CLI in this cut.
use super::*;
use bbt_proto::gui::PluginRequest;

pub(super) const VERBS: &[Verb] = &[
    ("plugin", "list", "Plugin"),
    ("plugin", "trust", "Plugin"),
    ("plugin", "enable", "Plugin"),
    ("plugin", "disable", "Plugin"),
    ("plugin", "check", "Plugin"),
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
                PluginRequest::Trust {
                    path: path.to_str().ok_or("plugin path must be UTF-8")?.into(),
                    sha256: None,
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
            _ => return Ok(None),
        },
    }))
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
];
