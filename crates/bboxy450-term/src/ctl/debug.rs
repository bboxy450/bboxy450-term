// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use super::*;

pub(super) fn native_settings() -> Result<Value, String> {
    let settings = bboxy450_desktop_notifications::settings().map_err(|e| e.to_string())?;
    Ok(native_settings_value(settings))
}

fn native_settings_value(settings: bboxy450_desktop_notifications::Settings) -> Value {
    use bboxy450_desktop_notifications::{AlertStyle, Authorization, Setting};
    let setting = |value| match value {
        Setting::Enabled => "enabled",
        Setting::Disabled => "disabled",
        Setting::NotSupported => "not_supported",
        Setting::Unknown => "unknown",
    };
    let authorization = match settings.authorization {
        Authorization::Granted => "granted",
        Authorization::Denied => "denied",
        Authorization::Undetermined => "undetermined",
    };
    let alert_style = match settings.alert_style {
        AlertStyle::None => "none",
        AlertStyle::Banner => "banner",
        AlertStyle::Alert => "alert",
        AlertStyle::Unknown => "unknown",
    };
    json!({
        "backend": "native",
        "read_only": true,
        "os_presentation": false,
        "authorization": authorization,
        "alerts": setting(settings.alerts),
        "alert_style": alert_style,
        "notification_center": setting(settings.notification_center),
        "sound": setting(settings.sound),
        "alerts_configured": settings.authorization == Authorization::Granted
            && settings.alerts == Setting::Enabled
            && matches!(settings.alert_style, AlertStyle::Banner | AlertStyle::Alert),
    })
}

pub(super) fn read(
    client: &mut Client,
    request: bbt_proto::gui::DebugRequest,
    path: Option<&Path>,
) -> Result<Value, String> {
    let data = client.debug(request)?;
    let Some(path) = path else { return Ok(data) };
    let mut bytes =
        serde_json::to_vec_pretty(&data).map_err(|_| "debug snapshot serialization failed")?;
    bytes.push(b'\n');
    use std::os::unix::fs::OpenOptionsExt;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
    {
        Ok(mut file) => file
            .write_all(&bytes)
            .map_err(|_| "debug snapshot could not be written")?,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            // A retry can reuse its own identical artifact; never overwrite or
            // follow an existing symlink, including one to a key/capability file.
            let metadata = std::fs::symlink_metadata(path)
                .map_err(|_| "debug snapshot destination unavailable")?;
            if !metadata.is_file()
                || metadata.file_type().is_symlink()
                || metadata.len() != bytes.len() as u64
            {
                return Err(
                    "debug snapshot destination exists with different content; choose a new file"
                        .into(),
                );
            }
            let mut existing = Vec::new();
            std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)
                .and_then(|file| file.take(bytes.len() as u64 + 1).read_to_end(&mut existing))
                .map_err(|_| "debug snapshot destination unavailable")?;
            if existing != bytes {
                return Err(
                    "debug snapshot destination exists with different content; choose a new file"
                        .into(),
                );
            }
        }
        Err(_) => {
            return Err(
                "debug snapshot destination unavailable; choose a writable new file".into(),
            );
        }
    }
    Ok(
        json!({"path":path,"bytes":bytes.len(),"sampled_at_ms":data["sampled_at_ms"],
        "seats":data["seats"].as_array().map_or(0,Vec::len),"omitted_seats":data["omitted_seats"]}),
    )
}

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("debug", "report", "-"),
    ("debug", "logs", "Debug"),
    ("debug", "errors", "Debug"),
    ("debug", "journal", "Debug"),
    ("debug", "metrics", "Debug"),
    ("debug", "history-admission", "-"),
    ("debug", "fault", "Debug"),
    ("debug", "snapshot", "Debug"),
    ("debug", "ps", "Debug"),
    ("debug", "notifications", "App"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! debug_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "debug": [
            DebugReport,
            DebugHistoryAdmission { cols: usize, rows: usize },
            DebugNotifications,
            DebugNativeNotifications,
            DebugFault {
                seat: String,
                from: u64,
                to: u64,
            },
            DebugSpawnGate(bbt_proto::gui::UiSpawnGateCommand),
            Debug {
                request: bbt_proto::gui::DebugRequest,
                snapshot: Option<PathBuf>,
            },
        ]] $($head)*);
    };
}
pub(super) use debug_commands;

/// `ctl debug …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    Ok(Some(match *words {
        ["debug", "report"] => Command::DebugReport,
        ["debug", "history-admission", size] => {
            let (cols, rows) = size.split_once('x')
                .and_then(|(c, r)| Some((c.parse::<usize>().ok()?, r.parse::<usize>().ok()?)))
                .filter(|(c, r)| *c >= 2 && *r > 0 && c.checked_mul(*r).is_some_and(|n| n <= 262_144))
                .ok_or_else(|| format!("{USAGE_PREFIX}history-admission: core-valid COLSxROWS, at most 262144 cells"))?;
            Command::DebugHistoryAdmission { cols, rows }
        }
        ["debug", "notifications"] => Command::DebugNotifications,
        ["debug", "notifications", "native"] => Command::DebugNativeNotifications,
        ["debug", "errors"] => Command::Debug {
            request: bbt_proto::gui::DebugRequest::Errors {
                since: flag("since")
                    .map(|s| {
                        s.parse().map_err(|_| {
                            format!("{USAGE_PREFIX}debug errors --since: Unix milliseconds")
                        })
                    })
                    .transpose()?,
                seat: flag("seat")
                    .map(|s| {
                        s.parse()
                            .map_err(|_| format!("{USAGE_PREFIX}debug errors --seat: seat ID"))
                    })
                    .transpose()?,
            },
            snapshot: None,
        },
        ["debug", "logs", process] => Command::Debug {
            request: bbt_proto::gui::DebugRequest::Logs {
                process: match process {
                    "app" => bbt_proto::gui::DebugProcess::App,
                    "logic" => bbt_proto::gui::DebugProcess::Logic,
                    "supervisor" => bbt_proto::gui::DebugProcess::Supervisor,
                    _ => {
                        return Err(format!(
                            "{USAGE_PREFIX}debug logs: app, logic or supervisor"
                        ));
                    }
                },
                since: flag("since")
                    .map(|s| {
                        s.parse::<u64>()
                            .map_err(|_| format!("{USAGE_PREFIX}debug logs --since: byte offset"))
                    })
                    .transpose()?
                    .unwrap_or(0),
            },
            snapshot: None,
        },
        ["debug", "journal", query] => {
            bbt_mux::debug::journal_query(query).map_err(|e| format!("{USAGE_PREFIX}{e}"))?;
            Command::Debug {
                request: bbt_proto::gui::DebugRequest::Journal {
                    query: query.into(),
                },
                snapshot: None,
            }
        }
        ["debug", "fault", "ring-gap", seat, range] => {
            let (from, to) = range.split_once("..").ok_or_else(|| {
                format!("{USAGE_PREFIX}ring-gap needs FROM..TO capture sequences")
            })?;
            let from = from
                .parse::<u64>()
                .map_err(|_| format!("{USAGE_PREFIX}invalid gap start"))?;
            let to = to
                .parse::<u64>()
                .map_err(|_| format!("{USAGE_PREFIX}invalid gap end"))?;
            if from == 0 || to < from || to - from >= 4096 {
                return Err(format!(
                    "{USAGE_PREFIX}ring-gap needs 1..4096 positive capture sequences"
                ));
            }
            Command::DebugFault {
                seat: (*seat).into(),
                from,
                to,
            }
        }
        ["debug", "fault", "spawn-hold"] => {
            Command::DebugSpawnGate(bbt_proto::gui::UiSpawnGateCommand::Arm)
        }
        ["debug", "fault", "spawn-release", op] => {
            let op = op
                .parse()
                .map_err(|_| format!("{USAGE_PREFIX}spawn-release needs an operation ID"))?;
            let request =
                bbt_proto::gui::UiRequest::SpawnGate(bbt_proto::gui::UiSpawnGateCommand::Release {
                    op,
                });
            request
                .validate()
                .map_err(|reason| format!("{USAGE_PREFIX}{reason}"))?;
            let bbt_proto::gui::UiRequest::SpawnGate(command) = request else {
                unreachable!()
            };
            Command::DebugSpawnGate(command)
        }
        ["debug", "metrics"] => Command::Debug {
            request: bbt_proto::gui::DebugRequest::Metrics,
            snapshot: None,
        },
        ["debug", "ps"] => Command::Debug {
            request: bbt_proto::gui::DebugRequest::Ps,
            snapshot: None,
        },
        ["debug", "snapshot", path] => {
            let path = std::path::absolute(path)
                .map_err(|_| format!("{USAGE_PREFIX}debug snapshot requires a local file path"))?;
            let destination = path
                .to_str()
                .filter(|s| s.len() <= 4096)
                .ok_or_else(|| {
                    format!("{USAGE_PREFIX}debug snapshot path must be UTF-8, at most 4096 bytes")
                })?
                .to_owned();
            Command::Debug {
                request: bbt_proto::gui::DebugRequest::Snapshot { destination },
                snapshot: Some(path),
            }
        }
        _ => return Ok(None),
    }))
}

/// Static harness guidance, available without opening an endpoint or collecting data.
pub(super) fn report() -> Value {
    json!({"repository":"bboxy450/bboxy450-term", "hint":r#"Use the gh CLI to report an observed bboxy450-term bug. This command only returns instructions; the harness gathers evidence and submits the issue.

1. Search before filing. Group repeated errors by operation and error kind, count occurrences, and record the time window. Search open AND closed issues:
   gh issue list --repo bboxy450/bboxy450-term --state all --search '<distinctive error or feature>' --json number,title,state,url
   gh issue view <number> --repo bboxy450/bboxy450-term --json number,title,state,body,url,comments
   Add new evidence to an existing matching issue. File a new issue for a different cause. Keep a bag of bugs in one issue when requested, with separate findings and evidence for each.

2. Establish the environment. Record UTC date, macOS version (sw_vers), architecture (uname -m), harness, affected operation, and observed seat/swarm/run scope. Read the actual installed app's Contents/Info.plist: CFBundleShortVersionString, CFBundleVersion and BBOXYSourceRevision. Hash the affected executable (shasum -a 256). Record git rev-parse HEAD and git status --short for inspected source, and distinguish that checkout from the installed/running build. Do not assume the terminal supports --version or that an installed build proves the running owner's version. Include relevant plugin/backend versions and hashes.

3. Gather bounded diagnostics through authorized interfaces. An outside operator can use bboxy450-term ctl app status, debug errors --since <UNIX_MS> [--seat <ID>], debug logs <app|logic|supervisor> --since <BYTE_OFFSET>, debug metrics, and debug ps, selecting the intended --run-dir. Follow returned cursors and report truncation. An unfiltered debug errors read marks retained errors read. For a retained operation, use debug journal <account/OP|seat/SEAT/OP>; unknown outcomes must be inspected before retrying a mutation. A seat uses only its admitted MCP tools: if diagnostics are unavailable, report that gap and search for an existing issue; never bypass ctl ancestry/grants through sockets, endpoint files or backing stores. Do not change live state merely to collect a report.

4. Reproduce only within the authorized scope, using private fixtures where possible. Record exact commands/tool arguments, actual exit codes, relevant error kinds, timestamps, and concise sanitized output. Separate observations, reproduction results, inferred causes and untested claims. A bare exit 0 or harness summary does not prove the requested behavior. Do not claim unseen errors were triaged. Do not attach capabilities, tokens, credentials, full environments, private document bodies or raw transcripts; summarize and redact evidence before upload.

5. Write a local issue body with these fields:
   Problem and user impact; version/build/source and environment; minimal numbered reproduction; expected vs actual behavior; findings with severity, frequency and evidence (file:line plus mechanism or command plus result); related issues; checks that passed/failed; untested scope and proposed next verification. Keep facts sufficient for someone without the local logs to continue. Describe a hypothesis as a hypothesis, and do not mark an issue fixed without verification.

6. Submit the prepared body, preserving literal newlines:
   gh issue create --repo bboxy450/bboxy450-term --title '<trigger>: <observed failure>' --body-file /absolute/path/report.md
   Then verify the returned issue:
   gh issue view <number> --repo bboxy450/bboxy450-term --json number,title,state,body,url
   Return the confirmed URL. If submission has an unknown outcome, query for the original issue before creating another. If gh authentication or diagnostics are unavailable, retain the local draft and report the precise blocker."#})
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
        Command::DebugReport => report(),
        Command::DebugHistoryAdmission { cols, rows } => {
            client.seats()?;
            if !client
                .identity
                .as_ref()
                .and_then(|i| i.reason.as_deref())
                .is_some_and(|r| r.starts_with("test-account:"))
            {
                return Err("history_admission_requires_test_account".into());
            }
            history_admission(cols, rows)
        }
        Command::DebugSpawnGate(command) => {
            serde_json::to_value(client.ui(bbt_proto::gui::UiRequest::SpawnGate(command))?)
                .map_err(|error| error.to_string())?
        }
        Command::DebugFault { seat, from, to } => {
            let seat = resolve(&client.seats()?, &seat)?.record.id;
            client.debug(bbt_proto::gui::DebugRequest::RingGap { seat, from, to })?
        }
        Command::Debug { request, snapshot } => {
            debug::read(&mut client, request, snapshot.as_deref())?
        }
        Command::DebugNotifications => {
            client
                .app(bbt_proto::gui::AppRequest::Notifications)?
                .notifications
        }
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "debug report",
    "debug history-admission 2048x128",
    "debug notifications",
    "debug logs app",
    "debug errors",
    "debug journal account/01M5Q8A7DQ7EQFKBZH4NXBW3QQ",
    "debug metrics",
    "debug fault ring-gap s 1..2",
    "debug snapshot /tmp/debug.json",
    "debug ps",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_accepts_no_collection_or_submission_arguments() {
        for words in [
            vec!["debug", "report", "extra"],
            vec!["debug", "report", "--seat", "00000000000000000000000001"],
        ] {
            assert!(
                super::super::parse_full(&words.into_iter().map(str::to_owned).collect::<Vec<_>>())
                    .is_err()
            );
        }
    }

    #[test]
    fn native_notification_settings_do_not_equate_permission_with_visible_alerts() {
        use bboxy450_desktop_notifications::{AlertStyle, Authorization, Setting, Settings};
        for (authorization, alerts, alert_style, configured) in [
            (
                Authorization::Granted,
                Setting::Enabled,
                AlertStyle::Banner,
                true,
            ),
            (
                Authorization::Granted,
                Setting::Enabled,
                AlertStyle::Alert,
                true,
            ),
            (
                Authorization::Granted,
                Setting::Disabled,
                AlertStyle::Banner,
                false,
            ),
            (
                Authorization::Granted,
                Setting::Enabled,
                AlertStyle::None,
                false,
            ),
            (
                Authorization::Granted,
                Setting::Unknown,
                AlertStyle::Unknown,
                false,
            ),
            (
                Authorization::Denied,
                Setting::Enabled,
                AlertStyle::Banner,
                false,
            ),
            (
                Authorization::Undetermined,
                Setting::Enabled,
                AlertStyle::Banner,
                false,
            ),
        ] {
            let value = native_settings_value(Settings {
                authorization,
                alerts,
                alert_style,
                notification_center: Setting::Enabled,
                sound: Setting::NotSupported,
            });
            assert_eq!(value["alerts_configured"], configured);
            assert_eq!(value["read_only"], true);
            assert_eq!(value["os_presentation"], false);
            assert_eq!(value["sound"], "not_supported");
        }
    }

    #[test]
    fn native_notification_diagnostic_is_explicit_and_has_no_extra_flags() {
        let parse = |text: &str| {
            super::super::parse_full(
                &text
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
        };
        assert!(matches!(
            parse("debug notifications").unwrap().command,
            Command::DebugNotifications
        ));
        assert!(matches!(
            parse("debug notifications native").unwrap().command,
            Command::DebugNativeNotifications
        ));
        assert!(parse("debug notifications unknown").is_err());
        assert!(parse("debug notifications native --authorize yes").is_err());
        assert!(parse("debug notifications native --post yes").is_err());
    }

    // keeps: e2e-script-map §L2 — the existing operator debug noun owns fixture controls.
    #[test]
    fn spawn_gate_parser_keeps_the_held_operation_and_rejects_nil_release() {
        let parse = |words: &[&str]| {
            super::super::parse_full(
                &words
                    .iter()
                    .map(|word| (*word).to_owned())
                    .collect::<Vec<_>>(),
            )
        };
        let arm = parse(&["debug", "fault", "spawn-hold"]).unwrap();
        assert!(matches!(
            arm.command,
            Command::DebugSpawnGate(bbt_proto::gui::UiSpawnGateCommand::Arm)
        ));
        let release = parse(&[
            "debug",
            "fault",
            "spawn-release",
            "00000000000000000000000001",
        ])
        .unwrap();
        assert!(
            matches!(release.command, Command::DebugSpawnGate(bbt_proto::gui::UiSpawnGateCommand::Release { op }) if op == Ulid(1))
        );
        assert!(
            parse(&[
                "debug",
                "fault",
                "spawn-release",
                "00000000000000000000000000"
            ])
            .is_err()
        );
    }
}

// A core-valid fixture, independent of the live Replica's 32768-cell limit.
// Uses real allocated cells and the production admission path; no fake Screen dimensions.
fn history_admission(cols: usize, rows: usize) -> Value {
    let term = bbt_core::Term::new_with_spare_rows(cols, rows, 0, 0);
    let core_heap = term.retained_heap_bytes();
    let screen = std::sync::Arc::new(bbt_proto::Screen::from_term(
        &term,
        String::new(),
        None,
        None,
    ));
    let view = bbt_proto::View {
        seat: Ulid::new(),
        generation: 1,
        view_generation: Ulid::new(),
    };
    let result = crate::scrollback::Playback::new(view, 0, screen);
    let (refusal, heap, budget) = match result {
        Ok(playback) => (
            None,
            playback.retained_heap_bytes(),
            Some(playback.heap_budget()),
        ),
        Err(reason) => (Some(reason), Some(0), None),
    };
    json!({"basis":"core-valid allocated fixture; independent of live GUI dimensions",
        "cols":cols,"rows":rows,"cells":cols * rows,"core_retained_heap_bytes":core_heap,
        "refusal":refusal,"retained_heap_bytes":heap,"budget_bytes":budget,
        "maximum_budget_bytes":crate::scrollback::MAX_HEAP_BUDGET})
}

#[cfg(test)]
mod admission_tests {
    use super::*;
    #[test]
    fn allocated_core_viewport_refuses_over_budget_and_reports_heap() {
        let fitting = history_admission(80, 24);
        assert!(fitting["refusal"].is_null());
        assert!(fitting["retained_heap_bytes"].as_u64().unwrap() > 0);
        let large = history_admission(2048, 128);
        assert_eq!(large["refusal"], "history_viewport_exceeds_budget");
        assert!(large["core_retained_heap_bytes"].as_u64().unwrap() > 0);
        assert_eq!(large["retained_heap_bytes"], 0);
    }
}
