# bboxy450-cef

Chromium for bboxy450, through the Chromium Embedded Framework
([CEF](https://github.com/chromiumembedded/cef), via
[tauri-apps/cef-rs](https://github.com/tauri-apps/cef-rs)).

- `defaults`: the one place our Chromium settings live. Windowless (offscreen)
  rendering, a private cache root, no remote-debugging port, the process's own
  arguments ignored, and the switches that keep Chromium from touching the
  login Keychain or phoning home.
- `cdp`: a Chrome DevTools Protocol client with no I/O: it numbers requests,
  encodes them, and matches every reply to exactly one request. It builds and
  tests without Chromium.
- `engine` (feature `engine`): applies the defaults to CEF and passes CDP bytes
  to and from a browser. Only this feature brings Chromium into a build.

Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
