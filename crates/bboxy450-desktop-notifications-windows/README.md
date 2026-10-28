# bboxy450-desktop-notifications-windows

DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications-research.md

This crate returns `Unsupported` and has no dependencies. Research retrieved
2026-10-23, before this placeholder was written:

- Classic Win32 cold activation requires a Start shortcut carrying
  `System.AppUserModel.ID` and `System.AppUserModel.ToastActivatorCLSID`,
  and a registered `LocalServer32` COM server implementing
  `INotificationActivationCallback`. An in-process click callback alone
  does not cover a stopped app. [Microsoft activation contract](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/win32_tile_badge_notif/respond-to-toast-activations).
- The Windows App SDK offers registration that creates the COM activation
  plumbing for an unpackaged app, with its runtime as an extra requirement.
  [Microsoft console-app path](https://learn.microsoft.com/en-us/windows/apps/develop/notifications/app-notifications/app-notifications-console).
- A real backend must map stable ids to `Tag`/`Group`, and remove with
  `ToastNotificationHistory.Remove`; it must prove both warm and cold clicks.
  [Tag](https://learn.microsoft.com/en-us/uwp/api/windows.ui.notifications.toastnotification.tag),
  [history removal](https://learn.microsoft.com/en-us/uwp/api/windows.ui.notifications.toastnotificationhistory.remove).

No app identity, shortcut, COM registration or notification is created here.

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
