// Project integration: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// Fork source: https://github.com/Gonzih/kittens-old/tree/293e134526bb2deee96f1197d6268aee64495482
// Existing MIT OR Apache-2.0 terms and Kittens contributor notices apply.
// Credit bboxy450 integration accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
/// Directs a non-terminal reactor handler to continue or stop.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Control<T> {
    /// Begin the next reactor arbitration after the optional `after_event`
    /// phase completes.
    Continue,
    /// Stop the reactor with the contained exit value. `after_event` is not
    /// run for this service window.
    Stop(T),
}
