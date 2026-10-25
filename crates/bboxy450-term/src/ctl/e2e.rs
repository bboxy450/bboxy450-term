//! The e2e promise records: `ctl e2e …`.

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[("e2e", "list", "Knowledge")];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! e2e_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "e2e": [
            E2e {
                group: Option<String>,
            },
        ]] $($head)*);
    };
}
pub(super) use e2e_commands;
