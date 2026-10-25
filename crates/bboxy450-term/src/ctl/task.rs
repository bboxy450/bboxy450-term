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
