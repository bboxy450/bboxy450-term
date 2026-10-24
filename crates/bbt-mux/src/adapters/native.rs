//! Native protocol implementation belongs to the reusable kit.
use super::{Binding, Endpoint, Priority, Ulid, WakeOutcome};
pub use bboxy450_harness_wake_up::native::*;
/// Translate native transport evidence at the app boundary.
pub async fn wake(
    binding: &Binding,
    endpoint: &Endpoint,
    id: Ulid,
    text: &str,
    priority: Priority,
    allow_interrupt: bool,
) -> WakeOutcome {
    super::mapping::outcome(
        bboxy450_harness_wake_up::native::wake(
            binding,
            endpoint,
            id,
            text,
            priority,
            allow_interrupt,
        )
        .await,
    )
}
