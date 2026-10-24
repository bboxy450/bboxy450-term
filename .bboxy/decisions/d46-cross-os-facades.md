---
id: "01M5NG5SZR9E300YP57X83C3A9"
type: "decision"
number: 46
title: "Everything cross-OS is a facade over per-OS crates"
status: "decided"
decided: "2026-10-23"
operator:
  - "as a GENERAL RULE, everything cross-OS is done this way, with an eye on eventually running in wasm, on iPad and on Android"
  - "we are a pure GPU rendering library"
  - "unless an existing crate already does this well enough to use"
links:
  - { to: "[[2026-10-22-architecture-round-1#46]]", rel: "supersedes-as-index" }
  - { to: "[[cross-os-facade]]", rel: "constrains" }
created: "2026-10-23T23:05:15Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 46: Everything cross-OS is a facade over per-OS crates

**Everything cross-OS is a facade over per-OS crates (operator, same
message: "as a GENERAL RULE, everything cross-OS is done this way, with
an eye on eventually running in wasm, on iPad and on Android", "we are a
pure GPU rendering library", "unless an existing crate already does
this well enough to use").** Raw OS bindings live only in `*-macos`,
`*-windows`, `*-linux`, `*-ios`, `*-android`, `*-web` crates behind an
OS-agnostic facade the suite depends on; OS-agnostic crates that are
facades themselves (`winit`, `wgpu`) are allowed anywhere; POSIX is not
an OS. Invariant [[cross-os-facade]] with `sh scripts/cross-os.sh` in
the default set; the bindings that predate the rule are dated
exceptions in the script (`bboxy450-term` window and Metal glue,
`bbt-render` CoreText raster, `bbt-mux` Keychain custody), each retired
when its facade arrives, never silently extended. Rejected: refactoring
the three exceptions now (velocity; nothing else runs on a second OS
yet); a rule without a check (a preference).
