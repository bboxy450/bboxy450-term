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
