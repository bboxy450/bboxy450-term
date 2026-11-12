// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Canvas scripts use the same scoped mux operations as MCP clients.
use super::*;

pub(super) const VERBS: &[Verb] = &[
    ("canvas", "open", "Knowledge"),
    ("canvas", "list", "Knowledge"),
    ("canvas", "draw", "Knowledge"),
    ("canvas", "read", "Knowledge"),
    ("canvas", "close", "Knowledge"),
    ("canvas", "archived-list", "Knowledge"),
    ("canvas", "restore", "Knowledge"),
];

macro_rules! canvas_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "canvas": [
            Canvas { method: String, args: Value, swarm: Option<Ulid> },
        ]] $($head)*);
    };
}
pub(super) use canvas_commands;

pub(super) fn parse(words: &[&str], cx: &ParseCx) -> Result<Option<Command>, String> {
    let (verb, source) = match *words {
        ["canvas", verb @ ("open" | "list" | "draw" | "read" | "close" | "restore"), source] => {
            (verb, source)
        }
        ["canvas", "archived-list", source] => ("archived_list", source),
        ["canvas", "list"] => ("list", "{}"),
        ["canvas", "archived-list"] => ("archived_list", "{}"),
        _ => return Ok(None),
    };
    if cx.on_screen_flag || cx.cells_flag || cx.unassigned_flag {
        return Err(format!(
            "{USAGE_PREFIX}canvas takes JSON arguments and --swarm"
        ));
    }
    if let Some(flag) = cx
        .flags
        .keys()
        .find(|flag| !["op", "as", "swarm"].contains(&flag.as_str()))
    {
        return Err(format!("{USAGE_PREFIX}--{flag} does not apply to canvas"));
    }
    if source.len() > bbt_mux::mcp::MAX_ARGS {
        return Err(format!(
            "{USAGE_PREFIX}canvas arguments exceed the MCP byte cap"
        ));
    }
    let args: Value = serde_json::from_str(source)
        .map_err(|error| format!("{USAGE_PREFIX}canvas arguments must be JSON: {error}"))?;
    if !args.is_object() || args.get("op_id").is_some() {
        return Err(format!(
            "{USAGE_PREFIX}canvas takes a JSON object; use --op for retries"
        ));
    }
    Ok(Some(Command::Canvas {
        method: format!("canvas_{verb}"),
        args,
        swarm: cx.swarm,
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
    let Command::Canvas {
        method,
        mut args,
        swarm,
    } = command
    else {
        return Err(no_flat_form());
    };
    // Scope lookup mints its own read ID; --op names only the canvas operation.
    let scope = client.workspace_scope(swarm)?;
    args["op_id"] = json!(id);
    let response = match client.call_workspace(
        id,
        GuiAction::Knowledge {
            id,
            scope,
            method,
            args,
        },
        |reply| matches!(reply, GuiReply::Knowledge { op, .. } if *op == id),
    )? {
        GuiReply::Knowledge { response, .. } => response,
        _ => unreachable!(),
    };
    let data = serde_json::from_value::<bbt_mux::knowledge::data_source::Result<Value>>(response)
        .map_err(|error| format!("invalid_canvas_result: {error}"))?
        .map_err(|error| error.to_string())?;
    print_reply(out, noun, verb, id, data, human)
}

#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "canvas open {\"title\":\"Example\"}",
    "canvas list",
    "canvas draw {\"id\":\"example\",\"ops\":[]}",
    "canvas read {\"id\":\"example\",\"image\":true}",
    "canvas close {\"id\":\"example\"}",
    "canvas archived-list",
    "canvas restore {\"id\":\"example\"}",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canvas_cli_preserves_mcp_arguments_scope_and_retry_id() {
        let id = Ulid::from(1_u128);
        let swarm = Ulid::from(2_u128);
        let args = json!({"title":"A kept canvas","tags":["plan"]});
        let parsed = parse_full(&[
            "canvas".into(),
            "open".into(),
            args.to_string(),
            "--swarm".into(),
            swarm.to_string(),
            "--op".into(),
            id.to_string(),
        ])
        .unwrap();
        assert_eq!(parsed.op, Some(id));
        assert_eq!(parsed.verb, Some(("canvas", "open")));
        assert_eq!(
            parsed.command,
            Command::Canvas {
                method: "canvas_open".into(),
                args,
                swarm: Some(swarm),
            }
        );
        let parsed = parse_full(&["canvas".into(), "list".into()]).unwrap();
        assert_eq!(
            parsed.command,
            Command::Canvas {
                method: "canvas_list".into(),
                args: json!({}),
                swarm: None,
            }
        );
    }

    #[test]
    fn canvas_cli_refuses_invalid_json_operation_injection_and_unrelated_flags() {
        for words in [
            vec!["canvas", "open", "[]"],
            vec!["canvas", "open", "{broken"],
            vec!["canvas", "draw", "{\"op_id\":\"caller\"}"],
            vec!["canvas", "read", "{}", "--on-screen"],
            vec!["canvas", "list", "--tag", "plan"],
            vec!["canvas", "list", "--root", "/caller/path"],
        ] {
            let words = words.into_iter().map(str::to_owned).collect::<Vec<_>>();
            assert!(parse_full(&words).is_err(), "accepted {words:?}");
        }
    }
}
