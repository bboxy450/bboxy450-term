//! Window chrome through the GUI owner: `ctl ui …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("ui", "open", "Ui"),
    ("ui", "key", "Ui"),
    ("ui", "focus", "Ui"),
    ("ui", "scroll", "Ui"),
    ("ui", "layout", "Ui"),
    ("ui", "clock", "Ui"),
    ("ui", "notification-click", "Ui"),
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
        ]] $($head)*);
    };
}
pub(super) use ui_commands;

/// `ctl ui …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], _cx: &super::ParseCx) -> Result<Option<Command>, String> {
    Ok(Some(match *words {
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
