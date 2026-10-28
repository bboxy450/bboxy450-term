# bboxy450-desktop-notifications-macos

The UserNotifications backend of `bboxy450-desktop-notifications`. The facade
owns the public application contract; this crate accepts owned string ids and
retains the process delegate before the application event loop starts.

The bundle check happens before the first notification-center call. There is
no bundle-id impersonation or desktop scripting fallback. Native completion
waits have a two-second bound and belong on an IO worker. An authorization
prompt may outlive that bound; a timeout does not mean the operator denied it.

References used for the mechanism, with no copied implementation:

- [Apple's delegate lifecycle](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter/delegate)
- [Request identifiers](https://developer.apple.com/documentation/usernotifications/unnotificationrequest/identifier)
- [Removing delivered notifications](https://developer.apple.com/documentation/usernotifications/unusernotificationcenter/removedeliverednotifications(withidentifiers:))
- `objc2-user-notifications` 0.3.2 generated API declarations

Ad-hoc bundle delivery, replacement of an already-delivered notification and
cold-launch clicks remain empirical acceptance checks, not assumptions based
on those declarations. The app's logged bundle probe supplies that evidence.

## Respectful slopping

Project-owned contributions use [Respectful Slop License 1.0-Draft](LICENSE).
Retained upstream material keeps its original terms; earlier grants are not
withdrawn. [ATTRIBUTION.md](ATTRIBUTION.md) identifies the scope, source and reuse
examples; [NOTICE](NOTICE) preserves factual upstream notices.
Slop respectfully. Keep the credit attached.
