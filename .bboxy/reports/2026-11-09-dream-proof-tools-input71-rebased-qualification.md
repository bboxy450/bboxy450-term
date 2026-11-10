---
type: report
title: INPUT71 rebased final-gates continuation and conditional qualification
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T21:08:35.020Z
commits:
- fe137b8801ef3eb2a07710367422a0da2713f519
verify:
  command: python3 -I target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py finish
  exit_code: 0
id: 01M7128BYSN9XGZ3V8QNWW7HH2
updated: 2026-11-09T21:08:35.020Z
number: 205483610615676
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:a2aae26dfebd8a92d853181e6ee63079c659c2d9da559808fe84a40d0dbc614a
  frontmatter_digest: sha256:c3894b280441e37c558635725f7b08a33fd331831dd8de2ed15c38ff68c924a1
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: 50f6/8HZIYxRrWB79IGWRdh0lr3qPpBOQGRZCNBLsihupO3SySWIP+ewSnLlEe1lOH8UBdMkb0IA+peFG83mDA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151183
  event:
    id: 01M7128D7V8NZDCJ7VB04JHDPH
    sequence: 151183
    hash: 91198a04f5d233e3c5490269e95ba9cd58ed1948fd84d1ec3b775bfb9629b785
---
The scoped second final-gates-only continuation completed with every remaining gate at exit 0. The retained INPUT71 candidate is review_ready; qualification and receipt were published without replacement. runtime_approved=false, conditional_on_bound_source_review, whole_command_lifetime_proof=false. No native rerun occurred.

This continues [[reports/2026-11-09-dream-proof-tools-input71-clock-correction]] and [[reports/2026-11-09-dream-proof-tools-input71-final-gates-continuation]]. Their original failures remain historical facts.

## Evidence locations

- Worktree: /Users/feral/mydev/bboxy450-harness-dream-proof-tools
- R: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M70XWHWKFCD1W6ZZMQB50WG1
- P: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70XWHWKFCD1W6ZZMQB50WG1
- N: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input71-formal-rebase-20261109
- E: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/input71-clock-correction-20261109
- Final handoff: N/final-handoff-evidence.json, 8964 B, SHA256 8cf1796c6400e1c423bd117824e62d476bfed2c26d61846a5b981f9b8e745f8a.
- Append-only completion: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M70XWHWKFCD1W6ZZMQB50WG1/final-gates-continuation-01M710HG4BP3TMGXY3BJH571MZ.json, 19413 B, SHA256 44d2565dd55863ff470cbe1d04c7e4c37c3630de43fa9b21013c07fcae586001.

## Rebase and ownership

Rebased the 42 builder commits from old harness base 38d99096f4f9c475a5547cf2192982af6ff7a5c0 onto exact harness origin/main ad6c8b72fe0bb766164c879acd27c1ece2e565a7. Current HEAD is fe137b8801ef3eb2a07710367422a0da2713f519. Every replayed commit was re-signed with Boxy's key 9B07E849480C68328CA20684844ECC0587DE7AB0; all 42 verify-commit results are 0 in the proof. The old c50 head remains at archive/dream-proof-tools-before-formal-rebase-20261109. No reset or stash was used.

N/rebase-identity-proof.json is 21583 B, SHA256 66e0ba90a9843b976b761432078458cac20183b374a061e69451c4dfad35dd4a. It records actual byte comparisons, membership/mode comparisons, all signatures and the complete mapping below. All 38 dedicated builder files are byte-identical to c50efb4e155b919f1d5a34620465ce14be725bd6: scripts/dream-proof-tools.py, tests/fixtures/dream-proof-tools/**, tests/dream_proof_tools.rs and tests/formal/dream_proof_tools.rs. Recipe SHA256 remains a8187a0ca0b093bdf4e82c2f3e172b664f60a4e5fadc2cb0d50256b8c5d62edd, equal to R/recipe.py.

Shared integration files retain upstream additions. tests/formal/models.rs is not claimed whole-file identical to c50: removing exactly the builder's three module/corpus registration lines reproduces ad6c8b72's file byte-for-byte. Its dedicated builder model module is one of the 38 identical files. The corpus retains all 447 main records plus the identical 73 builder records (520 total); contracts retain all 96 main records plus identical 9 builder records (105 total). Main's coverage scope is preserved verbatim and its model IDs retained. Conflict resolution touched only allowed integration paths, with individual records under N/conflict-resolution-first.json and N/rebase-step-*.json.

verification/upstream/**, verification/upstream-sources.json, scripts/formal.py and scripts/test-formal.py are taken as-is from ad6c8b72; git diff --exit-code against that commit for those paths exited 0. No manual tool synchronization occurred. Main's declared formal.py 5097490d and test-formal.py 11c3d433 pins resolve the earlier SOURCE mismatch.

Old -> new signed commits:

```text
f2c9f5c7ea25016973b6887afab230f180bb1c36 -> 057ade361e4fc5c49f4c69f19b7e24b5ec9eb076
45886cf5869cf1f5f34e5397f3e34398ea1fe1a6 -> 8672a507b5be6ee5b520c3c4cd98bda1628e1846
e18c73da3600bd39eaef3a36ac29d3587ae3df1d -> 27156e38cf69e1593dabf800c4d80ff61ea13905
a167dd02f485a4afd7416997d0aed041bd24b74a -> 56c96947302c8a67932148d959eac0755f74f23f
caf0972d869ba1aee7b41c4d619cadf6b8a09899 -> 8d3464d8c3b4b23088f311124ef99ccbbfd10d2e
f5f34b6634dacd7dcb93fffc646231b05a02b9fe -> 6d9731e2bf910c0f42eeaac5c7d9222d48f80aad
69d980a1925ef6c24946bd3ddf3c4fdaebf35876 -> 62abfe66dda9c714c5377e17b4fc86b0787d12db
9e07d68f1718b12f0ddf42ff099b570809e54680 -> 59c2cdf52c4f2e4df4965598d83e5c7601f38c73
d74b179259959d3f0c25eaf18f856f1d8b0a927f -> a3eb94a827f1940aa6eb4330197ff81e908fdc3b
e9b3059021b52273efbb4593cd7f32efc62e0e47 -> 8a233c5864e1bcf068de3b61d47bca0318458b8f
e2cd762b565887f26f3b1157a45071154570334b -> 3065c5a3370e178c961f4e7273bf242688d2e5c8
cb3015d54fc56f62556aa0b29b0fd26fba40162d -> a6b99d23a8836f1d4b4cb194a997cef6cb9ba9ba
4e1e4017a24135eb5f3a38cbfdeb14d4fbff29a7 -> cdeb53c85cd415d17aca71b7f165bef95952d7d7
154b832d841706ee371503c26c3faff146cff32b -> 488b05f410818ac4349115a874bac2ca7365a197
7e7189d5b2692734e72bf60bd12c6ecf9de04ff5 -> 1fcd591cd54b7442b246e0b0c4d80f5e461950b8
a646e09d5a5f51340037524d3b383511a57a3688 -> eaeac83a19e0e9e1805fe154ed10b260407ae9a5
c6354cffb96e0b713cb3de6bfcd79223f5fbfb5d -> e8f91aa3a59f89732c7984e3b6b02dde2689a065
d02cdd9fe3406d213699ec82473ef6d9309528b2 -> 6336fe3df79fa64a8e5742e168271240c5457b79
a28283160d146f4c215181e466daae00e95bdb4c -> dd59ed8c44494b81d4451d5a3dd773549a0fbcaa
ff2e650b694f33908dd34c18415cf0aee9041e41 -> 31b6cff54e48fb3273a625326dba06d7bc04ffe8
59a301e4c3a555e22e2ba6a9f7ca754b7ade3a5a -> bf7b1dd479c0f31f0d6d7bd6f6c61056e8a394f5
913224f577368c9bbc70c9afce57a91e099957c6 -> f594f58bd9ff597da36eb662bd9c25a5aae876b3
12d72305bf74abbadd46ae27abb91b508f40cd89 -> d5530d884e577d907b7f9a0b3de81d815fa3b237
5b8a11083f7d0edfe7ff5f3187578fd8c09af6d1 -> d8e4f6f96fd08ac3a4de634233330193d68b9aa1
5c717eacf0d632897436923899c48bf1d39c4a2a -> 94f0b56606e87662f6d4634e4885b12c145e64f8
a42a789ecef84b2778463de290274aa15596ad24 -> f6aece194961d85d9d2052d4e57ca0cef62d8cce
9081e326c3703bd73c36e3eb32a4085c2edd2458 -> 5e5e2677d1c82655e7c38709e7f3593b622607bd
f6cdc7c4769e82dd0999cdf9e40caf37e2f1a28e -> ddfaf21e105f6a0d01769b389d23d3a5eadf08eb
68f6d062d83af635dfc87e95beaf834b2c9766ca -> 332cb26a6fdc1bb0ffd17a895d9f0f7ad6012393
1691e3947ca04739f1301b596eae388f9c6f2aef -> 2056a2f2e07e5caa84363f3bdc142aeb9a20aaf3
0068adc9ed2597153f85ba1a8aa485c835f6054e -> c88f722730e473ab0f25add28696bed96b8f29f2
9ff0f6acbe32c7dfb0b06dcb870783defc8489f7 -> 1d8ad151e5aa91b39d30d395c9568e780481cc45
4332bd5327906bac33cd94c4e2e17dad904d869e -> ed8f7d04ecd650a2d7cd2409054c004da50abf13
c5230f53ddb2be8e620ac40a5690eef529d3c214 -> ebaafc008517b868cd595e14eee23c232d31d19e
41c3982df58218ea4c4754febb653183ca512d63 -> 4980b18370ce2577ba426ce069063e6b72b10b43
db0b6000003dac6402a6da6a4fb2477eab1dd7d2 -> bf5c5a90fe576b9e70a8dbad5b9ba63ceab2b678
6e9031570f9f1c990a8a76284497b41924f3f27d -> 07a493ddfec74bb9d43a4e95afeca0f059926fc6
eb42d39aafb86d9de7ded7ec30d9bc00a13297cf -> 57da4c5b2e25a71b8361240564f8a918289528ad
33c7a896bedd80968a6a1dafcccb64eec0fbe605 -> 36533a4000e8e56c7e1d41791a8e57fddec14967
99bda9ca5592ae8e4ce2d4098c80a506a6c5364c -> 3b9dd6e2a7b67e7df8ee0ff343e4789ad9c0f8d6
a7bb5535010affe9ddebc3b4407b794ef060ca26 -> a9284dae4ee8a9d9059c187a0c73a18c6d13c221
c50efb4e155b919f1d5a34620465ce14be725bd6 -> fe137b8801ef3eb2a07710367422a0da2713f519
```

## Actual verification and continuation mechanism

Commands were executed separately, each with actual exit 0:

```sh
python3 -I target/dream-proof-tools/input71-formal-rebase-20261109/rebase-continuation.py gates
python3 -I target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py publish
python3 -I target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py finish
```

| Step | Actual result |
| --- | --- |
| R/logs/062: /usr/bin/make test-formal-parity test-licenses TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target | 0 |
| Unchanged recipe.check through the recorded origin lookup bridge | 0; all 79 unit tests passed; retained component validation passed |
| R/logs/063: /usr/bin/make test-formal-parity TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target | 0 |
| R/logs/064: /usr/bin/make test-licenses TARGET_DIR=/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target | 0 |
| Final source status, diff --check, HEAD signature, main ancestry and unchanged upstream-tool checks | 0; clean worktree |

062 and 063 each passed formal_models (26 tests), formal_nonvacuity (55), reactor_projection (1), 12 formal-tool tests, and full native/TLC graph and expected-verdict agreement for all 520 models and 105 contracts, with one crate inventoried. Source is the clean new HEAD fe137b8801ef3eb2a07710367422a0da2713f519. Retained formal evidence is verification/evidence/92313b580f77dd3fa1b2fb897210ce6dd28abc45600781acffd407fb7814d87f/evidence.json, SHA256 92313b580f77dd3fa1b2fb897210ce6dd28abc45600781acffd407fb7814d87f. Header/license checks and all 10 license tests passed. These are bounded model verdicts, with complete_tlc_replacement=false retained.

Every continued command uses the original 060 environment with exactly RUSTC_WRAPPER='' added. 062 repeats the original argv/cwd. The completion binds all argv/environment records and real exits. No other environment or production recipe change was made.

Production cache lookup requires exact commit equality (scripts/dream-proof-tools.py:655, :705 and :3054). The literal new-HEAD make test-dream-proof-tools target is not claimed to have passed. Under the ruling's explicit byte-identity currentness binding, the private bridge called unchanged recipe.check(), recorded actual fe137 inputs, required every non-commit input to equal the original immutable intent, and supplied the explicit c50 origin lookup key only for retained-candidate lookup. Qualification, Basis, original intent and descriptors keep truthful c50 origin provenance. N/read-check-currentness-bridge.json (8446 B, SHA256 3a046f13d09378919bd1929b5e649a52fe26a5072b20c123839b3a7fc564b6e1) records both complete input tuples, the mapping and this limitation. N/read-check.stderr records 79 tests in 4.244 s, OK.

Publication used the unchanged qualify_candidate constructor (scripts/dream-proof-tools.py:3456) with narrowly recorded hooks: return the eight revalidated original native records exactly once; return the actual completed 062 output exactly once; capture only the new sealed-terminal payload outside the prefix exactly once. Every other original validator and exclusive write remained active. The publication driver is hash-bound in N/publication-start.json and the final completion: 82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d. The helper/recipe pins were checked before loading code, and finish required identical publication-driver bytes. N/captured-sealed-payload.json retains the original validated completion payload while the old unsupported R/terminal.json remains unchanged. No native callback or fallback was allowed.

## Retained eight native checks

The original run 01M70XWHWKFCD1W6ZZMQB50WG1 supplied all eight checks. Hashes, device/inode custody, prefix/fixture incarnations, descriptor argv/source/preparation bindings, compiler/SDK/platform/shared-cache/shim/tool currentness, raw stdout/result equality, actual original wait4 records and original terminal semantics were revalidated before gates, before publication and after final gates. Original artifact hashes agree with E/result-evidence.json SHA256 17225997801ea0923fa9c6ee4243db75d254ad5e1f86bc2104760e4abb702e19. Native reruns: 0.

Positive is valid; corrupt_proof is the expected invalid proof; pause_resume is valid with its specified actual exit 81. Each has all 28 final-conjunction terms true and first_failing_term=none. The five negative/control cases retain their failed/quarantined raw outcomes and expected exit 81 while their host checks are 0. No conjunction row is invented for those controls.

Margin below is the original admitted deadline minus reporting-start cutoff, not a claim about subsequent diagnostic completion. All exact admitted intervals and raw counters remain in E/result-evidence.json and the new handoff.

| Case | Actual native / host check exit | First failing term | Cutoff margin, s | ordinary_max_ns |
| --- | --- | --- | ---: | ---: |
| positive | 0 / 0 | none; 28/28 | 14.121944000 | 594084 |
| corrupt_proof | 0 / 0 | none; 28/28 | 14.547151000 | 472625 |
| pause_resume | 81 / 0 | none; 28/28 | 13.506766000 | 904667 |
| cancel | 81 / 0 | expected control; no conjunction row | 22.462447000 | 456333 |
| stale_incarnation | 81 / 0 | expected control; no conjunction row | 22.617067000 | 450459 |
| clock_failure | 81 / 0 | expected control; no conjunction row | 22.461756000 | 651209 |
| debt_overlap | 81 / 0 | expected control; no conjunction row | 22.601083000 | 719416 |
| reap_delay | 81 / 0 | expected control; no conjunction row | 22.579679000 | 443541 |

Debt split, seconds (full nanosecond values retained; unknown is a derived remainder, not measured raw CPU):

| Case | Helper debt | Ordinary raw debt | Ordinary floor debt | Unknown ordinary debt | Final paid debt |
| --- | ---: | ---: | ---: | ---: | ---: |
| positive | 2.104704000 | 1.802439756 | 9.942560244 | 0.000000000 | 13.849704000 |
| corrupt_proof | 2.367693000 | 1.509965244 | 9.659034756 | 0.000000000 | 13.536693000 |
| pause_resume | 2.209518000 | 1.974248244 | 10.185062625 | 0.000000000 | 14.368828869 |
| cancel | 0.104607000 | 0.474156378 | 5.924843622 | 0.000000000 | 6.503607000 |
| stale_incarnation | 0.009000000 | 0.471971250 | 5.909028750 | 0.000000000 | 6.390000000 |
| clock_failure | 0.009000000 | 0.661209372 | 5.818790628 | 0.027000000 | 6.516000000 |
| debt_overlap | 0.009000000 | 0.526485762 | 5.926514238 | 0.000000000 | 6.462000000 |
| reap_delay | 0.194823000 | 0.592205256 | 5.887794744 | 0.000000000 | 6.674823000 |

All final charged debt equals paid debt; no deadline or process_create excess was recorded. clock_failure retains ordinary_raw_complete=false and the derived 27 ms unknown ordinary remainder. Its three primary clock diagnostics remain in the frozen causal order; the exact auxiliary/resource/no-CONT/no-exec-receipt requirements remain enforced.

There are 50 successful shim receipts (16 each for positive/corrupt/pause, 1 cancel, 1 reap_delay); all in-process checks are true, priority 4, no shim refusal. Escape-check-unavailable rows are zero. ESRCH is intentionally silent under the frozen best-effort rule, so absence of a row is not proof that every live read succeeded or that arbitrary escape is impossible.

pause_resume retains 15 ownership rows: all pause_stop_errno=0, stopped_system_ns=0 and running_gap_max_ns=0. The three stopped intervals are 169926000, 169876000 and 150838000 ns. Six rows use terminal_observed_without_stop_attempt with pause_terminal_resolved=1; other rows use terminal_not_observed with resolved=0. Full stop/ACK/resume/owner fields remain in the original ledger and E/result-evidence.json; no missing owner row was fabricated.

reap_delay keeps its original quarantined partial wire snapshot (created/reaped 1/0, group_gone=false); its final terminal is physically settled 1/1 and its original parent's later actual wait4 proves exit 81. No raw status was rewritten. All eight controllers were actually reaped before this continuation; no teardown or signals were needed.

## Qualification and receipt

- /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70XWHWKFCD1W6ZZMQB50WG1/qualification.json: 39083 B; SHA256 83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335.
- /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70XWHWKFCD1W6ZZMQB50WG1/receipt.candidate.json: 969 B; SHA256 452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c.
- Basis 47490 B; SHA256 595d30106fbab7d947c296c161ffea5959a1b5c95dc84943d79c1e431ebbefb0.
- Prepared aggregate 99125 B, below 128 KiB; Basis is below the authorized 56 KiB individual ceiling.
- Qualification has 8 check exits of 0, 8 bound review findings, 35 log references, review_ready status, and exact original c50/a818 builder provenance.
- Git: 82a1e6f23c21011e59563c2d4e26323bb7a2cd7df55f12363a77453fc4437c5c; GPG: 998885382b7a2754e3a69c920e0ce3d386486454808883de59065faa3a9f3b91; frozen shim remains 4e95b2a83a34a6328d3c794133c3569dff7e5f1b2ee2c9bfc1d58cd08447527d at libexec/dream-exec-shim (0500).

Original schema, reference/inventory, source-binding and budget validation passed before publication; validate_candidate passed again afterward. Qualification was published without replacement, then receipt was the final prefix write. Their bytes/inodes remained unchanged through remaining gates.

Conditional source/compiler/platform assumptions remain explicit. The separate decision66459_exec_shim_limits evidence binds coalesced NOTE_FORK events and setsid-orphan limitations: event counts need not equal fork counts; an orphan using setsid can evade a post-leader census; wait4 is the CPU verdict and failed live reads are diagnostics. Census silence is not a whole-command lifetime proof. No runtime admission, installation into product authority, merge, push or release was performed.

## Preserved history and final state

060 remains actual exit 2 (sccache SUN_LEN failure); 061 remains actual exit 2 (old formal SOURCE pins); R/terminal.json remains its original unsupported record, SHA256 f7e3f02d67e2c2b09551908ac448e619c2fb6e58fbebbdd718ea999cb647cfde. The first final-gates continuation remains SHA256 4bc524861b9ff36b8f24e551858ddbecf2f49566ba18bf4d0e5f2a6c0cf58f83. Every byte/dev/inode pin for those records, all eight original native artifacts and all 33 earlier ledgers was rechecked unchanged after completion.

The original complete canonical shell invocation remains exit 2 as recorded in E/verify-exit.json. This report supplies the authorized, separately logged successful continuation; it does not relabel the original command. New append-only completion 44d2565dd55863ff470cbe1d04c7e4c37c3630de43fa9b21013c07fcae586001 binds the run, all eight native record hashes, old failures, exact environment delta, 42-commit mapping/identity proof, actual gate exits and qualified artifact hashes. All four final gates are 0. No required scoped work remains; Root review/acceptance and runtime approval are separate.

HEAD fe137b8801ef3eb2a07710367422a0da2713f519 has a Good Boxy signature; git status --porcelain is empty; git diff --check is 0. Full verification command and all limitations are recorded above.
