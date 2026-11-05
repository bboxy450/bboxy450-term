// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Exact-seat Dream controls through the already admitted operator door.
use super::*;
use bbt_proto::dream as w;

pub(super) const VERBS: &[Verb] = &[
    ("dream", "inspect", "Dream"),
    ("dream", "status", "Dream"),
    ("dream", "cancel", "Dream"),
];
macro_rules! dream_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "dream": [
            DreamInspect {seat:String,sources:Vec<bbt_proto::dream::Selection>},
            DreamStatus {seat:String,run_id:bbt_proto::dream::Ulid,page:Option<u32>},
            DreamCancel {seat:String,run_id:bbt_proto::dream::Ulid},
        ]] $($head)*);
    };
}
pub(super) use dream_commands;

fn invalid_argument() -> String {
    format!(
        "{USAGE_PREFIX}dream needs one seat, canonical run/session IDs and exact nonzero source generations"
    )
}
fn selection(text: &str) -> Result<w::Selection, String> {
    let (session, generation_text) = text.split_once(':').ok_or_else(invalid_argument)?;
    let session = w::Ulid::try_from(session.to_owned()).map_err(|_| invalid_argument())?;
    let generation = generation_text.parse::<u64>().map_err(|_| invalid_argument())?;
    if generation == 0 || generation.to_string() != generation_text {
        return Err(invalid_argument());
    }
    let source_generation = w::Number::try_from(generation).map_err(|_| invalid_argument())?;
    Ok(w::Selection {
        session,
        source_generation,
    })
}
pub(super) fn parse(words: &[&str], cx: &ParseCx) -> Result<Option<Command>, String> {
    let command = match *words {
        ["dream", "inspect", seat, ref sources @ ..] => {
            if !(1..=32).contains(&sources.len()) {
                return Err(invalid_argument());
            }
            let sources = sources
                .iter()
                .map(|s| selection(s))
                .collect::<Result<Vec<_>, _>>()?;
            let unique: std::collections::BTreeSet<_> =
                sources.iter().map(|s| &s.session).collect();
            if unique.len() != sources.len() {
                return Err(invalid_argument());
            }
            Command::DreamInspect {
                seat: seat.into(),
                sources,
            }
        }
        ["dream", "status", seat, run] => Command::DreamStatus {
            seat: seat.into(),
            run_id: w::Ulid::try_from(run.to_owned()).map_err(|_| invalid_argument())?,
            page: None,
        },
        ["dream", "status", seat, run, page] => Command::DreamStatus {
            seat: seat.into(),
            run_id: w::Ulid::try_from(run.to_owned()).map_err(|_| invalid_argument())?,
            page: Some(page.parse().map_err(|_| invalid_argument())?),
        },
        ["dream", "cancel", seat, run] => Command::DreamCancel {
            seat: seat.into(),
            run_id: w::Ulid::try_from(run.to_owned()).map_err(|_| invalid_argument())?,
        },
        _ => return Ok(None),
    };
    if cx.on_screen_flag || cx.cells_flag || cx.unassigned_flag {
        return Err(invalid_argument());
    }
    let seat = match &command {
        Command::DreamInspect { seat, .. }
        | Command::DreamStatus { seat, .. }
        | Command::DreamCancel { seat, .. } => seat,
        _ => unreachable!(),
    };
    if seat.is_empty() || seat.len() > 256 || seat.contains('\0') {
        return Err(invalid_argument());
    }
    Ok(Some(command))
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
    let (seat, request) = match command {
        Command::DreamInspect { seat, sources } => (
            seat,
            w::Control::Inspect(w::Inspect {
                v: w::Version,
                op_id: id.into(),
                sources,
            }),
        ),
        Command::DreamStatus { seat, run_id, page } => (
            seat,
            w::Control::Status(w::Status {
                v: w::Version,
                run_id,
                page,
            }),
        ),
        Command::DreamCancel { seat, run_id } => (
            seat,
            w::Control::Cancel(w::Cancel {
                v: w::Version,
                op_id: id.into(),
                run_id,
            }),
        ),
        _ => return Err(no_flat_form()),
    };
    request.validate().map_err(|_| invalid_argument())?;
    // Roster lookup is read-only and consumes no Dream operation ID. The explicit
    // --op remains attached to the one admitted control action below.
    let seats = client.seats()?;
    let seat = resolve(&seats, &seat)?.record.id;
    let response = match client.call(
        Some(id),
        GuiAction::Dream { id, seat, request },
        |reply| matches!(reply,GuiReply::Dream {op,..} if *op==id),
    )? {
        GuiReply::Dream { response, .. } => response,
        _ => unreachable!(),
    };
    print_reply(
        out,
        noun,
        verb,
        id,
        serde_json::to_value(response).map_err(|_| "Dream reply serialization failed")?,
        human,
    )
}

#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "dream inspect Dream 00000000000000000000000001:1",
    "dream status Dream 00000000000000000000000002",
    "dream cancel Dream 00000000000000000000000002",
];
#[cfg(test)]
mod tests {
    use super::*;
    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_owned).collect()
    }
    #[test]
    fn dream_cli_preserves_sources_run_pages_and_original_operation() {
        let parsed = super::super::parse_full(&args(
            "dream inspect Dream 00000000000000000000000001:2 --op 00000000000000000000000003",
        ))
        .unwrap();
        assert_eq!(parsed.op, Some(Ulid::from(3_u128)));
        assert!(
            matches!(parsed.command,Command::DreamInspect {sources,..} if sources[0].source_generation.0==2)
        );
        let parsed =
            super::super::parse_full(&args("dream status Dream 00000000000000000000000002 7"))
                .unwrap();
        assert!(matches!(
            parsed.command,
            Command::DreamStatus { page: Some(7), .. }
        ));
    }
    #[test]
    fn dream_cli_refuses_empty_duplicate_unsafe_and_injected_scope() {
        for input in [
            "dream inspect Dream",
            "dream inspect Dream /caller/path:1",
            "dream inspect Dream 00000000000000000000000001:0",
            "dream inspect Dream 00000000000000000000000001:01",
            "dream inspect Dream 00000000000000000000000001:9007199254740992",
            "dream inspect Dream 00000000000000000000000001:1 00000000000000000000000001:2",
            "dream status Dream 00000000000000000000000002 -1",
            "dream status Dream 00000000000000000000000002 --root /caller/path",
            "dream cancel Dream 00000000000000000000000002 --swarm 00000000000000000000000003",
            "dream cancel Dream lowercase",
            "dream status Dream 00000000000000000000000002 --on-screen",
        ] {
            assert!(
                super::super::parse_full(&args(input)).is_err(),
                "accepted {input}"
            );
        }
    }
}
