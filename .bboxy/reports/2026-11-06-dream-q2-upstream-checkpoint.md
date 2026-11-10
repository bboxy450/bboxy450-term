---
type: report
title: 'Dream Read Q2 upstream checkpoint: QoS prerequisite unmet'
task: '[[tasks/2026-11-05-dream-mcp-verified-snapshot]]'
author: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
created: 2026-11-07T03:31:56.896Z
commits:
- 238342d6a2034a2a695f8f007d2f17c91b6c55e4
- ca043659a66340b49e780e214142a8ac197fa237
- 8d6c0f24f66818e70b16d75273bf3bf5b02d7d2d
- 6ba34deea041599102b2778e46b318312390dc15
- feea6134ea3a7ee620955b65d752662796b68f12
- 9e4b6bb80f5eb6e7de37111c61a96edb6ee11dac
- b95f4106338d722a5ea4cc208a2f9e5c07ef0787
- 3c2e1d73d9cbf5f3b394a741c8dec03459da189b
- 0efadf1fa6f9fa68523490039c75bf86e1671f91
- 121627ce4316315d72c5f2bcd700bdc6546d0a6c
- ea59c063b29e822934f4e0b53defca3d8fdd270d
- 4841937c4927e923eac27fe8b54ffb7ce4b120de
- 7719e4c3b23b300166a17c9b9685609f28be1a01
- 5b3234afcc03194b7492deec652eda9400690a28
- d95eab5961e8e746c1b30609797eb21bf61f29b2
- 3e0e093db6da878b6c691cbdc73cee238685e36d
- 9e9a1ff66b2b444daf1cf5a86f8139d42f9410d4
- 543b12cc9f88e551c83d57f5fa24c7e2ae92c291
- 4ca39a2ea39aff1ea0b7b2bb37d5eb295abd7e42
- 0b2f3f2c8f659c0d446cd5261f2f1816c49c51b2
- 0d6783a931e29e6dd53fc9fe43e2bcc78eb13aee
- f64095fc61065bacb54c2519a9c505dcae8aaf8c
- 24be6edd4b1de868a2386834e77958de3da04b56
- 864a1e2f6f0889ad5a4f98ef901fd499378929cf
- 9edbfd651eac17006ff87f40f9eaad3a2ce1363b
- d57cc978e643554972354134fe1df18c804a2419
- 7ea8a494aa6e3f042789e3a52ea67782b433ffbd
- 819858371f057e4b2f4113d120201d38ecf6f8b2
- dfa2c929083466e92dff178878b4f204e77bb467
- fc5bbe40ef1cf88301c3717938a6b819952e184c
- 1e82a439640f4b209e986eb8648532084afc6807
- f9a90a485b630f61be4dec7f884773f2a8863d4c
- 3c104d1a3bb7dd99fad9a0b833de8c84b84f2298
- 34a4f567f38dbd04d81e627ef22fafb2b963607b
verify:
  command: python3 -I target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/landing-observer-preservation-controls.py
  exit_code: 0
supersedes: '[[reports/2026-11-06-dream-q2-final-verification]]'
id: 01M6T105E16KC2HQN514HSC8RV
updated: 2026-11-07T03:31:56.896Z
number: 93377961366215
created_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
updated_by: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
authorship:
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  kind: seat
  public_key: 3lIiKAyaYQbqAYQ3xsrS+NTGLp9s//BjqCA4HkK5nF0=
  process_generation: '1'
provenance:
  digest: sha256:f89b4796046d96ac9476b9b99ab616b6e9d4e3210f3c092732c47d66daf10179
  frontmatter_digest: sha256:bc980caeceeb13f850a709d2342f3348f27c3c06497354623e3b56841341d7f6
  signer: seat:01M6RZZXSHZT3KHBK1QC7K3XB3
  sig: 9vtJdcBciewW387EOq22/KrdEaQjDrZCZYhwtN1ezv0J1Oeror8NcN0E+IZXSeqBshPWOxmXUfQWFIjE0bDnCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150149
  event:
    id: 01M6T106YWCFBH2TG00A09P3JR
    sequence: 150149
    hash: 9860635ceb56a75e3b33b3cc58d5e06ad6b2924a29253fa2820ca3cbc7014a43
---
Q2 is **not landed**. The upstream GUI/formal fixes are integrated and the scoped landing-evidence preservation fix is signed. The remaining prerequisite is the term QoS landing required by Oracle message `01M6T02FX70DE8TD17368GJ785` (sequence1282), which is absent from the remote snapshot below.

This supersedes [[reports/2026-11-06-dream-q2-final-verification]] as the current handoff. That report retains the complete acceptance matrix, 19 explicitly deferred tests, previous red runs, immutable C preparation and proof boundaries. No deferral, runtime budget, deadline, qualification requirement or public wire was changed here.

All relative paths use `/Users/feral/mydev/bboxy450-harness-dream-read-q2`. `R` is `target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3`.

The current signed head is `238342d6a2034a2a695f8f007d2f17c91b6c55e4` (signature G, Boxy <bboxy450@gmail.com>). There are 34 signed branch commits above `origin/main=38d99096f4f9c475a5547cf2192982af6ff7a5c0`. The worktree and index are clean. Snapshot: `R/upstream-checkpoint-state.json`, SHA256 `7b892d051414b1c4536d154ada6d1ded5b378eb8affb2ee3d312b190b2dda7fc`.

The signed rebase onto `38d99096` retained every one of the 33 existing Q2 patches unchanged. New upstream includes GUI copy correction `7764eea618d22b1f862790b34b0e69d2e8aceae8`, the compact-heading change `35b4e0a29b109974e3034691a9a8477455d97ef7`, and `38d99096`'s `-lncheck final`. Rebase result: `R/38d-rebase-result.json`; log SHA256 `5fe4ce2c56261433078ad8830e28cedc86f84c29343e0565a74261e4d7b0ca82`. Range-diff has no changed patches: `R/38d-rebase-range-diff.log`, SHA256 `1d7942854fd373b61a0d65c437357760c548af1f39f849457c66332b513638a0`. The pre-rebase signed head remains on `checkpoint/dream-read-q2-before-38d-rebase-01M6RZZXSHZT3KHBK1QC7K3XB3`.

Five new observer summaries from the earlier successful MCP gate were preserved by same-filesystem rename with unchanged inode/device/length/hash before rebasing. Manifest: `R/retained-observer-summaries-before-38d-rebase/preservation.json`, SHA256 `94605ea5fc3abff0d70285a5c489cf6787763f620f4b9c8f1f1fb1e85ce8ca21`. Earlier evidence and checkpoint branches remain.

A deterministic landing issue was then resolved within the task's explicit `scripts/land.py` consume/preserve scope:

- `tests/dream_sinks.rs:649–682` writes five observer summaries into compile-time `CARGO_MANIFEST_DIR/.bboxy/runs`; private HOME does not redirect them.
- A successful canonical Dream MCP gate therefore creates five untracked files before landing's mandatory whole-worktree clean check.
- `scripts/land.py:53` now retains only those exact five completed cases after the canonical `make test-dream-mcp` returns0. It checks the successful command row, unchanged tracked source, canonical filenames, one instance per case, bounded nofollow reads, regular/euid/single-link identity and expected observed verdicts. It renames them into a fresh private TARGET directory, verifies original inodes/device/bytes/hashes and records the gate/source-bound manifest in the landing receipt.
- Any unrelated output, incomplete case set, failed gate, unsafe file or changed source still refuses. The existing source digest, every verification gate and final entire-worktree clean check remain unchanged. No output is discarded or ignored.

Commit `238342d6` contains only this preservation fix. Final `scripts/land.py` is 13,297 bytes, SHA256 `36bd8eb4b0d6bcba5efd46ee94d5d0f035bb3c5ddb5c12aa02efb8dc0a859354`.

The focused control command was:

```sh
python3 -I target/dream-read-q2-restart-01M6RZZXSHZT3KHBK1QC7K3XB3/landing-observer-preservation-controls.py
```

Actual exit0: one positive preservation witness plus 13 deliberate refusals (unexpected file, missing case, duplicate case, symlink, hardlink, malformed JSON, oversized input, wrong case, integer verdict, failed/missing gate, tracked change and post-rename tamper). The positive witness checks unchanged inodes/hashes and real Git cleanliness in a private clone of the existing signed commit. The synthetic command row is explicitly labelled source-known control; no actual product gate was executed by this unit script. Test script SHA256 `29a405c3b6da644af6cd745fab2e45ba0eca33bebdad12485f1372a3b1f2ad12`. Log `R/landing-observer-preservation-controls-nofollow-open.log`, SHA256 `af50f9f094f22306c7e20131b87c41edd4273851d3e41573f1ab10744205076f`.

The first control run failed exit1 on a real implementation mistake: `Path.open` does not accept `opener`. Both calls were corrected to built-in `open` before the green run. That red log remains `R/landing-observer-preservation-controls.log`, SHA256 `47c3103c498cb51a4460c00968745b949122de174f6cef2fd753bbae55cd36f9`. Syntax and `git diff --check` also exited0. Controls executed the final script bytes before the signed commit; the script bytes did not change at commit.

The Oracle's new ruling resolved the earlier scope question: **FORMAL_JOBS is declined and remains unapplied**. No parallelism, timeout, model, graph, verdict, tool or coverage gate was changed. Current `scripts/formal.py` SHA256 is `4b87e5b4f6388d8b1dbfe0ce5cf7f778462c6d05c589c5fb2d57d162d6a7cd79`; only the authorized upstream `-lncheck final` fix was integrated.

The same message independently verified and **froze** C tuple `35d3e602/7a753654/86c5c9b5/d66feb40` for the builder. The delayed messages about old `30f27ccc/76d579c2` and old quarantine source refer to superseded artifacts. No C source or preparation was changed or repeated.

Current retained input, prepared only:

| Item | Identity |
|---|---|
| Signed preparation source | `35d3e60288d394ff0f0d9e2e6abb521d3d60b007` |
| C source | 331,208 bytes; `7a7536546a8cae7d088a3a372d1baeb26f46a3990e2524a792624f59ae12030e` |
| Immutable executable | 269,176 bytes; `86c5c9b5705f117cefda90f0d0bcd892813c631cf7aa9de573c19607be5a8858` |
| Preparation receipt | 514,287 bytes; `d66feb400b89b47aac0e55fc2d2ada994362bde770a398108fe02f1fb1d318c6` |

Directory: `/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/final-signed-verdict-preparation/dream-read-c-preparation/compile-2ezbx4j3`. The preparation roots and compile directory are0700, source/receipt0400, executable0500 with single links. Their bytes were rechecked on this turn. The prior full352-output audit, lexical/resolved pthread header closure and compiler/schema/source-only boundaries remain in the superseded report. No real candidate was executed; qualification and `runtime_approved` remain false.

The required full command remains:

```sh
make test-dream-c1 && make test-dream-mcp && make test-formal-parity && make check-embedded-build
```

It has **not been rerun at current head**. Last actual full command was source `3ed45c8`, exit2: C1/MCP0, native448/5,841,767 states, full reference2 at a60s graph timeout; separate old embedded2. Those facts remain historical and are not current-source green evidence. The new focused preservation control's exit0 is not the task verification result.

Oracle message1282 requires waiting until both harness `-lncheck final` and term background-QoS landings are present before one full-reference rerun. The harness requirement is now present. Term remote `main`, observed by `git ls-remote` before and after this independent source work, is still `da555c380d37b44a8bc6802d0b12b1f023e6eac9`. An isolated scratch fetch verified its commit signature (exit0): `scripts/qos.sh` is absent and `scripts/cache-env.sh` clears compiler wrappers and selects sccache without a background-QoS wrapper. No other worktree was fetched or changed.

Dependency evidence: `R/term-qos-dependency-summary.json`, SHA256 `7facd1fc41f944eaa44a2499bb5de2dde78f201452d5fb19d33b1464003edeac`, with retained exact source/commit/tree snapshots under `R/term-qos-dependency-evidence/`.

The exact decision needed is a landed QoS revision satisfying message1282, or an explicit change to that ordering allowing the one post-`-lncheck final` reference attempt before term QoS lands. Until then no reference retry, `make land`, push, new qualification claim or task completion is asserted. When the prerequisite is present, the remaining authorized sequence is the unchanged task verification, report any repeated model timeout with state/edge counts, then canonical `make land` and remote containment proof.