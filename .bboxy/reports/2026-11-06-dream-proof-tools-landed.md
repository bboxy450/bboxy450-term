---
type: report
title: 'Dream proof tools: final frozen component refused; not landed'
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6S9GNN1HB7PW20DSH1SY8BG
created: 2026-11-07T03:35:01.336Z
commits:
- d02cdd9fe3406d213699ec82473ef6d9309528b2
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
supersedes: '[[reports/2026-11-05-dream-proof-tools-provisioner]]'
id: 01M6T15SP66GWFXN5C1GY8MZAT
updated: 2026-11-07T03:35:01.336Z
number: 93377961366217
created_by: seat:01M6S9GNN1HB7PW20DSH1SY8BG
updated_by: seat:01M6S9GNN1HB7PW20DSH1SY8BG
authorship:
  signer: seat:01M6S9GNN1HB7PW20DSH1SY8BG
  kind: seat
  public_key: j9rmEML4pGBlMmdgzCzyAhWJXtoIFbtcaz3XVJoJunY=
  process_generation: '1'
provenance:
  digest: sha256:3928db21ec8fa97f90432f8a9ef61254b3c2295b6c43636ef49e00fe1691f913
  frontmatter_digest: sha256:c83bb95ca59f725da940244b2b2353f7deacb0ea9f3f9f22a362b7fb394973e6
  signer: seat:01M6S9GNN1HB7PW20DSH1SY8BG
  sig: Z5iNTTDbwYUX1EfU/1P+tTUnGysmWk9xwe5GKkcyvPCC/UrrUmRcY1RstkiNIvhal3EPJlPSlLww7xgxm9CZBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150154
  event:
    id: 01M6T15V29BHCJG567Z83M8QBP
    sequence: 150154
    hash: 0046e88ad45397c8693d71a29ced1d3b2e18184f779bf0f4a4d7640915d4d2fb
---
The final frozen input was ingested and the scoped implementation is signed, but the task is BLOCKED on a real component refusal. The exact verification command exited 2. No successful qualification, candidate receipt, landing or runtime approval is claimed. The allowed filename ends in “landed”; this report explicitly records that landing did not occur.

Oracle decision needed: disposition of the retained resource/settlement refusal under the unchanged law. Further native work needs a causally changed, authorized Q2 input or a revised assignment. An unchanged-input timing retry is prohibited. The Q2 controller/source is outside this Executor's writing scope.

## Source and custody

Worktree: /Users/feral/mydev/bboxy450-harness-dream-proof-tools, branch feat/dream-proof-tools. Current signed source is d02cdd9fe3406d213699ec82473ef6d9309528b2, rebased with --gpg-sign onto origin/main 38d99096. All 18 branch commits report G signatures from the approved Boxy key 9B07E849480C68328CA20684844ECC0587DE7AB0. The final worktree is clean. Predecessor fac9257a189efdbf5cf4032c13d5878354ef9f28 and original failed effects remain preserved; no reset, stash or target cleanup occurred.

This report supersedes the initial partial report [[reports/2026-11-05-dream-proof-tools-provisioner]] as the current handoff, while preserving its historical claims and failures. Code commits carry Report: [[reports/2026-11-06-dream-proof-tools-landed]].

The task's FINAL RE-FREEZE was consumed exactly:

| Input | Exact binding |
|---|---|
| Signed source and preparation commit | 35d3e60288d394ff0f0d9e2e6abb521d3d60b007 |
| C source, 331208 bytes | 7a7536546a8cae7d088a3a372d1baeb26f46a3990e2524a792624f59ae12030e |
| Supplied executable, 269176 bytes | 86c5c9b5705f117cefda90f0d0bcd892813c631cf7aa9de573c19607be5a8858 |
| Preparation receipt, 514287 bytes | d66feb400b89b47aac0e55fc2d2ada994362bde770a398108fe02f1fb1d318c6 |

All reside under /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-read-c-audit/final-signed-verdict-preparation/dream-read-c-preparation/compile-2ezbx4j3/. git verify-commit and 2ce4359d ancestry checks both exited 0; signed C bytes were independently hashed. The adapter verified private 0700 roots, exact 0400/0500 single-link identities, source/preparation snapshots, compiler/build/header/SDK/platform bindings, 323 lexical header entries including aliases and symlink targets, all 352 retained outputs, stable linker inputs and actual Mach-O facts. Offline ingestion was not treated as execution authority. Old tuples are retired, never fallbacks. The supplied executable was not rebuilt or substituted.

Evidence directory D = target/dream-proof-tools/final-freeze-20261106:
- signed-source.json SHA c4ab11ca270145ebada007f424e5803724fb5dbd20afbc4b5d435b0aa5d12547.
- offline-ingestion.json SHA e6de4f797084f22a7b6b0d1549c30a32db0efd6eeb3b2ddb2e0b61205c9d1d9f.
- handoff.json, 39221 bytes, SHA ef3b9294557f3d6a99f1b9f3a19604a3b901465e66dfc9c43188ad29ae1efbc8: exact raw-file hashes/identities, all 60 command exits, source/pin/tool/budget/result/ledger/terminal/verification bindings.

## Implementation and acceptance evidence

All source pointers below refer to current signed HEAD.

| Requirement | Mechanism and observed evidence |
|---|---|
| One recipe; exact authenticated seven sources | Makefile:10 and scripts/dream-proof-tools.py:339,388,436,513: fixed archive/key/signature pins, actual bootstrap verifier SHA, fresh public-only homes and native signature status, bounded extraction and exact patched-source custody. All seven authenticated before extraction/build. No Homebrew binary library substitution. |
| Real guarded pair, config/link/source review | Script:610,653,728,2072: exact source/config/recipe/tool/platform binding, actual argv/PID/exit/log retention and typed Basis with 7 authentication/generated rows, 16 command forms and 8 finite review findings. ATTRIBUTION.md and fixture license-map preserve upstream terms. Guards and finite review remain conditional source/compiler/platform premises. |
| Exact supplied canonical input | Script:51,2188,2262,2312,2354,2528: literal selection, no-follow custody, complete lexical header and retained-output closure, exact projection hashes, final rereads. sources.lock.json binds the new C source; no probe discovery or recompilation. |
| One Basis/six pins; existing caps | Script:2072,2736,2881: actual canonical Basis and six identical pins; actual descriptor charged before launch. Materialized Basis 47112, lock 7176, descriptor 3439 bytes; with full 65536-byte result reserve, 123263 <= 131072. Closure projection 6442 bytes. Individual limits were unchanged. Final qualification/envelope was not emitted after failure. |
| Original effect, quarantine and actual exit | Script:634,644,2906,3102: unresolved effects fence replacement; original Popen handle remains owned through actual wait4, no result-wire-as-exit, timeout kill or replacement. Current controller actually reaped exit81; failed prefix/incarnation/records retained. Same-input check refused without another native launch. |
| Exit81 cannot mask persistence/terminal errors | Script:2989 and ComponentHandoffTests: separate control/persistence/terminal verdicts, paid tail before final receipt, complete ordered error histograms/diagnostics, successful result persistence, debt and final helper settlement. 56 offline tests passed, including missing/reordered/truncated/extra-error and masking controls. Actual current positive failed before acceptance; no real expected-control success is asserted. |
| Raw reports; receipt last | Script:3130,3139,2679,2775,2846,3147: raw stdout/stderr/wait4/ledger retained before checking, persisted result equals original stdout, inventory/currentness and actual aggregate charging precede no-replace qualification and final candidate receipt. Actual failure produced neither qualification nor receipt. Synthetic publication/custody tests do not substitute for a successful joined run. |
| Formal models and broken controls | tests/formal/dream_proof_tools.rs; verification/{corpus,contracts,coverage}.toml: 9 proof-tool families, 64 broken controls (73 models). New ComponentHost controls distinguish wire/exit, replacement, terminal masking and persistence masking. Current complete native/TLC state, edge and verdict parity passed. |
| Attribution and unattended checks | make test-licenses exited0: headers/policy plus 10 negative controls. All work used scripted private sources/homes/outputs; no operator, Keychain or GUI input. No install, release, push, merge, product worker or Q2 source edit occurred. |

The inherited direct-entry source-mutant witness at target/dream-proof-tools/runs/01M6Q7WSMDGKK0BS2QA4D2497R/source-custody.json compiled actual Git main variants containing fork/pthread_create/connect and rejected their changed SOURCE custody. Those objects were never linked or executed. This proves the retained source rejection/entry-call witness, not runtime observation of hidden effects or an arbitrary-native lifetime theorem.

## Actual final build and native failure

Build B = target/dream-proof-tools/runs/01M6T0EZ07D77BYZK7YNHP2A32.
Candidate P = target/dream-proof-tools/candidates/01M6T0EZ07D77BYZK7YNHP2A32.
All build commands 000–059 exited0, including actual Git and GnuPG compilation. Current step042 is GnuPG configure --help; the inherited out-of-profile candidate --version call was removed.

Real private tool bytes:
- P/bin/git: 3766096 bytes, SHA 65725aa08f6006b8683133cc0d92a47fe0aa6a772fa36c448006f38d5c4c467e.
- P/bin/gpg: 2405760 bytes, SHA 06edcc28d05fa7b41ee1cf5e20dc08c3e815db425ba03459419494081a04d852.
Both are owned mode0700; candidate root0700. Compiler SHA1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9, SDK27.0 settings SHA7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb; Darwin27.0.0/arm64→aarch64/build26A434, shared cache c284a291b68b3d0da1a03f866f87efdc. Compiler/platform/native reachability remain explicit assumptions.

P/provenance/pretest-basis.json SHA0b266a18d2c3c52ea2a89d89ea323c4a6de2f771c4275538e39c89e3fe01f1f8; source lock SHA33f4595e28af37f48c3ce476a21c6032d2f1d14ab8dff902f0fdf6feb5fa0e0d. Descriptor B/components/positive/pretest-input.json SHA14de93b1a22b405ec4239526fb9f9ee5a686977870dc3e625ee050c20b36c9b9. Proposal-source snapshot digest fbc2c762fb99e3537bb146056b35241bc7e581899bdc2086d97857d05afb52c8 is a frozen file-byte snapshot, not compiled product currentness or approval.

Only positive was launched. Spawn record bounds around Popen are 2026-11-07T03:25:06.115113100Z–03:25:06.118928775Z; exact native start timestamp was not captured. Controller90677 actual wait4 exit81; helper92222 created/reaped1, group_gone=true, retired_coverage=true. Proof was not reached; first form13/show_keys, grant16 failed. Raw ledger line18:
Q = max(W457000, A93542) + C569291 = 1026291 ns > 1000000 ns.
The roster stage reports 402542ns CPU. This is an observed violation under the fixed law; no C-defect or load-cause diagnosis is inferred from this run.

Raw first error is resource_limit/current member-window and controller lower bound exceeds one millisecond. Complete terminal diagnostics retain three resource_limit calls and one settlement_unknown/final unreaped group kill unresolved, including an additional ordinary-engine-unit overrun. Subsequent actual settlement is retained; it does not erase those failures. Final raw receipt independently says control_verdict=failed, persistence_verdict=persisted, terminal_cost_verdict=passed, errors_complete=true, diagnostics_complete=true; final debt charged/paid18387246375ns, retained units/overflow0. Parent wait4 user34367000ns/system116376000ns/maxRSS2326528bytes. No expected-control case was run after this failure.

Unchanged stdout equals output/result.json:
- stdout 2043 bytes SHA43943cbde45d57f20d573efda6c152952c1993d8e5416da4bd99a5e48f723ee1.
- stderr 1881 bytes SHA269355e1371d9dfc9c8ca48c64d68c6cc5063329945b6a45cb2f58e8935112ea.
- output/ledger.ndjson 26615 bytes SHA413574d03deaf982b431a7b972b90cf9fb0745a133d55d3160a1d2abb110e01a.
- wait4.json SHAe7673a1d579de63295cb6714d784eca458c53b0b5e8937e1f6166526a8facbaa.

All effects remain retained. No lucky retry, threshold waiver, corrupt-proof/control launch, qualification, receipt or landing followed.

## Actual gates

Exact task command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses.
It ran 2026-11-07T03:22:29.031922Z–03:25:28.097780Z, exited2: fetch0, build make2/Python1 following actual component81. Later && commands were skipped, then independent checks ran serially.

| Separate check | Actual exit |
|---|---|
| python3 -B -I scripts/dream-proof-tools.py unit | 0, 56 tests |
| make test-dream-proof-tools | 2, 56 tests pass; retained original failed build refuses, no new native launch |
| make test-formal-parity | 0, 354 models/87 contracts/one crate; complete graph/verdict parity |
| make check-formal-evidence | 0, source d02cdd9f, dirty=false |
| make test-licenses | 0, headers/policy and10 negative controls |
| cargo test --offline --locked --jobs 2 --test dream_proof_tools -- --nocapture | 0, Rust process witness invokes same56 offline tests |
| git diff --check | 0 |

Rust compilation used /usr/bin/nice and two jobs; tests/builds were serial. Disk remained above8GiB (about40GiB available); no peer cleanup occurred.

D/verify.log SHA52ad163ef1b57d12319977dedaad53ff1d6d991644e4e82dea0065886f5f116d.
D/independent-gates.json SHA21f2cd507978d596742c03c7b45da37f0aaea3ad71e9610bfdcdd98020005e73 carries actual commands/times/exits/log hashes; formal log SHA2d51bc7227789d154de5392e5a68268fd2cd01066f375cce6d200429c55a8ab1.
Current formal evidence: verification/evidence/cc3b9cfc7a2c2b8e5eea831dcc77c76ed30118a5c7bcc06cf040c0d65176c65e/evidence.json, matching SHA, source d02cdd9fe3406d213699ec82473ef6d9309528b2, dirty=false. Pinned Java21.0.13+9/TLC1.7.4. ComponentHost good321states/1631edges; controls WireIsExit510/2639, ReplaceUnresolved641/3405, ControlMasksTerminal323/1640, ControlMasksPersistence323/1640; expected safety failures matched. Thirteen variables have a110592-state Cartesian envelope; only local release/accept/seal bookkeeping is weakly fair. IO, actual exits, settlement, debt/verdict arrival and scheduling have no fairness. Finite parity is not arbitrary Rust/C/compiler/OS correctness.

## Preserved deviations and corrected historical interpretation

The following append-only JSON addenda under target/dream-proof-tools/corrected-posix-clock-20261106 remain binding report evidence.

1. PROTOCOL DEVIATION: inherited, side-effect-free candidate git --version, exit0, step042 of run01M6SF8AM69KRBKF8RZR5XHQNR. Exact executable /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6SF8AM69KRBKF8RZR5XHQNR/bin/git, SHA3acb3fdfd805540d47e8762d0036a3e60283c1958d58acdbb34098f912d556f4. Record bounds 2026-11-06T22:25:37.097713440Z–22:25:37.251537960Z; exact native start/end were not captured. Removed by signed10bb63510bcd1d8ee3d4e2b9fd5aff5d8e1625f0 (rebased154b832d841706ee371503c26c3faff146cff32b). Effects retained and excluded from qualification of ANY kind. report-addendum-protocol-deviation-step042.json SHA314d9b21330d93c6e8640ae2f614d3b809c43ff10aa55f2973d13c745236569c.

2. Run01M6SJ1GZKFB1C2KJ8CJ5PSBP0 diagnostic launch record bounds 2026-11-06T23:14:28.033742070Z–23:14:28.037275859Z occurred after Oracle no-launch instruction01M6SFJNHDW8XEH3HQVV1MQHXZ was issued22:27:27Z/stored22:27:27.406Z. Native-queued/storage is NOT verified model receipt; conversation order showed that instruction after the completed diagnostic. No subsequent launch followed its visible receipt until the FINAL RE-FREEZE. Oracle classifies the old run as positive-component diagnostic evidence ONLY, never qualification. report-addendum-launch-against-stop-instruction.json SHA40d241d188631473fd12eda6ec24002a18028e45f510124a9b7e4987e9c41b15.

3. Oracle retracted the old run's alleged “member-window C defect”: Q8785417ns was WALL late STOP ACK under load, with tiny CPU. Ledger43 W8381000/A6667/C404417, STOP-to-ACK8281000; refusal was correct. Earlier defect language is superseded interpretation only. report-addendum-correct-load-time-refusal.json SHA7caacc67f92fd7ec9a4613e356b951c75995f269a480e70ef061e6833e986830. Old raw evidence unchanged. Its handoff.json SHA94c56d2a490a13f6f02e846e46d60b1990cfa3384daf961c1b47cf824b68083b also preserves source-currentness refusal, conservative reservation failure and Mach/POSIX clock mismatch. The latter two host issues were corrected without changing caps or the30s law. Old formal/gates belong to2e9231ce and are not current qualification.

The CPU pause was observed; heavy work resumed only after Oracle lifted it and supplied the final re-freeze.

## Remaining limits

Real positive/corrupt proof/all causal controls have not passed, so full task acceptance and landing are unmet. Source review, compiler/SDK/platform assumptions, finite models, metadata ingestion and offline controls cannot substitute for the missing joined witness. Qualification/receipt-last success, successful cached-candidate reuse and Root admission are not established by this failed actual run. Q2 separately owns current runtime admission, qualifying lesson/publication, four-job/full exporter/foreground/control/heap/debt/reap product gates. No product availability or runtime authority was created. Installed tools remain unproven for Dream runtime.
