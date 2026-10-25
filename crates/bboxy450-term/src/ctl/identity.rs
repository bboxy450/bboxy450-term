//! The operator identity hold: `ctl identity …`.

/// This noun's rows of [`super::VERBS`], in parse order.
pub(super) const VERBS: &[super::Verb] = &[
    ("identity", "status", "Identity"),
    ("identity", "unlock", "Identity"),
];

/// This noun's `Command` variants, spliced into the flat enum by `commands` in `ctl.rs`.
macro_rules! identity_commands {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)* "identity": [
            Identity(bbt_proto::identity::IdentityRequest),
        ]] $($head)*);
    };
}
pub(super) use identity_commands;
