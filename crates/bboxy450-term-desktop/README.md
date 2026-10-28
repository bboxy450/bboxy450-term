# bboxy450-term-desktop

The terminal's OS-agnostic desktop integration facade. Applications depend on
this crate; native bindings are isolated in per-OS adapters under decision 46.

- `bboxy450-term-desktop-core`: platform-free actions, menus, errors and stop port.
- `bboxy450-term-desktop-macos`: AppKit menu bar, status item and window behavior.
- `bboxy450-term-desktop`: reexports the contracts and selects the native backend.

The public app API remains `MenuBar`, `Tray`, `run_daemon`, window commands and
system double-click preferences. Linux can add an adapter behind this facade;
its current unsupported result is explicit. Neither facade nor core depends on
raw OS bindings, including through development dependencies.

The UI process owns its menu bar. A desktop daemon beside the PTY supervisor owns
the status item, so closing the UI keeps the icon and sessions alive. Callbacks
enqueue owned actions; they never directly call NSApplication::terminate.

Window helpers use the host's raw-window handle. Title-bar double clicks honor
Fill, Zoom, Minimize or None; Fill restores the previous frame on its next
invocation. Native probes live in the macOS adapter and inject no desktop input.

Project/source: https://github.com/bboxy450/bboxy450-term
License: Respectful Slop License 1.0-Draft; see LICENSE. Applicable upstream terms
and prior grants remain intact. Attribution policy:
https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
