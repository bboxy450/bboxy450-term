---
type: decision
title: 'C1 worker: OS first-launch admission is a provisioning step, not part of the 250ms Hello window'
status: decided
derived_from:
- '[[decisions/d93377961366058-2026-11-06-dream-c1-durable-commit-and-tuple]]'
- '[[tasks/2026-11-05-dream-c1-consumer]]'
id: 01M6S2W07WA3421KNFQAM2WYXQ
updated: 2026-11-06T19:19:39.604Z
created: 2026-11-06T18:45:22.129Z
number: 93377961366076
created_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
updated_by: seat:01M6RZRGC3194VVFBC4M7XW4MF
authorship:
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  kind: seat
  public_key: DbWCjlMnSNUcBZMB2JrdfJ36/iQgSFRbtz78XE3WlhU=
  process_generation: '1'
provenance:
  digest: sha256:6b79f966323f7f3799c803b6650d0d8aca7b597445887eb4727175fbf298b1fb
  frontmatter_digest: sha256:198da775c9c588d3c0bad348fe4daa1ad373f4c3ec923e6716bb2d2f550fea46
  signer: seat:01M6RZRGC3194VVFBC4M7XW4MF
  sig: swyfOCt/7bOmXIHwt8HLGf8JCZyaABRZEDp9Er+PQtHb/YEFfhlWLkfkWYvqMLuba+CtAcKgH9ef6K083MVRAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149768
  event:
    id: 01M6S4TSY84S3V98BS0QPCPH3H
    sequence: 149768
    hash: f19f1cfc66ca977468ca7bec08cade85c7ae3631ddb4d6a335c5d6b37219f63d
---
# First-launch admission happens at provisioning

Oracle ruling, 2026-11-06, seat 01M6RZRGC3194VVFBC4M7XW4MF.

## Evidence

- The one ordinary trial (seat 01M6RZZY4FR22NC0Q6DAZGHYKC, term cba756bd, worker 2b85c711) failed the 250 ms Hello. Entry came 288.6–362.6 ms after spawn returned, while recognition took only 58 µs and write observation 107 µs. PID 13754 exited 2 and was reaped. Receipts are in `/var/folders/…/bboxy-dream-c1-gate-1sqmqa2v/`.
- Oracle measurement: the same binary (`target/release/bboxy450-harness`, SHA-256 2b85c711…, ad-hoc linker-signed) was copied to a fresh path and run with `--version` four times: **425.0 ms, then 11.6, 13.1 and 11.3 ms**. The cost sits before entry and appears only on the first exec of a new executable file. That is macOS's first-launch assessment of a new ad-hoc-signed binary (signature and policy evaluation), not worker startup. (The worker rejects `--version` with exit 1, but the exec and the OS assessment happen before argument parsing, so the timing stands.)
- Persistence is solved without the `durable_commit` lane: PERSIST mode gives a cold maximum unit of 437,833 ns and cleanup of 113,416 ns, with focused 3/3. Ruling 1(2) of decision 66058 is therefore **not** used.

## Ruling

1. **Provisioning step.** Preparing an immutable worker executable ends with one **OS admission exec** of that exact provisioned file, inode and hash, using a side-effect-free mode (**`--help`**; see the correction below) and no account, KB or Dream state. Record its wall time, exit code, path, inode and hash in the provisioning receipt as `first_launch_admission`. It counts as provisioning cost, once per provisioned file. It is not part of a run, not a warmup of the run's process, and never a retry of a failed run.
2. **The 250 ms Hello window stays unchanged** and measures the worker's own startup on an admitted file. Any change to the binary, path or inode invalidates the admission; the next provisioning repeats it. If admission exceeds 2 s or exits non-zero, provisioning fails.
3. **One new run is authorized.** The tuple is unchanged (2ce4359d / e003fd3d / 2b85c711; persistence was term-side). After admission on the exact provisioned file, run the ONE joined ordinary run from its original origin with strict entry evidence. The failed trial stays recorded as failed. There is no other retry.
4. **Cold MCP Status/page traces (1.44 s / 2.93 s).** Measure them cold versus warm the same way before any redesign. If the excess is first-launch admission of a new binary, rule 1 covers it. Otherwise, report per-phase CPU and wall against the existing 1 ms / 9× / 250 ms law; a slow phase gets split, never exempted. Latency is not accepted until it is measured.
5. **Model and contract.** Admission is a provisioning-state transition that must precede `Start`. Add a broken control that starts a run on a non-admitted inode; it must be distinguishable in the evidence.

## Correction (Oracle, 2026-11-06)

- **Flag.** `--version` was wrong: the worker implements only `--help` (`headless.rs:265–273`). The executor's `--version` admission exited 1 in 22.184 ms (PID 54097, stderr 'unknown option --version') and stays recorded as a **failed admission**. One fresh admission with `--help` on the same exact file is authorized.
- **Rule 4 result.** The latency is measured, and it is not admission: warm runs are as slow as cold, and wall time is roughly 300–600× CPU time because read calls pay paced rest. It moves to [[tasks/2026-11-06-dream-c1-cold-mcp-latency]] (serve reads from a prepared snapshot). Under the operator's landing override it is a known limit, not a land blocker.
