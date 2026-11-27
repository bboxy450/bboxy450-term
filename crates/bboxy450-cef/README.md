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

## Browser host on macOS

`make install` and `make release` bundle the CEF framework, its sandbox
library, Chromium notices and the five macOS helper app names. The helpers run
the same executable in CEF subprocess mode with the macOS sandbox initialized.
The app's visible terminal and its browser host run in different processes.
`BROWSER=0` omits Chromium. Use `BROWSER=1` to enable it in a debug bundle.

The logic owner starts one host per authenticated seat and process generation on
its first `browser_open`. It passes an absolute private profile directory to
`bboxy450-term browser-host PRIVATE_PROFILE_ROOT`. The host opens one windowless
1024×768 page. Stdin carries UTF-8 JSON CDP requests with numeric IDs, one per
line. Stdout carries a single `{"method":"BBoxy.browserReady"}` event when
registration succeeds, followed by CEF replies and events, one JSON message per
line. Diagnostics go to the profile log. The pipe is inherited by the owner
and Chromium listens on no DevTools port. EOF closes the browser and exits the
host.

| MCP tool | Arguments | Result |
| --- | --- | --- |
| `browser_open` | `url` with HTTP(S), `data:`, or `about:blank` | Final `url`, `title` and `http_status` after the main frame loads. Status is null when no HTTP status is available. |
| `browser_cdp` | `method`, optional object `params`, optional `timeout_ms` in 1–8000 | CDP `result` or `error`, with `events` and `events_dropped` accumulated since the previous CDP call. |
| `browser_screenshot` | Optional `full_page`, default false | Stored PNG artifact receipt with its `id`, revision and embeddable `reference`. Full-page capture fits within 8192 pixels per side and 4,000,000 pixels total. |
| `browser_close` | Empty object | `closed` reports whether there was a host to retire. |

One request is in flight per seat. An overlapping call gets `browser_busy`.
Closing interrupts active work, closes the pipe and reaps the owned process
group before releasing its profile. Seat close, seat exit and backend stop
retire the same owner. A host crash returns `browser_disconnected` and the
next open starts a fresh profile and pipe. Timeouts cancel that request ID;
late replies cannot complete another call. A startup or open has an
eight-second budget. CDP replies are capped at 24 MiB on the pipe and at the
normal MCP budget in tool results. The event buffer holds up to 128 events and
96 KiB, evicts oldest first, and counts each discarded event. Oversized MCP
results include a `cut` note. Open, CDP and close use transient correlation IDs.
Screenshot artifacts use the existing durable operation receipts. If the
artifact publisher is busy, `provider_busy` reports that no artifact operation
was admitted and the capture can be retried. Without browser support, the
advertised tools return `browser_unavailable` with the build flag.

`make test-browser` runs a throwaway signed bundle with two local seats and
offline data pages, exercises the tools through seat MCP, checks artifact
pixels and process/profile retirement, and tests a killed host and a late CDP
reply. Its owner model and broken reply-matching control are in
`verification/contracts.toml`.
