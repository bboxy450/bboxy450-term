---
id: 01M5NG4WXQ0B9JZS4FEP5JE5VA
type: spec
title: "desktop notifications — an OS-agnostic facade over one crate per OS; macOS real, the rest dummies that say so"
status: decided
links:
  - { to: "[[2026-10-22-architecture-round-1]]", rel: decisions }
  - { to: "[[attention]]", rel: consumer }
  - { to: "[[cross-os-facade]]", rel: invariant }
  - { to: "[[layering]]", rel: invariant }
  - { to: "[[licensing]]", rel: licence }
  - { to: "[[desktop-notifications-research]]", rel: research }
created: 2026-10-23T23:04:45Z
updated: 2026-10-23T23:04:45Z
---
# Desktop notifications

Owner: Corvin Duskmantle <ARCHITECT>. Decided 2026-10-23 (decisions #45 and #46). Operator, via the Oracle (verbatim gist): desktop notifications get their own crate family, an OS-agnostic facade `bboxy450-desktop-notifications` with per-OS sub-crates; macOS first and real; Windows and Linux are dummy crates for now with an explicit "DO RESEARCH BEFORE IMPLEMENTING" marker; as a general rule everything cross-OS is done this way, with an eye on wasm, iPad and Android ("we are a pure GPU rendering library"), unless an existing crate already does it well enough to use.

Lens: the app is a suite of kits (decision #39); the OS is a dependency to be fenced, not a home. Assumes `UNUserNotificationCenter` needs a bundle identifier and a delegate set at launch. Breaks when a target has no per-OS crate: then the facade refuses to compile, by design.

```mermaid
flowchart LR
  APP["app<br/>attention, decision #44"] -- "post, replace by id, remove" --> F["bboxy450-desktop-notifications<br/>facade: Notifier, Notification, Response"]
  F -- "cfg target_os = macos" --> M["bboxy450-desktop-notifications-macos<br/>UNUserNotificationCenter and its delegate"]
  F -- "cfg target_os = windows" --> W["bboxy450-desktop-notifications-windows<br/>dummy: Unsupported, research first"]
  F -- "cfg target_os = linux" --> L["bboxy450-desktop-notifications-linux<br/>dummy: Unsupported, research first"]
  M -- "didReceive response" --> F
  F -- "Response::Clicked(id)" --> APP
```

## 1. Shape

- **One facade, one crate per OS.** `bboxy450-desktop-notifications` holds the types and the trait and selects the backend with `[target.'cfg(target_os = "…")'.dependencies]`; `bboxy450-desktop-notifications-macos` is real; `-windows` and `-linux` are dummies; `-ios`, `-android` and `-web` are named here and not created until there is work for them. The suite depends only on the facade; the app never names an OS crate ([[cross-os-facade]]).
- **Layering.** The four crates lead the order in `scripts/layering.sh`, so they can depend on nothing of ours and nothing of ours but the app depends on them; the click never carries a `bbt-*` type, only the facade's `Response`, which the app maps to its own action ([[layering]]).
- **Names and licence.** The names are the operator's; crates.io availability is checked before any publish (the research records what the pages return today). Dual MIT OR Apache-2.0 like every kit ([[licensing]]).

## 2. The facade API

```rust
pub struct NotificationId(pub String);          // stable per episode: attention uses the seat id
pub struct Notification { pub id: NotificationId, pub title: String, pub body: String }
pub enum Response { Clicked(NotificationId), Dismissed(NotificationId) }
pub enum Authorization { Granted, Denied, Undetermined }
pub enum Error {
    Unsupported { os: &'static str, note: &'static str },  // a dummy answered
    NoBundle,                                                // macOS: no bundle identifier, see §3
    Denied,                                                  // the user refused authorization
    Backend(String),                                         // the OS said no, verbatim
}
pub trait Notifier: Send + Sync {
    fn authorize(&self) -> Result<Authorization, Error>;
    fn post(&self, n: &Notification) -> Result<(), Error>;   // the same id replaces, never stacks
    fn remove(&self, id: &NotificationId) -> Result<(), Error>;
}
pub fn open(on_response: Box<dyn Fn(Response) + Send + Sync + 'static>) -> Result<Box<dyn Notifier>, Error>;
```

`open` is called once at launch, before any window shows, because the OS delegate must exist before a notification can be delivered to the process. `on_response` runs on the backend's thread; the caller forwards into its own event loop (the app uses its winit proxy) and the facade knows nothing about windows. No images, no actions beyond the default click, no sounds in v1: attention needs none of them.

## 3. The macOS crate

- `UNUserNotificationCenter.currentNotificationCenter()` with the process as its own `UNUserNotificationCenterDelegate`; `requestAuthorization(options: [.alert, .sound])` on `authorize`; `UNMutableNotificationContent` + `UNNotificationRequest(identifier: id.0)` on `post`, so the identifier carries the replace semantics; `removeDeliveredNotifications(withIdentifiers:)` and `removePendingNotificationRequests(withIdentifiers:)` on `remove`.
- `didReceive response`: `UNNotificationDefaultActionIdentifier` → `Response::Clicked(id)`, `UNNotificationDismissActionIdentifier` → `Dismissed(id)`, the id read from `response.notification.request.identifier`; the completion handler is called after the callback returns.
- **Bundle check first.** `open` reads `NSBundle.mainBundle.bundleIdentifier`; `None` returns `Error::NoBundle` **before** touching the notification centre, which throws without one. The build produces `bboxy450-term.app` with an `Info.plist` for every run, ad-hoc signed; Developer ID stays deferred (decision #24; ad-hoc is not identity signing).
- **Use versus build** for the bindings underneath is decision #45, taken from [[desktop-notifications-research]]: the facade API and the per-OS layout above do not move with that choice; only what the macOS crate wraps does.

## 4. The dummies

- `bboxy450-desktop-notifications-windows` and `-linux` compile on their OS with no dependencies and answer `open` with `Err(Error::Unsupported { os, note: "DO RESEARCH BEFORE IMPLEMENTING: .bboxy/research/desktop-notifications.md" })`; their `README.md` and crate docs carry the same marker in capitals, with the research record's pointers for toasts (AUMID, COM activator) and freedesktop (`org.freedesktop.Notifications`, `ActionInvoked`).
- A target with no per-OS crate hits `compile_error!` in the facade: an unknown OS fails at build time, a known-but-unbuilt OS fails at run time by name. The app treats `Unsupported` and `NoBundle` the same way: the in-window attention loop runs, nothing is posted, one log line says why.

## 5. Acceptance, the spec signs off

1. `cargo build -p bboxy450-term` on macOS links the facade and the macOS crate only; `cargo metadata` shows no `bbt-*` dependency in any of the four crates; [[layering]] and [[cross-os-facade]] hold.
2. A test double of the delegate delivers a default-action response and the facade's callback receives `Clicked(id)` with the posted id; posting twice with one id yields one delivered notification (identifier replace); `remove` clears it.
3. Run unbundled, `open` returns `NoBundle` without touching the notification centre (no exception, the log line present); run as the bundle, `authorize` prompts once and the state persists across launches.
4. On a Linux or Windows build of the facade (CI cross-check or a local VM), `open` returns `Unsupported` with the marker text.

Owner: Corvin Duskmantle <ARCHITECT>.
