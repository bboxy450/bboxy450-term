# bboxy450-term-desktop-core

Platform-free desktop contracts for the terminal: owned `Action` values, menu
models, typed errors, title-bar action choices and the daemon stop port. It has
no native binding or backend dependency. The native adapter and the public
facade both depend on this crate, so they cannot form a dependency cycle.

Backends construct `StopHandle::new(wake, error)`. A wake function only enqueues
a nonblocking native-loop event; `stop()` invokes it at most once across clones.
`run_without_native` still runs and joins the supplied worker, exposing the
native failure through its stop handle and final result.

Project/source: https://github.com/bboxy450/bboxy450-term
License: Respectful Slop License 1.0-Draft; see LICENSE. Applicable upstream terms
and prior grants remain intact. Attribution policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
