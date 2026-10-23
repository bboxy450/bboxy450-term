---
id: 01M5NG4WXQE3YJH0M8KH4CJP8W
type: invariant
title: raw OS bindings live only in per-OS crates behind a facade
status: active
check: sh scripts/cross-os.sh
cost: an OS binding in a shared crate makes that crate unportable and hides the OS seam from the next platform; the suite's promise is one GPU kit on every OS
links:
  - { to: "[[desktop-notifications]]", rel: constrains }
  - { to: "[[architecture]]", rel: constrains }
  - { to: "[[layering]]", rel: sibling }
created: 2026-10-23T23:04:45Z
updated: 2026-10-23T23:04:45Z
---
# Raw OS bindings live only in per-OS crates behind a facade

**Property.** No workspace crate outside the per-OS family (`*-macos`, `*-windows`,
`*-linux`, `*-ios`, `*-android`, `*-web`) depends on a raw OS binding crate
(`objc2*`, `block2`, `core-foundation*`, `core-graphics`, `core-text`, `cocoa`,
`security-framework`, `windows*`, `winrt*`, `zbus`, `dbus`, `web-sys`,
`wasm-bindgen`, `jni`, `ndk*`), except the dated exceptions listed in the script
(`bboxy450-term`'s window and Metal glue, `bbt-render`'s CoreText raster,
`bbt-mux`'s Keychain custody), each of which leaves when its facade arrives
(decision #46). OS-agnostic crates that are facades themselves (`winit`, `wgpu`)
are not bindings; POSIX (`libc`, `mio`, `nix`) is not an OS.

**Check.** `sh scripts/cross-os.sh` — `cargo metadata` over the workspace, exit 0
held, non-zero names each `crate -> dep` that breaks the rule. Fast; in the
default set. A new exception is a decision, written into the script with its
date and the facade that retires it, never a silent edit.

**Cost of a violation.** The shared crate stops being portable, the OS seam is
hidden from the next platform's Executor, and the suite's promise ([[vision]],
decision #39: one GPU kit, every app) breaks where nobody is looking.

**Evidence.** `sh scripts/cross-os.sh` — `cross-os: held` at the commit that
adds it, with the three dated exceptions.

Owner: Corvin Duskmantle <ARCHITECT>.
