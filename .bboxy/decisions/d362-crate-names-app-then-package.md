---
id: 01M5XPB3CKB0FTSBREHYVPMTBY
type: decision
number: 362
title: "crate names are bboxy450-<app>-<package>; generic crates drop the app"
status: decided
decided: 2026-10-26
operator:
  - "and in our rules document a decision that naming convention is bboxy450-appname-packagename, so bboxy450-term-desktop if something is specific desktop integration for term only, not a generic crate, term is just one of many apps we will build"
  - "bboxy-firehose = \"0.1.0\" - that crate should be bboxy450-firehose!"
links:
  - { to: "[[d52-brand-and-face]]", rel: refines }
  - { to: "[[bboxy450-prefix]]", rel: constrains }
  - { to: "[[licensing]]", rel: constrains }
created: 2026-10-27T03:26:55Z
updated: 2026-10-27T03:26:55Z
---
# Decision 362: crate names are bboxy450-<app>-<package>

Decided 2026-10-26.

**Rule.** A crate that serves one app is named `bboxy450-<app>-<package>`; a crate any app can use is `bboxy450-<package>`. bboxy450-term is one of many apps, so terminal-only pieces carry `term`: desktop integration for the terminal alone is `bboxy450-term-desktop`, while a cross-app kit such as `bboxy450-keychain` stays generic. The binary a crate installs takes the crate's name. Rust library names may stay as they are (`bbt_*`, `bboxy_firehose`) so code does not churn.

**Applied now.** `bboxy-firehose` was published as 0.1.0 under the internal-tool exemption of [[bboxy450-prefix]], unbranded. It is renamed `bboxy450-term-firehose` (it generates terminal output to test this terminal) in the next release, and `bboxy-firehose` 0.1.0 is yanked once its dependents ship on the new name. [[bboxy450-prefix]] now refuses an internal crate unless it sets `publish = false`.

**Next release.** The other 23 published crates are checked against this rule before it ships; any app-specific crate is renamed then, in one release, not piecemeal.

Rejected: a flat `bboxy450-<package>` for everything (an app-specific crate would take a generic name another app needs); enforcing the app segment in `scripts/brand.sh` (whether a crate is app-specific is a judgement, recorded per crate in [[licensing]], not a string test).

Owner: Vesper Greywind <ORACLE>.
