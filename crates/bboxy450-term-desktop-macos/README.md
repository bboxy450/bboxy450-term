# bboxy450-term-desktop-macos

The terminal's AppKit desktop adapter, behind `bboxy450-term-desktop` and built
on `bboxy450-term-desktop-core`. Raw Objective-C, dispatch and AppKit dependencies
live here under decision 46; the facade and core contain no OS bindings.

`MenuBar` belongs to the UI process; `Tray` and `run_daemon` belong to the desktop
companion that follows the PTY supervisor. Native handles remain on the main
thread, callbacks enqueue owned core actions, and no action directly terminates
NSApplication. The backend stop port wakes the AppKit loop without polling.

Window helpers honor system double-click preferences and preserve Fill/restore
geometry on each window. The `native_probe` example uses owned windows and native
menu activation to verify behavior without desktop input or operator identity.

Project/source: https://github.com/bboxy450/bboxy450-term
License: Respectful Slop License 1.0-Draft; see LICENSE. Applicable upstream terms
and prior grants remain intact. Attribution policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
