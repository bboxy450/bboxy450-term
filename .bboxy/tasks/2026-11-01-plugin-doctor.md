---
id: "01M6D54N9BRV55J8XA2G7MFMC4"
type: "task"
number: 5097
title: "plugin-doctor: offline smoke test that lists concrete plugin gaps"
status: "active"
role: "executor"
verify: "make plugin-doctor-test"
created: "2026-11-02T03:34:09Z"
updated: "2026-11-02T04:20:00Z"
---
# plugin-doctor

ROLE: EXECUTOR. Worktree `~/mydev/bboxy450-term-plugin-doctor`, branch `plugin-doctor`,
from `origin/main` 86ce2653. The Oracle lands it; this branch never pushes to `main`.

## Goal

One command, offline and with no running app, that runs a plugin package or binary
through the host contract and prints the author a list of gaps: the check id, the
observed evidence and how to fix it. `--json` for CI; exit 1 on any error-severity gap.
Plugin authors call the same checks from their own `cargo test` through a testkit helper.

## Shape (operator, 2026-11-01)

- A module `doctor` inside `crates/bbt-plugin-sdk` plus the example
  `bboxy450-term-plugin-doctor` in that crate (helper tools are examples, never
  installed binaries: LAND.md rule 3). No new crate, no subcommand of another binary.
- `make plugin-doctor PLUGIN=<binary-or-package> [JSON=1]`; `make check-gitkb-plugin`
  also runs it on the GitKB plugin.
- Docs: SDK README "Smoke-test your plugin", crate rustdoc with a runnable testkit
  example, a pointer in `.bboxy/interfaces/plugin-host.md`, and every SDK example's own
  test calls the doctor helper.

## Reuse, not a fork

The SDK sits below `bboxy450-term-mux` in `scripts/layering.sh` ORDER, so the doctor
cannot call `bbt_mux::plugins::check`. The pure parts of that checker move down into
`bboxy450-plugin` and both callers use them:

- `shape::parse_descriptor` (bundled schema, `Shape::validate`, method and schema
  compilation) was `check.rs` `parse_descriptor`;
- `contracts::Contract::decode_result` (typed DTO decode) was `check.rs`
  `typed_data_result`, extended from `data_source` to every contract.

The doctor drives the binary with `testkit::Client`, `testkit::shape` and
`testkit::allowed_env`. Shipped models are read back into the native checker by a new
`bboxy450_tlc::parse` of the exporter's own subset (`Model::tla`/`config`), round-trip
tested, then checked natively and, when Java TLC is available, with Java TLC.

## Checks

| Id | What it proves |
|---|---|
| `shape.name` | file name is `bboxy450-term-<contract>-<provider>`; a package's binary equals its package name and is `bboxy450-` branded |
| `shape.descriptor` | `--shape` within 2 s, valid against the host descriptor validator |
| `shape.digest` | binary SHA-256 (the trust digest) and `--shape` bytes are identical across runs |
| `shape.catalogue` | `tools/list` terminates and has every contract and declared tool with compiling schemas |
| `shape.skills` | each declared skill is listed and read back byte for byte |
| `contract.dto` | every exercised method, declared tool, hook and health answers a DTO valid against the contract schema, the declared schema and the typed SDK contract |
| `call.refusal` | every failure is a typed tool refusal; no panic, JSON-RPC error, crash or missing reply |
| `call.cold_start` | `initialize` and the first call fit the host's 2 s budget |
| `call.deadline` | every call fits min(declared `timeout_ms`, host ceiling 2 s, commit 15 s) |
| `result.size` | results stay under MAX_RESULT 256 KiB; large unpaged results are flagged |
| `page.limit` | `limit` honoured, default page ≤ 128, oversize limits clamped or refused |
| `page.cursor` | cursors terminate and never repeat |
| `page.order` | keyset order stable; no duplicates or skips across page sizes |
| `identity.stable` | identical calls give identical ids, digests and etags |
| `identity.id` | `id` resolves to itself; `external_id` is never swapped in |
| `write.roundtrip` | (writable roots only) write then fetch returns exactly what was written: no shadow or copied fields |
| `write.status` | (writable roots only) an unmappable status is refused or stored verbatim |
| `pending.retry_hint` | pending and busy refusals carry `retry_after_ms` |
| `concurrency.busy` | pipelined calls each get one reply; busy is retryable |
| `env.scrubbed` | plugin works under the host's scrubbed environment |
| `env.children` | (observed children) no forbidden or earlier-call attribution reaches a child |
| `models.present` | package ships `models/*.tla` with `.cfg` |
| `models.native` | each model checks with native `bboxy450-tlc`; `Broken*` controls fail |
| `models.java` | the same verdicts with Java TLC when available |

Writes run only in a doctor-owned temporary root or an author-supplied `--scratch` root,
never in `--root`.

## Red/green controls

One SDK `data_source` fixture (`bboxy450-term-plugin-fixture` `Kind::Doctor`,
`src/doctor.rs`), built once; its installed name selects the fault
(`bboxy450-term-data_source-doctor-<check-id>`). `tests/doctor.rs`:

| Check | Broken fixture | Flagged with |
|---|---|---|
| `shape.name` | clean binary installed as `bboxy450-term-nosuch-doctor` | unknown system token |
| `shape.descriptor` | `--shape` drops the `links` command | `missing_method: data_source_links` |
| `shape.digest` | `--shape` description carries a clock | two `--shape` runs differ |
| `shape.catalogue` | `tools/list` reply loses `data_source_links` | declared but not listed |
| `shape.skills` | `resources/read` returns other text | skill reads back different content |
| `contract.dto` | record refs without `digest` | `"digest" is a required property` |
| `call.refusal` | `fetch` of an unknown id panics | panicked (`internal_error`) |
| `call.cold_start` | `initialize` sleeps 2.3 s | initialize took 2307 ms, budget 2000 |
| `call.deadline` | first `search` sleeps 2.3 s | took 2304 ms, budget 2000 |
| `result.size` | bodies of 300 KiB | default limits exceeded MAX_RESULT |
| `page.limit` | `limit` ignored | limit 1 returned 8 items |
| `page.cursor` | `next` is always `"again"` | cursor repeats |
| `page.order` | pages overlap by one | id appears on two pages |
| `identity.stable` | digest and etag carry a call counter | list digest differs from fetch digest |
| `identity.id` | `resolve` swaps `id` and `external_id` | resolve returned its external_id |
| `write.roundtrip` | `status` moved to `bboxy_status`, `bboxy_signed_record` added | gained fields never written |
| `write.status` | unknown status silently stored as `open` | accepted but stored as `open` |
| `pending.retry_hint` | `search` refuses `workspace_pending` without hint | no `retry_after_ms` |
| `concurrency.busy` | exits when two requests are outstanding | process exited 70 with 12 in flight |
| `env.scrubbed` | `initialize` needs `CARGO_MANIFEST_DIR` | fails scrubbed, works with full env |
| `env.children` | child gets every `_meta` attribution ever seen | ATC_TOKEN from `_meta` reached the child |
| `models.present` | empty models directory | no `models/*.tla` |
| `models.native` | `PluginRuntime` invariant `<=` changed to `<`; a hand-edited `.tla` | expected pass, Inv0 violated; outside the native subset |
| `models.java` | the same broken invariant, Java TLC configured | Java TLC says safety |

Green: the clean doctor fixture (every non-model check passes, writes and an
observed child included), the root fixture plain and with the Harmony knobs,
and all eight SDK examples (each example's own `#[test]`, with the SDK's
shipped models, Java TLC included when configured). The `reference` fixture is
a canned P-0 case server (fixed records, repeating opaque cursors), so the
doctor reports gaps on it by design; it is not a green control.

## Acceptance

- [x] Every check above has a broken Rust fixture the doctor flags with that id, and a clean fixture it passes.
- [x] Passes on every SDK example (each example's own test) and the good root fixtures.
- [x] Run on the GitKB plugin; real gaps recorded below, the plugin unchanged.
- [x] Docs and Make targets as listed under Shape.
- [x] `make plugin-doctor-test`; affected plugin, mux and fixture suites.

## GitKB plugin result

First run (`make plugin-doctor PLUGIN=target/plugins/bboxy450-term-data_source-gitkb
DOCTOR_ARGS='--package plugins/gitkb --child git-kb'`, doctor-owned disposable root
with the checker's fixture bootstrap, git-kb 0.2.19): 19 checks passed, exit 1.

- error `models.present`: `plugins/gitkb` shipped no `models/*.tla`. Closed in the
  second commit (below), per the coordinator's decision to keep the doctor strict.
- warning `identity.stable`: two identical `data_source_fetch` calls of an
  uncommitted workspace document return different `gitkb.created_at`
  (e.g. `04:08:42.192488Z` vs `04:08:42.230471Z`); id, digest and etag are
  stable. Handed to the executor owning `plugins/gitkb/src/**` (`store.rs`
  already drops git-kb 0.2.19's read-time `modified_at` from catalog rows).
- not observable: `env.children` — the plugin starts its compile-time pinned
  absolute `git-kb`, so a PATH shim cannot see the child's environment.

## GitKB models (second commit)

Coordinator, 2026-11-02: keep `make check-gitkb-plugin` strict and ship models.
`plugins/gitkb/src/**` is untouched (another executor owns it). The Rust source is
`plugins/gitkb/models.rs` (package root, outside `src/`, so it cannot change the
plugin binary); exports in `plugins/gitkb/models/`; the gate module
`crates/bboxy450-term/tests/formal_contracts/gitkb_models.rs` includes it, adds
the cases with one `cases.extend` line and holds the drift test. Registered in
`verification/corpus.toml`, `contracts.toml` (three contracts) and the GitKB
crate's own `coverage.toml` entry (now `contract-model`).

| Model | What the code does (cited in models.rs) | Broken control |
|---|---|---|
| `GitKbIndexFreshness` | layer and snapshot labelled by the signal read before export; re-check after export; save before publish; load only for the exact signal | publish without the post-export re-check: safety |
| `GitKbPaging` | snapshot-bound offset cursors, at least one record examined per page, `stale_cursor` after a source change | page may stop before any record: liveness |
| `GitKbAttribution` | children get the scrubbed backend env set only at bind; index builds clone it | a call writes its attribution into the shared env: safety |

Today the plugin forwards no seat attribution to `git-kb` at all (that is the
`atc-attribution` branch); the attribution model therefore proves only the
isolation half, and the positive delivery is unmodelled until that lands.

## Decisions

- The doctor lives in the SDK (operator shape); `bbt-mux` sits above it, so the
  pure parts of `plugins/check.rs` moved down instead of being copied.
- `bboxy450-tlc` gains `parse` for its own export only; anything that does not
  export back byte for byte is "outside the native subset", never guessed.
- `bboxy450-tlc`, `jsonschema` and `sha2` became regular SDK dependencies; the
  `model` feature now only gates `models.rs`.
- Text content differing from `structuredContent` is a warning: the host reads
  only `structuredContent`.
- `identity.stable` is an error for identity fields (id, external_id, digest,
  etag, version, number, slug, path, next) and a warning for other drift.
- `testkit::example` builds an example plugin into the same private target as
  `bboxy450-term-plugin-fixture` `sdk_example`; folding the two is a follow-up.

## Unproven

- Deadline and paging behaviour on a large real root: the doctor ran GitKB on
  its own small disposable root only, never on operator data.
- `env.children` for plugins that start children by absolute path.
- `task_system` writes need host-signed records and are left to
  `bboxy450-term plugin check`; the doctor exercises its reads only.

## Progress log

- 2026-11-02: task written before code; scope and reuse plan above.
- 2026-11-02: doctor, fixture controls, docs and Make targets; GitKB run recorded.
- 2026-11-02: GitKB models shipped (second commit); `make check-gitkb-plugin` strict.
