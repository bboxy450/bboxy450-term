//! Window chrome through the GUI owner: `ctl ui …`.

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
