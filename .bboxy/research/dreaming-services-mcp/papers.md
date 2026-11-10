---
type: research
title: Offline consolidation and memory evaluation evidence
parent: '[[research/dreaming-services-mcp]]'
id: 01M6P14M10BP0RY714ZX0PRE4V
updated: 2026-11-05T14:17:24.078Z
created: 2026-11-05T14:17:24.078Z
number: 93377961364650
created_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
updated_by: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
authorship:
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  kind: seat
  public_key: L62ggzXO9nS9icBZrd6nPN71BnmasbyzYyfTJE3eu0k=
  process_generation: '1'
provenance:
  digest: sha256:d14f8c0b25e5b30c9d97d086f2314d87987bf8181fec56441a79a39d32982422
  frontmatter_digest: sha256:75277350e28ec15103a131cd04f5743704a86b54481ff203e1ed2535dc113c82
  signer: seat:01M6NJ9RS9EXJ7M5AK1AY1B3JR
  sig: A9rXiq/utu1aZLQTyNbbtxWMwK1ESOJcUcwnL3tg2ybnLsv9pO0y4Wp4YZYN/FYx9Tr9STRxvXRz06GRN1SJBg==
  log: .bboxy/log/2026-11-05.jsonl#136570
---
# Offline memory needs bounded, falsifiable lessons

## Which mechanisms carry useful evidence?

### Takeaway
Separate offline preparation from live retrieval, preserve small inspectable lessons and temporal evidence, and test abstention/poisoning as seriously as successful recall. These are design inferences for bboxy, not reproduced results.

### Cited findings
All originals below were fetched on **2026-11-05 UTC**. Dates identify the original/retrieved version, not the search engine's relative date. Abstracts establish reported mechanisms; only ExpeL's methods HTML was additionally opened. No experiment was reproduced and scores from different setups are not a league table.

| Original | Supported mechanism and controlled-evidence limit | Consequence for our design (inference) |
|---|---|---|
| [Sleep-time Compute](https://arxiv.org/abs/2504.13171), 2025-04-17 | Offline work anticipates later queries. Authors evaluate modified Stateful GSM-Symbolic/AIME, multi-query amortization and a SWE case study. Their reported savings depend on repeated related queries and predictability. | Prepare once after closure; separately charge consolidation and retrieval. No guarantee of savings for unrelated tasks. |
| [LightMem v4](https://arxiv.org/abs/2510.18866), original 2025-10-21, revised 2026-02-28 | Staged filtering/topic organization and offline long-term updating separate online work from consolidation. Evaluations use LongMemEval/LoCoMo with selected GPT/Qwen backbones. Revised abstract numbers differ from earlier indexed snippets. | Bounded incremental index updates; measure total tokens/CPU, not just online latency. No benchmark multiplier is adopted as our target. |
| [ExpeL v3](https://arxiv.org/abs/2308.10144), original 2023-08-20, revised 2024-12-20; [methods](https://arxiv.org/html/2308.10144v3) | Experience collection, retrieval and natural-language insights from successes/failures; methods section 4.2 distinguishes those comparisons. This is agent experimentation, not proof of arbitrary production lessons. | Keep contrasting evidence and exact command outcomes; hold out tasks for with/without lesson trials. Reflection alone is not verified adoption. |
| [ACE v3](https://arxiv.org/abs/2510.04618), original 2025-10-06, revised 2026-03-29 | Incremental generation/reflection/curation addresses loss of detail under repeated whole-context rewriting. Authors report agent/domain benchmarks and execution-feedback adaptation. | Immutable lesson IDs/revivals and explicit supersession beat repeatedly overwriting one global memory paragraph. |
| [LongMemEval v2](https://arxiv.org/abs/2410.10813), original 2024-10-14, revised 2025-03-04 | Tests extraction, multi-session and temporal reasoning, updates and abstention. Chat-history QA is not a coding-agent end-to-end benchmark. | Test stale/updated/no-answer cases, not only hit rate. Add coding-specific command/time/false-advice measures. |
| [MINJA v5](https://arxiv.org/abs/2503.03704), original 2025-03-05, revised 2026-02-12 | Query-only interaction can poison retrieved memories without direct write access to the memory bank. Threat model and constructed bridges matter. | Treat feedback, tool and web text as untrusted observations; never let repeated retrieval or model confidence grant authority. |
| [Memory Poisoning Attack and Defense v2](https://arxiv.org/abs/2601.05504), 2026-01-09/12 | EHR-agent experiments vary initial legitimate memory, prompts and retrieval parameters; defense calibration can reject too much or miss attacks. The domain and tested models bound transfer. | Require a poisoned-feedback negative and a useful-memory positive so a filter cannot pass by rejecting everything. |

### Inferences
Our IR-backed advantage is not a larger self-editing memory store. It is auditable evidence, explicit trial/publication state, per-harness eligibility, immutable pins and honest currentness. Inference: fast prepared reads plus reported-use feedback can reduce repeated investigation without permitting live consolidation.

### Gaps
No live vendor evaluation, paper reproduction, provider spend or coding-workflow improvement was measured. No paper establishes our process responsiveness, signing/authority correctness, CPU allocation or resource ceilings. Those require repository-local formal controls and unattended process witnesses in [[specs/dreaming-mcp]].
