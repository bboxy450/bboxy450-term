//! Forked alacritty_terminal model; see VENDOR.md for provenance.
// modified by bboxy450.
/// Terminal-generated events and embedding listener interfaces.
pub mod event;
pub mod grid;
pub mod index;
/// Synchronized-output timing interfaces used by the parser and terminal model.
pub mod sync;
// Vendored upstream: style lints are not fixed in the imported source.
#[allow(
    clippy::let_and_return,
    clippy::collapsible_if,
    clippy::manual_is_multiple_of
)]
pub mod term;
