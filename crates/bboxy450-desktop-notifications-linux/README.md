# bboxy450-desktop-notifications-linux

DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md

This crate returns `Unsupported` and has no dependencies. Research retrieved
2026-10-23, before this placeholder was written:

- Use the session-bus `org.freedesktop.Notifications` interface (spec 1.3),
  with `zbus` as a future transport. Preserve the server's returned id and
  pass it as `replaces_id` for an update; `CloseNotification` removes it.
- `ActionInvoked` with action key `default` reports a click only when the
  server supports actions. `ActivationToken` may arrive before it and is
  needed for a Wayland toplevel activation. Neither signal is guaranteed.
- Query capabilities and handle a missing notification server. Prove both
  replacement and focus in an actual desktop session before shipping.

[Protocol and signals](https://specifications.freedesktop.org/notification/latest/protocol.html),
[basic design and default action](https://specifications.freedesktop.org/notification/latest/basic-design.html).

No D-Bus call or shell fallback is made here.
