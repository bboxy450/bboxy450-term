---
id: "01M6CV29FM9KQPJCGJ0Z21SGBT"
number: 5118
type: "report"
title: "Resource-ledger signal handlers precede child spawn"
task: "[[tasks/2026-11-02-term-resource-ledger-race]]"
commits: ["300026133ac696975699515297fa28bbc31c21f4", "ea90d343226e9b4361fee4434a3362ada115dbb7", "585ac6c7e2c48910fd3a1657cd5a8c7ca9db1d4b"]
verify: {"command": "python3 -I scripts/test-resource-ledger.py", "exit_code": 0}
author: "seat:01M6ADTTF9QWWFZFVZJHJ3DD8E"
created: "2026-11-02T01:13:32.584370Z"
---
# Resource-ledger child-start signal ownership

Task [[tasks/2026-11-02-term-resource-ledger-race]] (#5117), Executor E1 harness-core,
seat 01M6ADTTF9QWWFZFVZJHJ3DD8E. Worktree
`/Users/feral/mydev/bboxy450-term-ledger-race`, branch `resource-ledger-race`,
base dbd29ef02c47d3b54b6222e8598b048a15a2bd05. Implementation commit
300026133ac696975699515297fa28bbc31c21f4 is signed by Boxy. The containing
signed commit and its Report trailer bind this report and the unchanged task copy.

## Mechanism and witnesses

- `scripts/resource-ledger.py::run` defines the child slot and signal handler,
  then installs SIGINT/SIGTERM handlers before accounting setup or Popen.
  A signal delivered before the child handle is published is retained, with
  duplicate signal types coalesced (at most two). After spawn/attach, pending
  signals use the existing descendant-only forwarding path. Child waiting,
  ledger finish and normalization to 128+signal keep their existing behavior.
- `scripts/test-resource-ledger.py::signal_at_spawn_boundary` intercepts only
  the command's real Popen call in a fresh wrapper process. Immediately before
  returning its handle, it sends a real SIGTERM to that wrapper. A parent-owned
  pipe keeps the workload alive; closing that pipe also releases a child left
  by the intentionally broken wrapper. No sleep selects the race window.
  The companion case delivers the signal just before the child exists.
  Each suite asserts 143 for 32 after-spawn and eight before-child deliveries.
  Thirty-second waits are hang guards, not performance or percentile gates.
- `test_installing_handlers_after_spawn_is_detected` copies the fixture into a
  temporary directory, moves the registration block back after spawn/attach,
  and runs the same witness. It must fail specifically with
  `AssertionError: -15 != 143 : spawn-window signal took default action`.
  Missing dependencies, timeouts or other errors cannot satisfy this control.
- No existing ledger-wrapper lifecycle model/source mapping was found in the
  formal corpus. `verification/coverage.toml` has a separate, multiline
  `implementation_only` entry, ResourceLedgerSignalForwarding, naming source,
  tests, bounds, control and checks. These real POSIX signal/OS scheduling
  semantics are implementation-tested, not claimed formally proved. The full
  crate inventory and existing native/TLC parity remain required by make test.
- Project origin/license/policy headers were attached to the two updated
  project-owned Python files; the canonical ATTRIBUTION.md remains in force.

## Completed commands and evidence

On the original production implementation, with only the new witnesses added:
`python3 -I scripts/test-resource-ledger.py LedgerContracts.test_signal_immediately_after_spawn_is_forwarded_many_times LedgerContracts.test_signal_before_child_exists_is_forwarded`
exited 1, with two named -15-versus-143 assertion failures at iteration zero.
`target/resource-ledger-race/baseline-red.log` and `.exit` retain this red run.

After the fix, the exact ledger suite exited 0 (37 tests). A separate late-handler
mutation run exited 1 at the required assertion; restoration plus both boundary
witnesses exited 0. `control.json`, `late-handler-red.log`,
`late-handler-green.log` and `control-green.log` retain this progression.
The first combined focused run failed only because the temporary control fixture
omitted inference-ledger.py's wake-ledger.py dependency. Adding that copied
fixture dependency fixed the control; the initial output is retained in
focused-green.log with exit 1. It is not counted as successful control evidence.

`python3 -I scripts/test-resource-ledger.py` ran 50 times consecutively on signed
final source 585ac6c7: every exit was 0, with no retry. Each log reports 37
passing tests: 1,850 tests, 2,000 forced forwarded SIGTERMs and 50 detections of
the broken late-handler variant. `suite-50.json`, `suite-50.exit` and
`repetition-evidence.json` retain the source, command, all iteration statuses and
SHA-256 of each individual suite-01.log through suite-50.log. Python source bytes
are unchanged in this report commit.

Before the final repetition run, the bootstrap was found writing a Python
bytecode cache into the author worktree. Fixture follow-up ea90d343 disables
bytecode writes; only that generated cache was removed. The first 50-run batch
on 30002613 passed but is preserved under initial-50, not substituted for the
final-source evidence. The next batch stopped at run 17 when the existing
CPU-units witness timed out on its two-second global ps walk. Follow-up
585ac6c7 supplies only that test's known self PID/parent/group for discovery,
while retaining real proc_pid_rusage, mach_timebase_info, getrusage reads and
the original numeric assertions. Production ps timeout/forwarding behavior is
unchanged. The failed batch is retained under ps-timeout-batch. The final
50-run batch passed on 585ac6c7 and checked the absence of source bytecode after
every suite. All three implementation/fixture commits are signed.

The first `make test-boundaries` stopped with exit 2 before any test because the
new worktree's harness submodule was uninitialized. A misnamed harness-init
attempt also exited 2; the actual `make init-submodules` target exited 0 and
checked out approved pin 47b140fa9bd2bec99054394c206deb1b36d7b3b0. The boundary
target was then restarted and finished with exit 0. Setup failures are retained
separately; they are not reported as passing tests. The final cockpit process
cases reported seven passed, with no failures or ignored cases in that batch.

## Final committed-candidate verification

This immutable report records the completed witnesses and repetition run above.
The exact `make test` gate is still pending while this report is prepared. It
will run on the containing signed candidate; the task_report handoff must give
its actual exit code, final SHA, current-source formal receipt and PR.
The completed `make test-boundaries` command exited 0; its runtime source inputs
are unchanged by this report-only commit. This paragraph does not claim
future checks passed. A gate failure must be retained and resolved before handoff.
Raw logs and throwaway identities stay outside Git under this worktree's target.
No production land.sh, merge, release or live-provider request is authorized.

The task copy preserves the authenticated source file byte-for-byte solely to
resolve this report's task link in the committed tree. Its SHA-256 is
2276c44bdea03736da659f715e9131d542783d4191c42e927877cc3589b5bc15.
The report's local number is the candidate produced by scripts/next-number.sh;
that script documents that corpus rejects races and the candidate is not a
reservation. The live MCP catalogue still omits report creation and task_report's
optional report argument, so the authorized repository report is created locally
and its qualified locator will be carried in the task_report summary.

## Retained checksum inventory

- `target/resource-ledger-race/baseline-red.log` — SHA-256 `430dfbed374d9592399abace75db02ce144def674f36f3f46a4e5c8d18a34cf1`.
- `target/resource-ledger-race/baseline-red.exit` — SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`.
- `target/resource-ledger-race/suite-green.log` — SHA-256 `7dbe0a816befe234301e52330bee70bd98917c897800d257372857c2e49e3926`.
- `target/resource-ledger-race/suite-green.exit` — SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`.
- `target/resource-ledger-race/focused-green.log` — SHA-256 `dcca7e751803e25c656a16f551f670a68e70b60e65af3dc5796a6a294c07a9d2`.
- `target/resource-ledger-race/focused-green.exit` — SHA-256 `4355a46b19d348dc2f57c046f8ef63d4538ebb936000f3c9ee954a27460dd865`.
- `target/resource-ledger-race/control-green.log` — SHA-256 `0ab7aa75866acfa8827c1cd7791a0a148fc27cedeb8cf7cf7eff7b42df584f4f`.
- `target/resource-ledger-race/control-green.exit` — SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`.
- `target/resource-ledger-race/late-handler-red.log` — SHA-256 `2a7e0c09b8a0a8d210bdb8f8db3429bfb174275e8e2d1378aa46f1695b7e2aa4`.
- `target/resource-ledger-race/late-handler-green.log` — SHA-256 `25a3a683bbfc6a68e0deab3a8302495425a3f608ea19700fd46a096f06018e7a`.
- `target/resource-ledger-race/control.json` — SHA-256 `52cebef1ff87e120ccfa81c39c0c7e89a3b570be5bc868ecdeb9a4ae745dc2a2`.
- `target/resource-ledger-race/suite-50.json` — SHA-256 `06490af0225f422d479fa2d26ab78fcce7990f0ca8efbb7aad4688e3ac558156`.
- `target/resource-ledger-race/suite-50.exit` — SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`.
- `target/resource-ledger-race/repetition-evidence.json` — SHA-256 `78afc17ddaa71309a84dea7a766f9dca055c5f3b58efe4e75ec0a317dd70508a`.
- `target/resource-ledger-race/test-boundaries-before-init.log` — SHA-256 `577caf6a0cca28c561d02d53fdfb781a8e3f8026d84c66da6f2e2b6c68f37679`.
- `target/resource-ledger-race/test-boundaries-before-init.exit` — SHA-256 `53c234e5e8472b6ac51c1ae1cab3fe06fad053beb8ebfd8977b010655bfdd3c3`.
- `target/resource-ledger-race/setup.json` — SHA-256 `653c205c03e1c5ef067709de43dba5856fc3b3abbf3495d3c916b4a602d2f3bb`.
- `target/resource-ledger-race/initial-50/suite-50.json` — SHA-256 `a17be5192d49d02f783ec483fa65afddc948485f77d109efd4f91b9ede0a2430`.
- `target/resource-ledger-race/initial-50/repetition-evidence.json` — SHA-256 `1407176386ee158f69cbe2a0d25f4a97e2059dd71b46ddf939e18725e8ad93df`.
- `target/resource-ledger-race/ps-timeout-batch/suite-50.json` — SHA-256 `a90b190b1ebb094339bd62af6d5a40976a0780452759d1c73bf42e1e42da6657`.
- `target/resource-ledger-race/ps-timeout-batch/suite-17.log` — SHA-256 `2daac1d9318060b0ab919ffcffa73a506e9ac4da039b7a3619374407f6c5c06f`.
- `target/resource-ledger-race/test-boundaries.log` — SHA-256 `d6832ffec5842724e9e3a9fc9b47740b8ad157b75911b73d74769f96b5048cf2`.
- `target/resource-ledger-race/test-boundaries.exit` — SHA-256 `9a271f2a916b0b6ee6cecb2426f0b3206ef074578be55d9bc94f6f3fe3ab86aa`.
