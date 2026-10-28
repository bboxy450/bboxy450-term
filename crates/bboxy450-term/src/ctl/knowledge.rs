// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Account-owned root controls. Selecting a checked provider moves no files.
use super::*;
use bbt_proto::gui::{KnowledgeRequest, KnowledgeStatus};
pub(super) const VERBS: &[Verb] = &[
    ("knowledge", "status", "KnowledgeControl"),
    ("knowledge", "select", "KnowledgeControl"),
    ("knowledge", "reset", "KnowledgeControl"),
];
macro_rules! knowledge_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "knowledge": [
            KnowledgeStatus,
            KnowledgeSelect {root:String,provider:String},
            KnowledgeReset {root:String},
        ]] $($head)*);
    };
}
pub(super) use knowledge_commands;
pub(super) fn parse(words: &[&str], _cx: &ParseCx) -> Result<Option<Command>, String> {
    Ok(match *words {
        ["knowledge", "status"] => Some(Command::KnowledgeStatus),
        ["knowledge", "select", root, provider] => Some(Command::KnowledgeSelect {
            root: root.into(),
            provider: provider.into(),
        }),
        ["knowledge", "reset", root] => Some(Command::KnowledgeReset { root: root.into() }),
        _ => None,
    })
}
fn ask(client: &mut Client, request: KnowledgeRequest) -> Result<KnowledgeStatus, String> {
    let op = client.mint();
    match client.call(
        Some(op),
        GuiAction::KnowledgeControl { id: op, request },
        |reply| matches!(reply,GuiReply::KnowledgeControl {op:id,..} if *id==op),
    )? {
        GuiReply::KnowledgeControl { status, .. } => Ok(status),
        _ => unreachable!(),
    }
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
    let request = match command {
        Command::KnowledgeStatus => KnowledgeRequest::Status,
        Command::KnowledgeReset { root } => KnowledgeRequest::Reset { root },
        Command::KnowledgeSelect { root, provider } => {
            let status = ask(&mut client, KnowledgeRequest::Status)?;
            let root = status
                .roots
                .iter()
                .find(|r| r.id == root)
                .ok_or("unknown_knowledge_root: use an ID from knowledge status")?;
            let choice = status
                .offered
                .iter()
                .find(|p| p.id == provider || p.name == provider)
                .ok_or("provider_not_offered: trust, check and enable this data_source first")?;
            KnowledgeRequest::Select {
                root: root.id.clone(),
                provider: choice.id.clone(),
                revision: root.revision,
                sha256: choice.sha256.clone(),
            }
        }
        _ => return Err(no_flat_form()),
    };
    let status = ask(&mut client, request)?;
    print_reply(out, noun, verb, id, json!({"status":status}), human)
}
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "knowledge status",
    "knowledge select root candidate",
    "knowledge reset root",
];
