//! Seats: listing, input, screens, capture and lifecycle: `ctl seat …`.

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("seat", "ls", "ListSeats"),
    ("seat", "spawn", "Spawn"),
    ("seat", "screen", "Screen"),
    ("seat", "type", "Input"),
    ("seat", "key", "Input"),
    ("seat", "paste", "Input"),
    ("seat", "wait", "Screen"),
    ("seat", "resize", "Resize"),
    ("seat", "watch", "EventsRead"),
    ("seat", "rename", "RenameSeat"),
    ("seat", "focus", "FocusSeat"),
    ("seat", "restart", "RestartSeat"),
    ("seat", "close", "CloseSeat"),
    ("seat", "undo-close", "UndoCloseSeat"),
    ("seat", "capture", "Capture"),
];
