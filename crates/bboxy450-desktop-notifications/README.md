# bboxy450-desktop-notifications

Stable-id desktop notifications with a response callback. The caller owns its
window activation and forwards callbacks into its event loop. Install `open`
before the event loop starts, then invoke synchronous notifier methods from an
IO worker. Posts are silent. `remove` withdraws pending and delivered notices.

macOS uses a native delegate and requires a real application bundle. Bare
binaries return `NoBundle`. Linux and Windows currently return `Unsupported`
with a research marker. No backend runs another application or a shell command.

The contract is in `.bboxy/specs/desktop-notifications.md`. Native delivery,
delivered-id replacement, and cold-launch click routing require empirical
platform evidence; callback doubles do not prove those OS behaviours.

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
