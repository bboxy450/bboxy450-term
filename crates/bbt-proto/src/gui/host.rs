//! Hosts the window can place seats on.

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! host_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            ListHosts,
        ] $($head)*);
    };
}
pub(super) use host_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! host_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Hosts(Vec<crate::GuiHost>),
            HostState {
                host: crate::GuiHost,
            },
        ] $($head)*);
    };
}
pub(super) use host_replies;
