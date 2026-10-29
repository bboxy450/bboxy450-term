// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Window chrome through the GUI owner: `ctl ui …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("ui", "open", "Ui"),
    ("ui", "key", "Ui"),
    ("ui", "click", "Ui"),
    ("ui", "right-click", "Ui"),
    ("ui", "drag", "Ui"),
    ("ui", "select", "Ui"),
    ("ui", "shift-select", "Ui"),
    ("ui", "copy", "Ui"),
    ("ui", "clipboard-read", "Ui"),
    ("ui", "sidebar-scroll", "Ui"),
    ("ui", "launcher-scroll", "Ui"),
    ("ui", "focus", "Ui"),
    ("ui", "scroll", "Ui"),
    ("ui", "layout", "Ui"),
    ("ui", "clock", "Ui"),
    ("ui", "notification-click", "Ui"),
    ("ui", "compose", "Ui"),
    ("ui", "paste-image", "Ui"),
    ("ui", "swarm-workdir", "Ui"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! ui_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "ui": [
            Ui(bbt_proto::gui::UiRequest),
            UiFocus(String),
            UiScroll {
                seat: String,
                motion: bbt_proto::gui::UiScroll,
            },
            UiNotificationClick(String),
            UiCompose { to: String, text: String },
        ]] $($head)*);
    };
}
pub(super) use ui_commands;

/// `ctl ui …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    Ok(Some(match *words {
        ["ui", verb @ ("select" | "shift-select"), x0, y0, x1, y1] => {
            let values: Result<Vec<u32>, _> =
                [x0, y0, x1, y1].into_iter().map(str::parse).collect();
            let v = values.map_err(|_| {
                format!("{USAGE_PREFIX}ui select requires unsigned from-x from-y to-x to-y")
            })?;
            let request = bbt_proto::gui::UiRequest::TerminalSelect {
                from: [v[0], v[1]],
                to: [v[2], v[3]],
                shift: verb == "shift-select",
            };
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "copy"] => Command::Ui(bbt_proto::gui::UiRequest::Copy),
        ["ui", "clipboard-read"] => Command::Ui(bbt_proto::gui::UiRequest::ClipboardRead),
        ["ui", "drag", x0, y0, x1, y1] => {
            let values: Result<Vec<u32>, _> =
                [x0, y0, x1, y1].into_iter().map(str::parse).collect();
            let v = values.map_err(|_| {
                format!("{USAGE_PREFIX}ui drag requires unsigned from-x from-y to-x to-y")
            })?;
            let request = bbt_proto::gui::UiRequest::Drag {
                from: [v[0], v[1]],
                to: [v[2], v[3]],
            };
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "launcher-scroll", lines] => {
            let request = bbt_proto::gui::UiRequest::LauncherWheel {
                lines: lines
                    .parse()
                    .map_err(|_| format!("{USAGE_PREFIX}launcher-scroll requires signed lines"))?,
            };
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "sidebar-scroll", lines] => {
            let request = bbt_proto::gui::UiRequest::SidebarWheel {
                lines: lines
                    .parse()
                    .map_err(|_| format!("{USAGE_PREFIX}sidebar-scroll requires signed lines"))?,
            };
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "click", x, y] => {
            let request = bbt_proto::gui::UiRequest::Click {
                x: x.parse()
                    .map_err(|_| format!("{USAGE_PREFIX}ui click requires unsigned x y"))?,
                y: y.parse()
                    .map_err(|_| format!("{USAGE_PREFIX}ui click requires unsigned x y"))?,
            };
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "right-click", x, y] => {
            let request = bbt_proto::gui::UiRequest::RightClick {
                x: x.parse()
                    .map_err(|_| format!("{USAGE_PREFIX}ui right-click requires unsigned x y"))?,
                y: y.parse()
                    .map_err(|_| format!("{USAGE_PREFIX}ui right-click requires unsigned x y"))?,
            };
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "swarm-workdir", swarm, path] => {
            let swarm = swarm
                .parse::<bbt_mux::Ulid>()
                .map_err(|_| format!("{USAGE_PREFIX}expected swarm ULID"))?;
            let path = if path == "--default" || path == "default" {
                None
            } else {
                let folder = std::fs::canonicalize(path)
                    .map_err(|error| format!("{USAGE_PREFIX}folder: {error}"))?;
                if !folder.is_dir() {
                    return Err(format!("{USAGE_PREFIX}expected a directory"));
                }
                Some(folder.to_string_lossy().into_owned())
            };
            Command::Ui(bbt_proto::gui::UiRequest::SwarmWorkdir { swarm, path })
        }
        ["ui", "paste-image", path] => {
            let path = std::path::absolute(path).map_err(|error| format!("{USAGE_PREFIX}image path: {error}"))?;
            let request = bbt_proto::gui::UiRequest::PasteImage {path:path.to_string_lossy().into_owned()};
            request.validate().map_err(|error| format!("{USAGE_PREFIX}{error}"))?;
            Command::Ui(request)
        }
        ["ui", "compose"] => {
            let to = cx
                .flags
                .get("to")
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .ok_or_else(|| {
                    format!("{USAGE_PREFIX}ui compose requires --to <seat|role:r|channel:c>")
                })?;
            let text = cx
                .flags
                .get("text")
                .filter(|s| !s.trim().is_empty())
                .cloned()
                .ok_or_else(|| format!("{USAGE_PREFIX}ui compose requires --text <message>"))?;
            if to.chars().any(char::is_control) || to == "role:" || to == "channel:" {
                return Err(format!(
                    "{USAGE_PREFIX}ui compose requires a resolved seat, role or channel"
                ));
            }
            if text.len() > bbt_proto::messaging::MAX_TEXT {
                return Err(format!(
                    "{USAGE_PREFIX}ui compose text exceeds the message limit"
                ));
            }
            Command::UiCompose { to, text }
        }
        ["ui", "clock", "freeze"] => Command::Ui(bbt_proto::gui::UiRequest::Clock(
            bbt_proto::gui::UiClockCommand::Freeze,
        )),
        ["ui", "clock", "advance", ms] => Command::Ui(bbt_proto::gui::UiRequest::Clock(
            bbt_proto::gui::UiClockCommand::Advance {
                ms: ms.parse::<u64>().map_err(|_| {
                    format!("{USAGE_PREFIX}ui clock advance requires unsigned integer milliseconds")
                })?,
            },
        )),
        ["ui", "clock", "resume"] => Command::Ui(bbt_proto::gui::UiRequest::Clock(
            bbt_proto::gui::UiClockCommand::Resume,
        )),
        ["ui", "layout"] => Command::Ui(bbt_proto::gui::UiRequest::Layout),
        ["ui", "focus", seat] => Command::UiFocus(seat.into()),
        ["ui", "scroll", seat, motion] => Command::UiScroll {
            seat: seat.into(),
            motion: match motion {
                "top" => bbt_proto::gui::UiScroll::Top,
                "bottom" => bbt_proto::gui::UiScroll::Bottom,
                "page-up" => bbt_proto::gui::UiScroll::PageUp,
                "page-down" => bbt_proto::gui::UiScroll::PageDown,
                _ => {
                    return Err(format!(
                        "{USAGE_PREFIX}ui scroll <pane> top|bottom|page-up|page-down|row N"
                    ));
                }
            },
        },
        ["ui", "scroll", seat, "row", row] => Command::UiScroll {
            seat: seat.into(),
            motion: bbt_proto::gui::UiScroll::Row(row.parse().map_err(|_| {
                format!("{USAGE_PREFIX}ui scroll row requires an unsigned zero-based row")
            })?),
        },
        ["ui", "notification-click", seat] => Command::UiNotificationClick(seat.into()),
        ["ui", "open", target] => Command::Ui(bbt_proto::gui::UiRequest::Open(match target {
            "graph" => bbt_proto::gui::UiTarget::Graph,
            "settings" => bbt_proto::gui::UiTarget::Settings,
            "launcher" => bbt_proto::gui::UiTarget::Launcher,
            _ => {
                return Err(format!(
                    "{USAGE_PREFIX}ui open graph|settings|launcher|docs <path>"
                ));
            }
        })),
        ["ui", "open", "docs", path] => {
            let request =
                bbt_proto::gui::UiRequest::Open(bbt_proto::gui::UiTarget::Docs(path.into()));
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }
        ["ui", "key", ref keys @ ..] => {
            let request =
                bbt_proto::gui::UiRequest::Key(keys.iter().map(|s| s.to_string()).collect());
            request
                .validate()
                .map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Ui(request)
        }

        _ => return Ok(None),
    }))
}

/// This noun's noun-verb commands, run on the admitted connection; prints the one reply.
pub(super) fn run_verb(cx: super::VerbCx<'_>, command: Command) -> Result<(), String> {
    let super::VerbCx {
        mut client,
        out,
        noun,
        verb,
        id,
        human,
        ..
    } = cx;
    let data = match command {
        Command::UiCompose { to, text } => compose(&mut client, to, text)?,
        Command::Ui(request) => {
            serde_json::to_value(client.ui(request)?).map_err(|e| e.to_string())?
        }
        Command::UiFocus(seat) => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::Focus { seat })?)
                .map_err(|e| e.to_string())?
        }
        // The window replies with its first produced layout at the new size (ctl-cli §1a).
        Command::UiScroll { seat, motion } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::Scroll { seat, motion })?)
                .map_err(|e| e.to_string())?
        }
        Command::UiNotificationClick(seat) => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::NotificationClick { seat })?)
                .map_err(|e| e.to_string())?
        }
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "ui click 10 20",
    "ui right-click 10 20",
    "ui drag 258 100 418 100",
    "ui sidebar-scroll -3",
    "ui launcher-scroll -3",
    "ui open graph",
    "ui key cmd-t esc",
    "ui focus s",
    "ui scroll s top",
    "ui layout",
    "ui clock freeze",
    "ui notification-click s",
    "ui compose --to s --text hi",
    "ui paste-image /tmp/fixture.png",
    "ui swarm-workdir 00000000000000000000000001 default",
];

/// Resolve the explicit target and send it through the admitted window composer.
pub(super) fn compose(client: &mut Client, to: String, text: String) -> Result<Value, String> {
    use bbt_proto::gui::{UiCompose, UiComposeTarget, UiRequest};
    let to = if let Some(role) = to.strip_prefix("role:") {
        UiComposeTarget::Role(role.into())
    } else if let Some(channel) = to.strip_prefix("channel:") {
        UiComposeTarget::Channel(channel.into())
    } else {
        UiComposeTarget::Seat(resolve(&client.seats()?, &to)?.record.id)
    };
    serde_json::to_value(client.ui(UiRequest::Compose(UiCompose { to, text }))?)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_paste_parses_an_operator_path_without_opening_it() {
        let words = ["ui","paste-image","missing-fixture.png"].map(str::to_string);
        let parsed = super::super::parse_full(&words).unwrap();
        let Command::Ui(bbt_proto::gui::UiRequest::PasteImage {path}) = parsed.command else {panic!("typed operator paste")};
        assert!(std::path::Path::new(&path).is_absolute());
        assert!(path.ends_with("missing-fixture.png"));
        assert!(super::super::parse_full(&["ui".into(),"paste-image".into()]).is_err());
    }

    #[test]
    fn compose_requires_bounded_text_and_an_explicit_target() {
        let parse = |words: &[&str]| {
            super::super::parse_full(&words.iter().map(|s| (*s).into()).collect::<Vec<_>>())
        };
        let parsed = parse(&[
            "ui",
            "compose",
            "--to",
            "role:executor",
            "--text",
            "two words",
        ])
        .unwrap();
        assert!(
            matches!(parsed.command, Command::UiCompose {to, text} if to == "role:executor" && text == "two words")
        );
        for words in [
            vec!["ui", "compose"],
            vec!["ui", "compose", "--to", "role:executor"],
            vec!["ui", "compose", "--text", "hello"],
            vec!["ui", "compose", "--to", "role:", "--text", "hello"],
            vec![
                "ui", "compose", "--to", "seat", "--text", "hello", "--other", "x",
            ],
        ] {
            assert!(parse(&words).is_err(), "{words:?}");
        }
    }
}
