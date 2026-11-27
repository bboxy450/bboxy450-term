---
allowed_paths:
- crates/bbt-mux
- Makefile
- scripts/ci-local.sh
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
created: 2026-11-26T21:12:11.301Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8CV76MZYYMPQ5F676KEF97T
number: 1794292861628001
role: executor
status: open
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'test-harness-native: early status expiry, missing ACP exit notice, and the spaced TARGET_DIR'
type: task
updated: 2026-11-26T21:12:11.301Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test
provenance:
  digest: sha256:27dd8aa76961ae8e11019c267b8a22568f110df65ca55086790e12dd782c4f1d
  frontmatter_digest: sha256:5961802e3a644cbc6bd4c0ae82866f2887562079199afa9ad989f7cace724968
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: 1DD6Wjv4lB2z630uyuP91tVSUHRRfTYug8HSBZozP8321HLeX14qUlmv9sgejKr+g5wxddWy5tkEq3bhA4Y5Aw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2280
  event:
    id: 01M8CV77GKAKT04MMSNSV2QK6M
    sequence: 2280
    hash: 822ffae6914f2df9fb75aa2816fa21e00131660887fef1cf8708505816ef917f
---
Our stand: subtract before you add. A test that times the machine instead of the product is measuring the wrong thing.

## Evidence (Oracle, 2026-11-26)
Main has never shown `test-harness-native` green since harness fb4bcfa. My `make test` on 62318feea stopped at `test-harness-source`, so it never reached this stage. The dream-c1-gate-repair executor's run on signed `38a9fbdd5` reached it. That branch touches only the Dream C1 files and the Makefile, none of bbt-mux's tests. Its log is in the dream-c1 workspace at `target/dream-c1-repair/harness-native-alias.log`.

1. **The managed worktree path has a space in it.** The path is `~/Library/Application Support/bboxy450-term/run/implementation-workspaces/checkouts/<id>`. With `TARGET_DIR` under that path, six `mcp_harness_seat_*` tests failed in about 0.09 s (`full-test-timed-shell.log` tail). A no-space alias target, `/private/tmp/bboxy-dream-c1-target.*`, brought four of them back. Every managed workspace hits this.
2. **Early expiry.** `mcp::mcp_seat_isolated_process` panicked at `crates/bbt-mux/tests/mcp/harness_status.rs:162` ("early expiry"). The status went `source: stale` ("bash · no word for 1m") before the test's own `Instant` reached 59.5 s, on two runs out of two. Machine load at the time was about 7 on 10 cores.
3. **Missing ACP exit notice.** The same test panicked at `crates/bbt-mux/tests/mcp.rs:5132` ("delegator received no ACP exit notice"), twice. `mcp_harness_seat_agent_exit_notifies_claim_delegator_once` also failed.

## Do
1. Make `test-harness-native` and the tests it runs work when `TARGET_DIR` or the worktree path contains spaces. Fix the quoting or path handling where it breaks. Do not use an alias as the fix.
2. Early expiry: work out whose clock is wrong. The daemon starts its 60 s quiet clock at the last word, but the test measures from a later point. If that is the cause, the test should assert against the observed time the daemon reports, not against its own wall clock. If it is a real product expiry before 60 s, fix the product. Do not widen the 59.5 s bound.
3. Exit notice: find out whether the notice is lost, raced or delivered late. Fix the cause. A product race also updates its model and broken control (`verification/`, per AGENTS.md formal rules); if that is needed, report it and it will be granted.
4. Run the full `make test` once on the final commit, from your managed worktree with its default `TARGET_DIR`. Check `pgrep -f ci-local.sh` first. Then `make land`. Report the log path and exit.

Two named exceptions are allowed in that log:
- the four `bboxy450-yarn` loom broken controls, which fail by design;
- Dream C1 command 7 Store witnesses, which are owned by `tasks/2026-11-26-dream-c1-store-pacing.md` and are not part of `make test`.

Anything else that is red: fix it if it is in your allowed paths, otherwise report it with evidence.
