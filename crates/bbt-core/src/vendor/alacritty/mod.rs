//! Forked alacritty_terminal model; see VENDOR.md for provenance.
// modified by bboxy450.
pub mod event;
pub mod grid;
pub mod index;
pub mod sync;
// Vendored upstream: style lints are not fixed in the imported source.
#[allow(
    clippy::let_and_return,
    clippy::collapsible_if,
    clippy::manual_is_multiple_of
)]
pub mod term;
