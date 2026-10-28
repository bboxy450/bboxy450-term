# bboxy450-term-desktop

The terminal's portable desktop action and menu contract, with a native macOS
adapter. Linux can implement the same contract later; unsupported adapters return
an explicit error. App actions never call `NSApplication::terminate`.

`MenuBar` belongs to the UI process. `Tray` and `run_daemon` belong to a surviving
desktop daemon beside the PTY supervisor. Keep their native handles on the main thread. Menu callbacks enqueue
owned `Action` values; they must never perform blocking application work.
`run_daemon` runs the supplied worker even when the native tray is unavailable,
and `StopHandle::stop` wakes the main runloop without periodic polling.

Window helpers use the host's raw-window handle. Title-bar double clicks honor
the macOS Fill, Zoom, Minimize or None preference; Fill restores the previous
frame on its next invocation. No window geometry is written to user defaults.
`activate` invokes an installed native menu item directly for unattended probes;
it does not inject desktop input.

Project/source: https://github.com/bboxy450/bboxy450-term
Attribution: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
