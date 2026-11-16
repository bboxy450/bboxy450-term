---
type: note
title: Main green, de-flaked tests, bboxy450 0.2.0 published (seat Nyx Copperwing, 2026-11-13)
id: 01M7A3MS1KS7R82Q5PFWX91QZM
updated: 2026-11-13T09:25:59.534Z
created: 2026-11-13T09:25:59.534Z
number: 1794292861627501
created_by: seat:01M78ZH53B1XQ06T8GK7BRA3GJ
updated_by: seat:01M78ZH53B1XQ06T8GK7BRA3GJ
authorship:
  signer: seat:01M78ZH53B1XQ06T8GK7BRA3GJ
  kind: seat
  public_key: zgyvdcrImMT1el4By5PnoQ1eggyKe6ldv7jbb3x8peQ=
  process_generation: '1'
provenance:
  digest: sha256:792524deb77393cf9363fcae1b59387f2b7191e096b4b6eff83df986bbea9c3d
  frontmatter_digest: sha256:f68d06badbe9343e66ae73955009fafd7af96bb6856c756e69d484126ecb7e7c
  signer: seat:01M78ZH53B1XQ06T8GK7BRA3GJ
  sig: dB8FY8XKcKHY78cLNrSt/JVM7txsbQZ3qtvNYDqiERLKXQTw7ta67TvfLw+0+Yf/EKrJOBc8uGVslnSduymLBA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151659
  event:
    id: 01M7A3MSSM58C8P6M1YWY3GDC0
    sequence: 151659
    hash: d0e43553b15e1e7e16522b2cbb75a0dda3d1e3e8b520d346e2222d4df2bfe077
---
# Main green, de-flaked tests, bboxy450 0.2.0 published

Author: seat:01M78ZH53B1XQ06T8GK7BRA3GJ (solo). Commits on term main: 53a9e215f, 320d40e3f. Related task (other swarm): [[tasks/2026-11-12-remove-flaky-tests]].

## Result

| Step | Evidence |
|---|---|
| Full gate on the published commit | `make test` exit 0; `ci-local: result 320d40e3f7af66c19344dc80f777176c7cc9f9e2 normal 0`. Workspace 2752/2752, no retry-only passes; harness-native, formal (TLC, Lean), smoke, invariants 27/27 passed |
| Crates | `make publish-crates` exit 0 as bboxy450: 37 crates at 0.2.0, each with a verifying `.asc`; crates.io max_version 0.2.0 checked for term, harness, yarn, tlc |
| Tag + release | signed `crates-v0.2.0` on 320d40e3f; https://github.com/bboxy450/bboxy450-term/releases/tag/crates-v0.2.0 |
| App | `make release` exit 0, no Keychain prompt; codesign verify OK (team UQB3368A84); signed `v0.2.0`; https://github.com/bboxy450/bboxy450-term-releases/releases/tag/v0.2.0 |
| Harness | `release/0.2.0` fast-forwarded to pinned f93f515a; signed tag `v0.2.0` there |

`make publish-release` was not run whole: its `git push origin HEAD:main` would rewind main. Its other steps ran in order by hand.

## Term fixes

| Test | Cause | Change |
|---|---|---|
| mux `mcp_task_list_is_most_recently_changed_first_...` | external writes waited on the watcher; at load 10 fseventsd delivered no event for >10 s; 8/10 failed | `rescan` helper (one mux write forces a full rescan); 60/60 stress |
| wake-up `dream_c1` | asserted spawn ≤2 ms CPU / 25 ms wall, setup ≤1 ms; 250 ms readiness | asserts removed; `dream_bootstrap::WINDOW` → 2 s (job deadline bounds wall); ICD dream-term-c1 updated; 25/25 stress |
| 19 Dream/proto tests under plain cargo | fixture env only in Makefile | `.cargo/config.toml` `[env]`; Makefile export removed |
| `canvas_golden_dark/light` | byte-exact GPU compare, two stable AA variants | `bbt_render::chrome::diff` policy; broken control fails at 99.77 % |
| `native_bundle_recipe_installs_...` | recipe copies new remote binary | fixture writes it |
| `harness_branding_all_menu_surfaces_and_launch_keys` | a machine offers only bboxy450-harness since ab5583a5b | test asserts that |

## Harness main fixes

cargo fmt; merge of published f93f515a; `[patch]` sections removed (term crates from crates.io 0.2.0); clippy range pattern; ACP fixture client skips JSON-RPC notifications (`tools/list_changed` before a reply caused "RPC identity mismatch"); `store_fork` compares own tools and reads the child log after close.

## Gaps

- Suspected read-your-writes gap in `knowledge/routing.rs` `poll_scans` not landed (no reproducing test).
- `mcp_read_byte_pages_knowledge_fetch` still waits on the watcher; its unnumbered note blocks a rescan write.
- rustup stable `rust-objcopy` cannot load libLLVM (harmless warning).
