//! Messages between seats: `ctl message …`.

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("message", "send", "Messaging"),
    ("message", "post", "Messaging"),
    ("message", "inbox", "Messaging"),
    ("message", "receipts", "Messaging"),
    ("message", "ack", "Messaging"),
];
