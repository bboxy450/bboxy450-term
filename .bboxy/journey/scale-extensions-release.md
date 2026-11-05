---
authorship:
  kind: seat
  process_generation: '1'
  public_key: fewHFqX1/aT03A8Ik46eNy0fXOwtv5Rt1gZmJu1Aids=
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
created: 2026-10-29T18:56:41.150Z
created_by: seat:01M649MS05FAD5CY9CS0Q07H76
id: 01M64GAZNR8B1KGJ6R6Z51EA0V
links:
- rel: context
  to: '[[vision]]'
number: 435
provenance:
  digest: sha256:3738454b1c1da338aba0fff536a06e456ab9c997e821c220a383f3c6cd90d16b
  frontmatter_digest: sha256:4b404c6aa16c2078cd1599099864325e92fd4bffc98b3f1e0c2600242c62c612
  log: .bboxy/log/2026-10-29.jsonl#2670
  sig: /UN0GQbGw50DQHbKVZQVXj4khUEYgkcXIpwyKwCChG9FDD2bEdgWAcwSmPSkZ2YyHE5Ww/l5LuXjv4gZImvPDg==
  signer: seat:01M649MS05FAD5CY9CS0Q07H76
title: 'Journey: full scale, extensions and the next release'
type: note
updated: 2026-10-29T22:40:41.234Z
updated_by: seat:01M649MS05FAD5CY9CS0Q07H76
---
<!-- bboxy450-term: https://github.com/bboxy450/bboxy450-term
Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md -->
# Full scale, extensions and the next release

Back to [[implementation-journey]]. Smaller proxies and private fixtures locate mechanisms; the unchanged full-count night decides the scale promises. None of these remaining issues is closed by recent input/layout micro-optimizations alone.

| Issue / next task | Remaining requirement | Evidence / constraint |
|---|---|---|
| [#9 · P1 idle/hitches](https://github.com/bboxy450/bboxy450-term/issues/9) | Zero idle hitches at 120 Hz with 1000 rows/50 watched panes and the actual HUD | P1.sh is absent. Preserve the count/rate and frame-work/CPU/GPU distinction in [[budgets]]. |
| [#10 · P2 bus/memory](https://github.com/bboxy450/bboxy450-term/issues/10) | 1000-recipient broadcast within bus budget and 400 idle seats within memory budget | P2.sh is absent; retain delivery/loss/order and actual memory evidence. |
| [#46 · full-count frame](https://github.com/bboxy450/bboxy450-term/issues/46) | Pass existing 967-seat/50-watched frame and UI CPU verdict with phase availability counts | First night: frame-work p99 20.710417 ms versus 8.33; CPU p99 20.361334 ms versus 4. Phase sampling landed; a passing new full-count result has not been established here. |
| [#47 · ingestion/input](https://github.com/bboxy450/bboxy450-term/issues/47) | Meet unchanged input-to-GPU limit without dropping tails, durability or background fairness | First night 1398.155791 ms; Written → LogicOutputRead dominated. Keep authoritative queue/cursor/endpoints; completion latency is not GPU execution time. |
| [#48 · bus throughput](https://github.com/bboxy450/bboxy450-term/issues/48) | Existing rate/loss/order checks pass at >=10,000 events/s | First retained rate 9999.018339; no rounding, pacing/threshold relaxation or unrelated fix claim. |
| [#49 · unused template](https://github.com/bboxy450/bboxy450-term/issues/49), CLOSED | Verified fallback-construction optimization landed at signed d70e94d0 | Oracle verified exact old/new paint+pane equivalence, scoped tests, exact-source full Make0, author landing0 and remote main; GitHub closed2026-10-29T22:33:29Z. Native64/50 functional phase has zero idle/guard violations; timing failures remain ungated. No measured speedup or full967-seat claim;46–48/50 remain open. |
| [#50 · synthetic late idle work](https://github.com/bboxy450/bboxy450-term/issues/50) | Identify the late producer/settle race and prove deterministic regression under the unchanged idle assertion | Two frames/callbacks on unlanded `f2403dd2`, under unequal load. Generic R5h idle fixes predate and do not explain that packet. No release-main regression attribution is established. |
| [#57 · plugin setup/auth](https://github.com/bboxy450/bboxy450-term/issues/57) | Approved bounded setup/login lifecycle with explicit unavailable/failure states | Production setup refusal is currently honest. Define the [[plugin-host]] agreement first; no credential exposure or invented success. |
| [#58 · external signed writer](https://github.com/bboxy450/bboxy450-term/issues/58) | Host-owned signing, opaque provider storage, preserved authorship, etag/conflicts and lossless metadata/body | The 418-record **read** swap does not provide external signed writes. Providers get neither keys nor caller-selected signers. |
| [#59 · Windows custody](https://github.com/bboxy450/bboxy450-term/issues/59) | Approved native Windows credential lifecycle, isolation, failure/restart evidence | Native entry point remains Unsupported; file permission custody is separate. Cross-compilation on macOS is not native evidence. |
| [#61 · plugin hooks/timings](https://github.com/bboxy450/bboxy450-term/issues/61) | Actual independent hook lifecycle/budgets and bounded host timeline/trace measurements | Keep unavailable labels until production observations exist. Mock chart/reference arrays are not measurements. |
| [#63 · skills over MCP](https://github.com/bboxy450/bboxy450-term/issues/63), CLOSED | Verified compatibility tools/resources at signed main6641b5b7; negotiated Beta migration remains future scope | Exact-source full Make0,27 invariants/0 failures/11 slow skipped; source and SHA-checked raw log independently inspected. [[issue-63-skills-over-mcp]] retains current implementation and dated proposal, without claiming unsupported2026 negotiation. |
| [#69 · firehose release order](https://github.com/bboxy450/bboxy450-term/issues/69) | Publish bboxy450-term-firehose/dependents before any old-name yank | Requires a release brief and approved bboxy450 publisher identity. No publication or yank belongs in this triage. |
| [#70 · package classification](https://github.com/bboxy450/bboxy450-term/issues/70) | Classify generic versus term-specific crates under decision 362, then coordinate any rename/release | Accepted design470 at signed5d8be221 updates [[licensing]]:27 current+2 absent proposals,24 public old0.1.0 names checked. mux/proto become term-specific; reusable render/harness/wake-up and standalone Book package remain generic; Book binary follows362. No rename/release implemented. |
| [#71 · remote spawn cuts](https://github.com/bboxy450/bboxy450-term/issues/71) | Cut 2 remote Via/Connect/routes/transport argv; cut 3 transport plugin contract | Local cut 1 shipped in `391630c2`/`88e0c196`, with native navigation evidence. Do not rebuild it. Assert compose(fields) equals actual argv without real SSH hosts. |

[Frame partition](../runs/night-2026-10-26-frame-partition.log), [[budgets]], [[supervisor-logic]], [[logic-gui]], [[launch-defaults]], [[plugins]] and [[licensing]] are the starting records.

The full workload is already owned by the existing unattended night/scale workflow. Do not overlap it with another standalone 967-seat run, shrink it to obtain a green label, add independently measured p99s, or infer a product fix from a loaded single sample. This triage did not launch the slow scale night.

Next release work follows the Makefile: `make package-crate` prepares signed artifacts without publishing; `make release`, `make publish-release` and `make push` need their explicit authorized scope. The present request authorizes issue triage and local KB organization.



Admission note, 2026-10-29: this dated triage was first prepared in signed private commits de793c11b7dd3bfd17ac453d8c47c98dc91adbae and 0efe0d917de225fa702667dc646b7981ae7edecc. The authenticated host now assigns this local copy a fresh identity and global number; the private draft numbers are not imported as current authority. Follow [[2026-10-29-zero-issues-cohort]] for subsequent assignments and verified dispositions. Unresolved promises remain unresolved.


Latest verified snapshot,2026-10-29 22:40UTC: remote signed d70e94d0,54 open. Accepted470 distinguishes old render→firehose DEV dependency from term's normal old mux/proto/render dependencies;69's fresh-install causal claim is unproven by that dev edge. Retain old license grants and existing public-name resolution until a concrete verified release plan is authorized. No publication/yank occurred. [[2026-10-29-zero-issues-cohort]] carries actual API/source/log hashes; [[implementation-journey]] carries the implementation sequence.
