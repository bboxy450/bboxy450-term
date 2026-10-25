//! Task records and their lifecycle: `ctl task …`.

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
