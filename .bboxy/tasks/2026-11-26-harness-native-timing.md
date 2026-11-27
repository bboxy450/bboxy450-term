---
allowed_paths:
- crates/bbt-mux
- Makefile
- scripts/ci-local.sh
- scripts/test-dream-c1.py
- scripts/bundle-macos.sh
- Cargo.toml
- Cargo.lock
- crates/bboxy450-harness
- crates/bboxy450-harness-wake-up
- crates/bboxy450-term/tests/swarm_workdir.rs
- crates/bboxy450-term/tests/swarm_workdir
- crates/bboxy450-term/tests/native_signing.rs
- crates/bboxy450-term/tests/dream_c1.rs
- crates/bboxy450-svg/install_chart_runtime.py
- verification
assignee: seat:01M8D6TMCX8GCFM1TN8QBVG4G8
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8vBJJUo859ud3HChzp/5sSde2DPae2JxjC/mzluItnA=
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
closed:
  by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  op_id: 01M8FBQ8Z8PCZE82WNRY399AG4
created: 2026-11-26T21:12:11.301Z
created_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
delegated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
id: 01M8CV76MZYYMPQ5F676KEF97T
number: 1794292861628001
report:
  commit: 201e17ef1e42cc241b16ecb6c3d1cdfba92bd89d
  op_id: 01M8FBNZWVNT0MFHESEDVERT5X
  report: '[[reports/2026-11-27-harness-native-timing-landed.den]]'
  reported_by: seat:01M8D6TMCX8GCFM1TN8QBVG4G8
  summary: '[[Implement]] [[Verified]] Landed signed `201e17ef` on term main and `7cd999f3` on harness main. Full `make test` and `make land` exited 0. All six native mux fixtures passed in the spaced checkout, with 60,207 ms journal-based staleness and one ACP exit notice. The approved worker matched across two checkout paths at `1a763a71…`, and the restored GUI handoff held the tool 21,145 ms before explicit release and replayed its single completion. The patch removes Make path-list expansion, the test-local expiry clock, the raw claim rewrite, build-checkout dependencies, the detach countdown and the JSON from `Contents/MacOS`. Real ad-hoc codesign and the clean rebased 1,008-model receipt passed.'
  verify_revision: 0
role: executor
status: done
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'test-harness-native: early status expiry, missing ACP exit notice, and the spaced TARGET_DIR'
type: task
updated: 2026-11-27T20:39:04.011Z
updated_by: seat:01M89ZRSC32VXKQKRMXZ87PFCT
verify: make test
verify_result:
  command: make test
  evidence:
  - criterion: Full default-target verification
    reference: target/harness-serial-watchdog-make-test.log and target/ci-local.log:3,5,2729, signed pre-land bdf3f86a, exit 0.
  - criterion: Native space handling, quiet clock, and ACP notice
    reference: target/ci-local.log:2385,2395,2448 records 60,207 ms and all six passed.
  - criterion: Reproducible worker and reviewed bump
    reference: target/harness-worker-two-paths-explicit-strip.log:227,448–452 and target/harness-bump-explicit-strip.log. Both measured SHA-256 1a763a711e55575adfa590d18c4d8c741217c2d93796b1fc3299a2ff73a81cd6.
  - criterion: Upstream IR portability and landing
    reference: target/harness-ir-proposal-check.log and target/harness-upstream-link-land.log:11240,11265. Both harness land invocations exited 0.
  - criterion: Private GUI UV, release, and causal control
    reference: target/harness-rich-detach-release.log, target/ci-local.log:2566,2600 and target/rich-gui-handoff-control/restored.log:18,38,47.
  - criterion: Sealed macOS receipt layout and native signing
    reference: target/harness-allocated-bundle-receipt-unit.log; target/harness-real-resource-signed-handoff.log:55–60 shows codesign exit 0, exact proof bytes and only three Mach-O app binaries.
  - criterion: Landed rebased source
    reference: target/harness-native-term-land.log:1123,1147,1656–1658. Both tracked worktrees clean and both HEADs on their origin/main.
  - criterion: Current-source formal verification
    reference: target/harness-rebased-formal-refresh-2.log:1708,1712. 1,008 models, 309 contracts, 42 crates. Archived SHA-256 28c33cdc7410276b6532f4291d4f92da59917c57f73271a72e5fb4c19e558906.
  exit_code: 0
provenance:
  digest: sha256:ce756466dcf6b3c88f600bdde3fc3b82eb79b8bf0d8d083196dc4136921edca4
  frontmatter_digest: sha256:9394ba71b6bdeab327edaea0b297929af7159a37a6cbb0ba252835ea184441c0
  signer: seat:01M89ZRSC32VXKQKRMXZ87PFCT
  sig: JE3BN0JH5V+oscZ35enqsr4G9ys68GTP7FwRZSsFr7iJSkQahLzds5bp4KpdPi0FZ+t8udIla0TOMXIEXKetCQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/2380
  event:
    id: 01M8FBQ9Q285K5C06G4J4E4Y4D
    sequence: 2380
    hash: 350b603de46d224e4c94047188f22b3df5c975c5d725b4906b0e04853d417e1f
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

## Oracle ruling 2026-11-27: both blockers are yours; steps 1–3 are accepted as observed
I read `d0f66b159`'s diff stat and your report. The space, clock and indexed-claim fixes stay as they are. Both remaining blockers fail in **every** managed worktree, not only yours, so main cannot turn green without them. They are in scope now; `allowed_paths` has grown to cover them.

### 5. The worker executable must not depend on the checkout path
In your worktree, source `fb4bcfa` and compiler `408acb04…` produce executable `51b704dd…`. The approved hash `a1cbcd6f…` was measured in the dream-c1 workspace, whose checkout path has since been released. Same source and same compiler with a different binary means the build embeds something specific to the machine or the path. An executable approval that depends on the path can never match again, so it proves nothing.
1. Confirm the cause from the two binaries, or from one binary and its path strings. For example, `strings | grep implementation-workspaces`, or diff the two outputs. Record what differs.
2. Make the worker build path-independent inside the gate's own build command (`scripts/test-dream-c1.py:543`, `:3409`, and the `bump-harness` staging). Keep the gate's refusal of ambient `RUSTFLAGS` (`:600`). The remapping belongs to the gate's recorded command or to a Cargo profile, not to the environment. Use a profile `trim-paths` setting if the pinned toolchain supports it; otherwise use an explicit `--remap-path-prefix` that the gate itself passes and records.
3. Prove it: build the worker from two different checkout paths (your worktree and one temporary copy you create and delete) and show the same `shasum -a 256`.
4. **Pre-granted:** once step 3 shows two matching hashes, stage the new tuple with the `make bump-harness` staging path (`--stage-worker-build`) into `crates/bbt-mux/src/logic/dream-c1-worker-build.txt`. Use the same source and compiler and the new path-independent executable hash. Put both measurements in the report. That proof is the approval; you do not need another ruling from me. Do not stage a tuple without the two-path match.

### 6. The private GUI fixture finds `uv` explicitly
`crates/bboxy450-term/tests/swarm_workdir.rs:82-87` clears the environment and excludes `/opt/homebrew/bin`. `crates/bboxy450-svg/install_chart_runtime.py:29` calls bare `uv`.
- Keep the private PATH as it is. Instead, the fixture resolves `uv` on the host before clearing the environment and passes its absolute path explicitly, for example `BBOXY_UV`. The installer uses that path and falls back to bare `uv`.
- If no `uv` exists on the host, fail with a message that names it. Do not skip the test.
- Keep one name for this variable. If the product already has a variable for `uv`, reuse it.

### Verify
Verify stays `make test`, on the final commit, once. After that, `make land`. Report once, with:
- the full `make test` log path and exit;
- the two-path worker hashes;
- the `uv` fix evidence.

## Oracle ruling 2026-11-27 (second): fix the harness at its source
I checked your diagnosis against the harness source. `crates/bboxy450-harness/src/ir_scan/proposal.rs:112` has `let root = Path::new(env!("CARGO_MANIFEST_DIR"));`, and `:121` runs `python3 -I scripts/ir-compat.py check` with `current_dir(root)`. That is a product defect as well as a hash problem: a shipped harness binary would look for `ir-compat.py` on the build machine's path. Path remapping cannot fix it, as you showed. The harness is now in your `allowed_paths`.

The flow is the same as tasks/2026-11-24-harness-test-debt. Commit inside the harness submodule and land it on harness `origin/main` with the harness's own gate. Then `make bump-harness REV=<sha>` in term, which also restages the worker tuple. Then continue in term.

### 7. Harness: no build-path literal in the product binary
1. The IR proposal check must not use the build-time manifest directory. Find out what `ir-compat.py` actually reads: its schema and fixture files, relative to what. Then pick the smallest option that holds:
   - embed the script, and whatever it reads, with `include_str!` and run it through `python3 -I -` on stdin, or from a temporary file;
   - or resolve the root from runtime input the caller already has: the scan `dir`, or an explicit argument.

   Do not use an environment variable as the only source, and do not keep a fallback to `CARGO_MANIFEST_DIR`.
2. The other `env!("CARGO_MANIFEST_DIR")` uses (`src/dream/bootstrap.rs:968`, `src/dream/read_wire.rs:617,640`, `src/frontends/gui/bridge.rs:2203`) may stay only if they are `#[cfg(test)]`, so they are not in the release worker. Check each one and say which it is.
3. Keep or add a test that runs the proposal check from a different working directory than the source tree.
4. The step 5 proof now applies to the new harness source. Build the worker at two checkout paths, get the same `shasum -a 256`, and `make bump-harness` stages the tuple: new source, same compiler, new executable. Two-path match is still the approval. Keep the remap flags only if they are needed for the match; say which.

Verify is unchanged. Report:
- the harness commit and the harness land log;
- the bump commit;
- the two-path hashes;
- the full `make test` log and exit;
- the `make land` log and exit.

## Oracle ruling 2026-11-27 (third): rich_detach holds the tool open, not a countdown
I read `crates/bboxy450-term/tests/swarm_workdir/rich_detach.rs`:
- `:434` makes the tool `sleep 30`;
- `:529-533` then asserts for 21 more seconds after `ui.wait()` that the tool PID is alive.

With a handoff of about 9 s under load, the tool ends normally at about 30 s, inside the survival window. Your journals show completions at 30,034 and 30,099 ms. That is a race between two clocks in the test, not a product failure. `crates/bboxy450-term/tests/swarm_workdir/` is now in `allowed_paths`.

### 8. Explicit release
- The tool blocks until the test releases it, for example by waiting on a FIFO or a release file that the test writes. It must not end on a timer.
- The test releases it only after the survival check. It then asserts that the turn completes normally, and that the completion appears exactly once in the resumed rich face.
- Keep the 21 s survival window and every existing assertion.
- Add an upper bound so a lost release fails the test with a clear message instead of hanging. Use the fixture's existing timeout style.

Verify and report unchanged. Add the rich_detach run to the report, with its log path and exit.

## Oracle ruling 2026-11-27 (fourth): the receipt is a resource, not code
`target/harness-ci-41f60afd.log:2595–2596`: `codesign` refuses the bundle because `Contents/MacOS/bboxy450-harness.dream-c1.json` is a non-code subcomponent. `Makefile:876` (from 1af3c0ca2d, 2026-10-26) copies the receipt there because the product finds it as the binary's sibling (`binary.with_extension("dream-c1.json")`, `crates/bbt-mux/src/logic/dream_c1.rs:9712`, `:9733`, `:9868`, `:10005`). It was latent until the admitted release worker existed. It fails every signed bundle, so it is yours. `native_signing.rs`, `dream_c1.rs` (tests) and `scripts/bundle-macos.sh` are now in `allowed_paths`.

### 9. Packaged receipt layout
- **Layout (decided):** in a bundle the receipt lives at `Contents/Resources/bboxy450-harness.dream-c1.json`. Outside a bundle it stays the binary's sibling.
- **One lookup:** add one function in `dream_c1.rs` that maps a worker binary to its receipt path. When the binary's parent is `MacOS` and the grandparent is `Contents`, it returns `Contents/Resources/<file name>.dream-c1.json`; otherwise it returns the sibling. Replace the four `with_extension("dream-c1.json")` call sites with it. No fallback that reads both locations.
- **Binding is unchanged:** the receipt still binds the exact worker bytes by hash. Moving the file changes no seal field. If the seal records the receipt's own path, say so and update it to the new one.
- Update `Makefile:876` (copy, and the `rm -f` on the else branch) and `bundle-macos.sh` if it copies the receipt, to the Resources path. Update `native_signing.rs:338-341` to expect the Resources path, and assert that nothing non-Mach-O remains under `Contents/MacOS`.
- Add a unit test for the lookup: bundle path → Resources; plain path → sibling.

Verify and report unchanged. Add the signed-bundle step's log lines (codesign exit) to the report.


## Report from seat:01M8D6TMCX8GCFM1TN8QBVG4G8

[[Implement]] [[Verified]] Landed signed `201e17ef` on term main and `7cd999f3` on harness main. Full `make test` and `make land` exited 0. All six native mux fixtures passed in the spaced checkout, with 60,207 ms journal-based staleness and one ACP exit notice. The approved worker matched across two checkout paths at `1a763a71…`, and the restored GUI handoff held the tool 21,145 ms before explicit release and replayed its single completion. The patch removes Make path-list expansion, the test-local expiry clock, the raw claim rewrite, build-checkout dependencies, the detach countdown and the JSON from `Contents/MacOS`. Real ad-hoc codesign and the clean rebased 1,008-model receipt passed.

```json
{
  "op_id": "01M8FBNZWVNT0MFHESEDVERT5X",
  "commit": "201e17ef1e42cc241b16ecb6c3d1cdfba92bd89d",
  "report": "[[reports/2026-11-27-harness-native-timing-landed.den]]",
  "verify_result": {
    "command": "make test",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "Full default-target verification",
        "reference": "target/harness-serial-watchdog-make-test.log and target/ci-local.log:3,5,2729, signed pre-land bdf3f86a, exit 0."
      },
      {
        "criterion": "Native space handling, quiet clock, and ACP notice",
        "reference": "target/ci-local.log:2385,2395,2448 records 60,207 ms and all six passed."
      },
      {
        "criterion": "Reproducible worker and reviewed bump",
        "reference": "target/harness-worker-two-paths-explicit-strip.log:227,448–452 and target/harness-bump-explicit-strip.log. Both measured SHA-256 1a763a711e55575adfa590d18c4d8c741217c2d93796b1fc3299a2ff73a81cd6."
      },
      {
        "criterion": "Upstream IR portability and landing",
        "reference": "target/harness-ir-proposal-check.log and target/harness-upstream-link-land.log:11240,11265. Both harness land invocations exited 0."
      },
      {
        "criterion": "Private GUI UV, release, and causal control",
        "reference": "target/harness-rich-detach-release.log, target/ci-local.log:2566,2600 and target/rich-gui-handoff-control/restored.log:18,38,47."
      },
      {
        "criterion": "Sealed macOS receipt layout and native signing",
        "reference": "target/harness-allocated-bundle-receipt-unit.log; target/harness-real-resource-signed-handoff.log:55–60 shows codesign exit 0, exact proof bytes and only three Mach-O app binaries."
      },
      {
        "criterion": "Landed rebased source",
        "reference": "target/harness-native-term-land.log:1123,1147,1656–1658. Both tracked worktrees clean and both HEADs on their origin/main."
      },
      {
        "criterion": "Current-source formal verification",
        "reference": "target/harness-rebased-formal-refresh-2.log:1708,1712. 1,008 models, 309 contracts, 42 crates. Archived SHA-256 28c33cdc7410276b6532f4291d4f92da59917c57f73271a72e5fb4c19e558906."
      }
    ]
  }
}
```
