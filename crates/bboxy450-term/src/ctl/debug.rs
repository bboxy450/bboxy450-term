use super::*;

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
    ("debug", "logs", "Debug"),
    ("debug", "journal", "Debug"),
    ("debug", "metrics", "Debug"),
    ("debug", "fault", "Debug"),
    ("debug", "snapshot", "Debug"),
    ("debug", "ps", "Debug"),
    ("debug", "notifications", "App"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! debug_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "debug": [
            DebugNotifications,
            DebugFault {
                seat: String,
                from: u64,
                to: u64,
            },
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
        ["debug", "notifications"] => Command::DebugNotifications,
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
    "debug notifications",
    "debug logs app",
    "debug journal account/01M5Q8A7DQ7EQFKBZH4NXBW3QQ",
    "debug metrics",
    "debug fault ring-gap s 1..2",
    "debug snapshot /tmp/debug.json",
    "debug ps",
];
