// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Task records and their lifecycle: `ctl task …`.
use super::*;

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("task", "list", "Task"),
    ("task", "get", "Task"),
    ("task", "status", "Task"),
    ("task", "report", "Task"),
    ("task", "accept", "Task"),
    ("task", "reject", "Task"),
    ("task", "reconcile", "Task"),
    ("task", "import", "Task"),
    ("task", "import-context", "Task"),
    ("task", "delegate", "Task"),
    ("task", "create", "RecordWrite"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! task_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "task": [
            Delegate {
                task: String,
                seat: String,
            },
            Reports {
                since: Option<String>,
            },
            /// `task list [--unassigned | --seat <s> | --status <st>]`.
            TaskList {
                unassigned: bool,
                seat: Option<String>,
                status: Option<String>,
            },
            /// `task create <title> --verify CMD [--brief TEXT]`.
            TaskCreate {
                title: String,
                verify: String,
                brief: Option<String>,
            },
            /// `task get` and `task status <#N|path|id>` use the same task read.
            TaskGet {
                task: String,
            },
            /// `task report <#N|path|id> --summary TEXT [--commit SHA] [--verify-result JSON]`.
            TaskReport {
                task: String,
                summary: String,
                commit: Option<String>,
                verify_result: Option<TaskVerification>,
            },
            TaskAccept {
                task: String,
            },
            TaskReject {
                task: String,
                reason: String,
            },
            TaskReconcile { task: String, disposition: String, reason: String, evidence: Vec<String> },
            TaskImport { manifest: String, audit: String, source_dir: String, expected_root: String, owner_seat: Ulid },
            TaskImportContext {},
        ]] $($head)*);
    };
}
pub(super) use task_commands;

/// `ctl task …` and its pre-grammar aliases, parsed into this noun's commands; `None`
/// when the words are not this noun's.
pub(super) fn parse(words: &[&str], cx: &super::ParseCx) -> Result<Option<Command>, String> {
    let flags = cx.flags;
    let flag = |name: &str| flags.get(name).cloned();
    let unassigned_flag = cx.unassigned_flag;
    Ok(Some(match *words {
        ["delegate", task, seat] => Command::Delegate {
            task: task.into(),
            seat: seat.into(),
        },
        ["reports"] => Command::Reports {
            since: flag("since"),
        },
        ["task", "delegate", task, seat] => Command::Delegate {
            task: task.into(),
            seat: seat.into(),
        },
        ["task", "list"] => Command::TaskList {
            unassigned: unassigned_flag,
            seat: flag("seat"),
            status: flag("status"),
        },
        ["task", "get" | "status", task] => Command::TaskGet { task: task.into() },
        ["task", "import-context"] => Command::TaskImportContext {},
        ["task", "report", task] => Command::TaskReport {
            task: task.into(),
            summary: flag("summary")
                .ok_or_else(|| format!("{USAGE_PREFIX}task report needs --summary TEXT"))?,
            commit: flag("commit"),
            verify_result: flag("verify-result")
                .map(|value| {
                    serde_json::from_str(&value)
                        .map_err(|e| format!("{USAGE_PREFIX}task report --verify-result: {e}"))
                })
                .transpose()?,
        },
        ["task", "accept", task] => Command::TaskAccept { task: task.into() },
        ["task", "reject", task] => Command::TaskReject {
            task: task.into(),
            reason: flag("reason")
                .ok_or_else(|| format!("{USAGE_PREFIX}task reject needs --reason TEXT"))?,
        },
        ["task", "reconcile", task] => Command::TaskReconcile {
            task: task.into(),
            disposition: flag("disposition")
                .ok_or_else(|| format!("{USAGE_PREFIX}task reconcile needs --disposition"))?,
            reason: flag("reason")
                .ok_or_else(|| format!("{USAGE_PREFIX}task reconcile needs --reason"))?,
            evidence: serde_json::from_str(
                &flag("evidence")
                    .ok_or_else(|| format!("{USAGE_PREFIX}task reconcile needs --evidence JSON"))?,
            )
            .map_err(|e| format!("{USAGE_PREFIX}{e}"))?,
        },
        ["task", "import", manifest] => Command::TaskImport {
            manifest: manifest.into(),
            audit: flag("audit")
                .ok_or_else(|| format!("{USAGE_PREFIX}task import needs --audit"))?,
            source_dir: flag("source-dir")
                .ok_or_else(|| format!("{USAGE_PREFIX}task import needs --source-dir"))?,
            expected_root: flag("expected-root")
                .ok_or_else(|| format!("{USAGE_PREFIX}task import needs --expected-root"))?,
            owner_seat: flag("owner-seat")
                .ok_or_else(|| format!("{USAGE_PREFIX}task import needs --owner-seat"))?
                .parse()
                .map_err(|_| format!("{USAGE_PREFIX}invalid owner seat"))?,
        },
        ["task", "create", title] => Command::TaskCreate {
            title: title.into(),
            verify: flag("verify")
                .ok_or_else(|| format!("{USAGE_PREFIX}task create needs --verify CMD"))?,
            brief: flag("brief"),
        },
        _ => return Ok(None),
    }))
}

/// The flat (pre-grammar) form of this noun's commands; the others are noun-verb only.
pub(super) fn run(client: &mut Client, command: Command) -> Result<Vec<Value>, String> {
    let to_value = |v: &dyn erased::Serialize| v.to_json();
    Ok(match command {
        Command::Delegate { task, seat } => {
            let seats = client.seats()?;
            let seat = resolve(&seats, &seat)?;
            let response = client.task(TaskRequest::Delegate {
                task_path: task.clone(),
                seat_id: seat.record.id,
                role: None,
            })?;
            vec![
                json!({ "kind": "delegated", "task": task, "seat": seat.record.id, "response": to_value(&response) }),
            ]
        }
        Command::TaskCreate {
            title,
            verify,
            brief,
        } => {
            let mut row = client.record_write(task_create(&title, &verify, brief.as_deref()))?;
            row["kind"] = json!("task");
            vec![row]
        }
        Command::Reports { since } => {
            let mut cards = vec![];
            let mut after = None;
            loop {
                let TaskResponse::Items { items, next } = client.task(TaskRequest::List {
                    unassigned: false,
                    swarm_id: None,
                    status: None,
                    assignee: None,
                    after,
                    limit: Some(PAGE),
                })?
                else {
                    return Err("reports: unexpected task response".into());
                };
                cards.extend(items);
                match next {
                    Some(next) if Some(next) != after => after = Some(next),
                    _ => break,
                }
            }
            let mut out = vec![];
            for card in cards {
                if since
                    .as_deref()
                    .is_some_and(|since| card.updated.as_deref().is_none_or(|at| at < since))
                {
                    continue;
                }
                let TaskResponse::Item { item } = client.task(TaskRequest::Status {
                    task_path: Some(card.path.clone()),
                    task_id: None,
                })?
                else {
                    return Err("reports: unexpected task response".into());
                };
                let report = &item.metadata["report"];
                if report.is_null() {
                    continue;
                }
                out.push(json!({
                    "kind": "report",
                    "task": card.id,
                    "path": card.path,
                    "title": card.title,
                    "status": card.status,
                    "assignee": card.assignee,
                    "updated": card.updated,
                    "summary": report["summary"],
                    "commit": report["commit"],
                    "reported_by": report["reported_by"],
                    "verify_result": item.metadata["verify_result"],
                }));
            }
            out
        }

        _ => {
            return Err(no_flat_form());
        }
    })
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
        Command::TaskList {
            unassigned,
            seat,
            status,
        } => {
            let assignee = match seat {
                Some(seat) => Some(resolve(&client.seats()?, &seat)?.record.id.to_string()),
                None => None,
            };
            let mut cards = vec![];
            let mut after = None;
            loop {
                let TaskResponse::Items { items, next } = client.task(TaskRequest::List {
                    unassigned,
                    swarm_id: None,
                    status: status.clone(),
                    assignee: assignee.clone(),
                    after,
                    limit: Some(PAGE),
                })?
                else {
                    return Err("task list: unexpected task response".into());
                };
                cards.extend(items);
                match next {
                    Some(next) if Some(next) != after => after = Some(next),
                    _ => break,
                }
            }
            serde_json::to_value(cards).map_err(|e| e.to_string())?
        }
        Command::TaskGet { task } => {
            let request = match task.parse::<Ulid>() {
                Ok(id) => TaskRequest::Status {
                    task_path: None,
                    task_id: Some(id),
                },
                Err(_) => TaskRequest::Status {
                    task_path: Some(task),
                    task_id: None,
                },
            };
            serde_json::to_value(client.task(request)?).map_err(|e| e.to_string())?
        }
        Command::TaskReport {
            task,
            summary,
            commit,
            verify_result,
        } => serde_json::to_value(client.task(TaskRequest::Report {
            task_path: task,
            summary,
            commit,
            verify_result,
        })?)
        .map_err(|e| e.to_string())?,
        Command::TaskAccept { task } => {
            serde_json::to_value(client.task(TaskRequest::Accept { task_path: task })?)
                .map_err(|e| e.to_string())?
        }
        Command::TaskReject { task, reason } => {
            serde_json::to_value(client.task(TaskRequest::Reject {
                task_path: task,
                reason,
            })?)
            .map_err(|e| e.to_string())?
        }
        Command::TaskReconcile {
            task,
            disposition,
            reason,
            evidence,
        } => serde_json::to_value(client.task(TaskRequest::Reconcile {
            task_path: task,
            disposition,
            reason,
            evidence,
        })?)
        .map_err(|e| e.to_string())?,
        Command::TaskImport {
            manifest,
            audit,
            source_dir,
            expected_root,
            owner_seat,
        } => {
            let bundle_sha256 = bbt_mux::index::migration::bundle_digest(
                Path::new(&manifest),
                Path::new(&audit),
                Path::new(&source_dir),
            )
            .map_err(|e| e.to_string())?;
            serde_json::to_value(client.task(TaskRequest::Import {
                expected_root,
                manifest_path: manifest,
                audit_path: audit,
                source_dir,
                bundle_sha256,
                owner_seat,
            })?)
            .map_err(|e| e.to_string())?
        }
        Command::TaskImportContext {} => {
            serde_json::to_value(client.task(TaskRequest::ImportContext {})?)
                .map_err(|e| e.to_string())?
        }
        command => super::flat_reply(&mut client, command, verb)?,
    };
    super::print_reply(out, noun, verb, id, data, human)
}

/// One parse sample per verb of this noun's table (the verb gate in ctl.rs's tests).
#[cfg(test)]
pub(super) const PARSE_SAMPLES: &[&str] = &[
    "task list",
    "task get #1",
    "task status #1",
    "task report #1 --summary proof",
    "task accept #1",
    "task reject #1 --reason incomplete",
    "task reconcile #1 --disposition superseded --reason obsolete --evidence [\"record:1\"]",
    "task import m --audit a --source-dir s --expected-root /tmp/k --owner-seat 01M630JKTP20TAX8FN05X4GMEX",
    "task import-context",
    "task delegate #1 s",
    "task create t --verify true",
];
