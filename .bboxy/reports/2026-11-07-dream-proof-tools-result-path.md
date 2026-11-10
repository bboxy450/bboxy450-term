---
type: report
title: Result-path Q2 freeze consumed; sole positive deadline refusal retained
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-07T23:57:16.355Z
commits:
- 5b8a11083f7d0edfe7ff5f3187578fd8c09af6d1
supersedes: '[[reports/2026-11-07-dream-proof-tools-total-66280.md]]'
verify:
  command: make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses
  exit_code: 2
id: 01M6W73SP1TS6GGJY2ME00PD1T
updated: 2026-11-07T23:57:16.355Z
number: 93377961366307
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:d32d6b2ad2b209a054e5c0fa8d7324a8f114a25331256699caac7795b0f31179
  frontmatter_digest: sha256:839bb3f27fe14f0161d3c9dd20bb5be923c583289fb3db449c9a3b42ded1560d
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: ipo9pZ/G5doo6IhyWB3M7TromTnKuemtIUkKWb05Oo5Mby8eMW84jGKACy1qYryIIHxOBknMh51D+3Wx0nmZCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/150469
  event:
    id: 01M6W73V9PATNM0GNSEWHR8G3V
    sequence: 150469
    hash: 9b9c33cb4c9c4f718d97e463c8c30580ea23a106d4c44f96b0e6ec1acd10585d
---
Blocked at the sole authorized positive; no task completion or landing. Exact frozen result-path input was consumed and one fresh private run was retained. Its actual controller exit is 81 with deadline refusal; the canonical verification command exits 2. No timing retry, later component case, final gate, qualification or candidate receipt followed. runtime_approved=false throughout.

## Change and immutable inputs

Signed code commit `5b8a11083f7d0edfe7ff5f3187578fd8c09af6d1` changes only `scripts/dream-proof-tools.py` and `tests/fixtures/dream-proof-tools/sources.lock.json`. The selected tuple, 18 preparation projection bindings, private file/root identities and Mach-O UUID now consume the task's last ORACLE FREEZE, result-path section. Source and executable are supplied immutable artifacts; neither was rebuilt or substituted. The source lock changes only its canonical_q2 source binding. The existing attributed source headers, seven upstream pins, patches, models, resource gates and unapproved boundary are preserved.

- Source/preparation commit: `79fa0d9d266cc9ccda367dc340aeae12d2c76964`; C 316503 B, SHA `33ce96860b0a8ba4d3bbfd984d5649348878ef3bf8ea51e9a37e6469a8f23b4e`.
- Supplied probe: 250872 B, SHA `f74ab7346d851f92dd0a428ab267dc0fc0d0cb464c63c24f2f3da450318e549c`.
- Preparation: 535646 B, SHA `8708273f762772745777aae92498483b79e97f587cb672692b7cf467dc72473e`, under Q2's `prepared-result-path-source50/dream-read-c-preparation/compile-r6zut56v`.
- Compiler SHA `1590ac950a3d627817d09ade5cb60b2115f17a72182a3141e010b4bcc482a0c9`; SDK 27.0 settings SHA `7b93ad7e534cc4b31c6a4e39d19b5e0288acf2168c2649479a796d1cb51939fb`; Darwin 27.0.0, OS build 26A434, canonical aarch64/raw arm64, shared-cache UUID c284a291b68b3d0da1a03f866f87efdc.
- Proposal-only read-set source digest `177cff0e6a8a658a4514494e28506184754f15ba9c50a504dd8f02817db6850c`; no compiled-current-Q2 or runtime admission claim.

Evidence directory E = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/result-path-20261107`.
`E/ingestion.json` SHA `b3494ab59e18804b6e8bbe49462da857d08eb70bddefd59ffae926115327abdd` records successful complete offline ingestion. `E/selection.json` records all exact pins; `E/preflight.json` records individual exit 0 for both source/code commit signature checks and source ancestry, plus exact equality of the frozen C with signed commit bytes. Both signatures verify as the approved Boxy fingerprint 9B07E849480C68328CA20684844ECC0587DE7AB0. Disk before build had 78749968 KiB available, exceeding 8 GiB.

## Actual run and refusal

R = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6W6RKAZEN3P4CHXXEPKPQNS`.
P = `/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M6W6RKAZEN3P4CHXXEPKPQNS`.
Original builder incarnation is 1564454904534500; candidate dev/inode 16777234/185608034 remains unchanged. Only `R/components/positive` exists.

`R/components/positive/wait4.json` records actual parent-owned reap of controller PID 36779, exit 81, user CPU 60010999 ns, system CPU 171252000 ns, ru_maxrss 4227072 B. `owned-pending.json` was written while the original handle remained alive; it is historical pending evidence, not the final state. The final wait4 settles that original handle. No timeout kill or replacement launch was performed.

Raw stdout and persisted `output/result.json` are byte-identical, 1998 B, SHA `e48546d70ea66cc72ee2a8941fc11a32ff0066cd3ad777c00d2be0d92ca2d121`. They say phase=settled, outcome=failed, error=deadline, observed_proof=not_reached, attempts=6, members_max=1, created/reaped=6/6, group_gone=true, retired_coverage=true, runtime_approved=false. Result metrics retain ordinary_max=499291 ns, native CPU/wall=532375/541000 ns, inclusive_max=650834 ns and A=64902000 ns. Retired per-grant Q/W remain null, providing no qualification evidence.

`output/ledger.ndjson` has 31 lines, 14714 B, SHA `ba5b53d8c53b2cb5fd7fc9a380ae9dd39e11322c9b4b80436c55a62da1925829`. Lines 2,7,12,17,22,27 record six decision-66280 helpers at observed PRI 4, own wait4/group-gone, no quarantine and helper debt paid. Authoritative rounded terminal CPU ns are 17002000,22611000,5385000,9116000,5139000,5649000 (sum 64902000). These are known one-member helpers; real Git→GPG child roll-up and the B startup barrier are still unproven and not reached by this result.

`stderr` is 2000 B, SHA `33206a5b1bdbec84a19e782db2a4826527da37ba0f61a2bd2f09f1f7f924fc92`. Its final receipt retains deadline count18 and no other error code; persistence=persisted, persistence/stdout errno=0, terminal_cost=passed, result_complete/tail_complete=true, 25829118000 ns debt charged=paid. It also retains closed_unit_count2727, path_walk_components2268/path_walk_units736. First deadline is attributed to frozen C line860 (`private_named_current` paid end-unit), unit44792 ns, followed by reporting_only=1. These counters are observations; this run does not isolate the cause of all elapsed time. In this run the prior source_changed/settlement_unknown/absent-result symptoms did not recur. Persistence success does not turn a deadline refusal into a qualifying proof.

`R/terminal.json` preserves unsupported, effect_unknown=false, reason="canonical real proof/component verdict failed; no timing retry". `P/qualification.json` and `P/receipt.candidate.json` do not exist.

## Verification and acceptance evidence

| Requirement | Actual result and reference |
|---|---|
| Exact frozen authority; no discovery/rebuild/substitution | PASS offline ingestion: script:52/2342–2549; E/selection.json, ingestion.json, preflight.json. Compiler/header/SDK/retained-output/closure bindings checked through the existing adapter. |
| Source authentication before extraction; fixed seven-source/private build | All seven fresh crypto authentications exit0; R/authenticated.json SHA `5c3162508f76753f4812a2219c57be65fdace3424ce0f02e8b7439f0819dcb52`. Exact source/patch snapshot retained at R/source-snapshot.json SHA `e05af70b4126db817145ebc5149409330be3b42a4a72ad3733c54d71ab14c78d`. All60 numbered build/config/link commands 000–059 exit0; their original command/PID/output/exit records remain under R/logs. No new dependency or global install. |
| Actual source/config/closure/commands/review basis and existing budgets | P/provenance/pretest-basis.json 47112 B SHA `ca87538b4c2563b2d561d119c1d81d079ef9a157cb62429fc0e5d58d838e9a60`; source-lock7176 B SHA `18c97cf7e3b8684f6b3726f368b28b8ec6252d4a904fc0c9bac2ea8c26fc93fd`; descriptor3439 B SHA `dee5b3bc38de32cd0dac0c65c61106889aa8d4cd705485f37a629c30fa320aea`. Basis has two candidate images and all8 review findings. Actual hashes/config/static/link/platform/source evidence remain in that basis and retained blobs; compiler/platform/native reachability remain conditional trust premises, not an arbitrary-native theorem. |
| Current input/negative/unit controls | `python3 -I scripts/dream-proof-tools.py unit --target /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target`: 59 tests exit0, E/unit.log SHA `744f2190bea2e9e5137879869cf7ed6919ef66175642dd38a0443189c0a797b9`. No timing attempt in this unit suite. |
| Sole positive and unaltered raw result/actual reap | FAIL as above; script:2953–2998 refuses the actual deadline result; R/components/positive/wait4.json SHA `98d9bac7b0b022ec4fa40d39709cadf7e658ccc3da361b2dc9b53741217becb4`. No claim of positive proof. |
| Positive→components→final gates→qualification→receipt | Stopped at positive. Corrupt-proof, pause/cancel/ABA/clock/debt/reap cases, fresh formal parity/licenses and final qualification/publication were NOT run. Existing model code is unchanged by this pin-only cut; inherited parity is not presented as current-source final evidence. No product/e2e availability claimed. |
| Preserve history, original effects and unapproved custody | All7 pre-existing ledger paths retain exact dev/inode/length/SHA, checked against E/predecessor-ledgers.json; none reset/stashed/deleted. Current candidate inode unchanged; known actual command/controller/helper effects settled in original records. No unknown work is certified absent. |
| Signed handoff; scope and tree state | HEAD `5b8a11083f7d0edfe7ff5f3187578fd8c09af6d1` Good/Boxy; git diff --check exit0, git status --short empty. Source delta is two assigned paths. No push, merge, landing, install, release or publication. |

Exact task verification command:
`make fetch-dream-proof-tools && make build-dream-proof-tools && make test-dream-proof-tools && make test-formal-parity && make test-licenses`
actually exits **2**. Fetch succeeds; build subcommand returns1 at the preserved positive verdict; make returns2 and short-circuits later targets. `E/verify.log` SHA `317f010e49125386ac1f569e26506eeffa9182b10e372a09c66db3d2f77b6704`; `E/verify-exit.json` SHA `c807907967cf4b0aa09fd7d1d2aba1449578fc2eb119943164cbf61c40f431a2`.

Full machine-readable handoff `E/handoff.json` 67345 B, SHA `9750b33b12c6f1f8b3edfd274ab756f4d77d11a21edec38fe0b0d9324fb7e961`, binds commands, authentication, immutable inputs, raw output hashes, helper totals, basis/source/compiler/platform, original identity/inode, predecessor preservation and remaining gates. It is offline evidence, not an admitted runtime input.

## Additional gap and decision needed

Static/actual-terminal comparison found a separate adapter gap: frozen C:3934 emits13 fields absent from script:3024's exact terminal whitelist: stdout_attempted, reporting_only, original_entry_ns, original_deadline_ns, deadline_observed_ns, deadline_first_ns, deadline_first_unit_ns, deadline_first_line, deadline_last_line, reporting_started_ns, closed_unit_count, path_walk_components, path_walk_units. `E/terminal-shape.json` retains the exact producer/script hashes and differing field sets. This check was not reached in the canonical run: component_result first refused the real deadline. No permissive parser, dropped diagnostics, source edit or replay was used to hide either issue.

An initial local selection-construction command exited1 because it used the wrong identity helper, producing an incompatible in-memory receipt shape. It made no source write or process launch. Correcting construction to the existing q2_private_file reader produced the exact full identity and successful ingestion exit0; E/selection-construction.json records the distinction. This was not a candidate/probe/timing retry.

Oracle decision needed: disposition of this actual deadline refusal and a causally changed authorized Q2 input/scope before any further run, plus disposition of the producer/consumer terminal-field mismatch. The existing 30s/1ms/native/debt/resource laws were not waived. No unchanged-input retry is authorized. Blocker DM `01M6W6YTG77Q0MRGMEVD5S0J3Y` gives the exact ledger; message_receipts reports native-queued, model consumption unverified. This append-only report supersedes the prior freeze's current handoff only; all historical failures remain failures.
