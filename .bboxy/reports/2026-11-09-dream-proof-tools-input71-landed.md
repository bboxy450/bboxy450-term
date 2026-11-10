---
type: report
title: Dream proof tools landed with retained INPUT71 conditional qualification
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T23:01:49.129Z
commits:
- 7b8ed04fe222c27970901811323592d7a23c73ae
verify:
  command: RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path("target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()=="82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d";s=importlib.util.spec_from_file_location("retained_verify",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/"qualification-published.json");assert published["locator"]["evidence_sha256"]=="83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335" and published["locator"]["receipt_sha256"]=="452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c";assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan["native_records"];m.HERE=Path(tempfile.mkdtemp(prefix="retained-verify-",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];m.c.save(m.HERE/"verification.json",{"exit_code":status,"read_check":result,"rebase_proof":plan["rebase_proof"],"locator":published["locator"],"native_reruns":0,"runtime_approved":False});print(json.dumps({"exit_code":status,"evidence":str(m.HERE/"verification.json"),"native_reruns":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses
  exit_code: 0
id: 01M718QQ1D8560B78EVJAFV5W6
updated: 2026-11-09T23:01:49.129Z
number: 205483610615690
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:adc54b00869ab6b92815f8cabf4b9c43bbbf757de54ec1e639aaa33caab37e52
  frontmatter_digest: sha256:30ded970ef08eba8d1c156e84e9653eedb0d1011437d92983905467ad08bfb51
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: udzx5Madazoy3OmyTcnuhsyHncsYr9rg0FFe59WDgnjyO/ZL4b5Z4NEgMixM8zrHDKxIbeTBMPbbs6jYfDkmAQ==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151229
  event:
    id: 01M718QR1A0WZFE9H4J9Z6QC8S
    sequence: 151229
    hash: a72401ba558a23080a9bc6adae4a944f472b807d514b17a0bd4bf94a092f7fd0
---
LANDED on harness main: 7b8ed04fe222c27970901811323592d7a23c73ae. The one authorized canonical make land invocation exited0; the successful final push was followed by a fresh remote fetch and ancestry check, both0. Observed origin/main and local HEAD both matched the landed SHA; worktree is clean. The qualified INPUT71 candidate and all original native evidence remain intact. runtime_approved=false; runtime admission remains a separate Root review.

This completes the scoped provisioner/landing work under [[tasks/2026-11-05-dream-proof-tools-provisioner]]. It continues [[reports/2026-11-09-dream-proof-tools-input71-attributes-land-clippy-blocked]] and preserves all preceding failures as history.

## Upstream fix, rebase and owned-byte proof

Fetched and verified upstream b46fad659627de01e704a087082d10380e120193, signed by the expected Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0. Its actual ACP test fix uses events.iter().rfind(...), replacing filter(...).last(). I made no edit to tests/acp_agent.rs.

Refetched task etag71e6deffa46b932d6ee55304b611a87df3fcfa722c30d90911582fbc59a15707 and the “upstream clippy break, one more land” ruling: rebase onto the fix, preserve builder bytes, run one canonical landing; no task verify or private native qualification rerun. The ruling accepts the external INPUT71 availability note without adding a gate.

Explicit rebase of all43 owned commits onto b46fad65 exited0 with signing enabled. Pre-rebase31ff83c585dd1def7f68fbc38db5d92c12dc722e is preserved at archive/dream-proof-tools-before-b46fad65-20261109. Initial post-rebase HEAD was cd44bb311115a057c8300f43bf77b0d359d5665b.

H/rebase-identity.json, SHA256798887542476a1fca2a319b74b2f11a98d7679104647569acb208a5e8d387a11, proves all39 owned Git entries—38 dedicated builder files plus .gitattributes—identical in bytes, modes and complete path membership to31ff. The38 dedicated files also remain byte-identical to frozen c50; recipe SHA256 remains a8187a0ca0b093bdf4e82c2f3e172b664f60a4e5fadc2cb0d50256b8c5d62edd.

The exact132B attributes file remains SHA2564355695061f09cd9f23c77fbfe27d67f224d685e36912c37f46d29ba8104c481:

```gitattributes
tests/fixtures/dream-proof-tools/corpus/valid.commit -whitespace
tests/fixtures/dream-proof-tools/corpus/corrupt.commit -whitespace
```

Both707B raw commit fixtures remain unchanged, including their required signature-continuation space+LF lines:
- valid.commit: f0b48d875f9069efa0ad0f12540200b4cf568d59c31e5a6ecff36fadb698d8bd.
- corrupt.commit: b5995d22949063f0e6b2cb6a323bc3a95a04ca32a7dd58f9a56cd4aecee20207.

## One canonical invocation; two internal source-bound rounds

Starting disk exceeded21 GiB, priority was0, source was clean, and the owned39-entry comparison passed immediately before spawning. Exact invocation:

```sh
RUSTC_WRAPPER='' make land
```

The owned logger at H/run-land-once.py has no retry loop and used exclusive start/PID/exit/log records. There was ONE Make invocation.

Canonical scripts/land.py performed its existing internal fast-forward handling:
1. At cd44bb31, all required gates passed, including520-model/105-contract parity and final evidence validation. Formal evidence SHA256a210ee157fc381e9b5e5c9e2ba1fa9be967d4408d87ef49ff2a3693d42da4bde binds that source. The push then exited1 because remote main had advanced (fetch first).
2. Within the same invocation, the canonical script fetched/rebased onto5f83d8f5f310ae25ef5a9264245dc21877136d8a, re-signed and verified all43 commits, reran its required gates, refreshed final source-bound formal evidence and pushed successfully. Final push0; Make0; landed SHA7b8ed04fe222c27970901811323592d7a23c73ae.

No manual re-invocation or lucky private timing attempt occurred. Both sets of43 commit-signature checks passed (86 actual checks). Initial cd44 is preserved at archive/dream-proof-tools-verified-cd44-20261109; earlier c50/fe137/9674/31ff refs and reports remain.

The log's nonzero intermediate commands are not hidden: each rebased source first failed the optional stale check-formal-evidence lookup (Make2), which the canonical script followed with a complete fresh parity run and a successful final check. The first push was the expected non-fast-forward rejection1. The final required gate path and push all exited0.

## Actual final-round verification

| Canonical gate | Actual exit / result |
| --- | --- |
| Source diff --check, public-only key import and compiler-priority smoke | 0 |
| Header/license inventory and10 license tests | 0 |
| Embedded Clippy | 0 |
| All-targets acp-agent Clippy | 0 |
| acp-agent,test-fixtures lib/MCP/launch/formal Clippy | 0 |
| Concurrent nextest lib lane | 0;107 passed,103 skipped by its configured lane/ignore filters |
| Every named serialized lib family | 0; counts below |
| Changed dream_proof_tools integration wrapper | 0;1 passed, calling the unchanged79-unit recipe suite |
| Formal Rust tests | 0;45 formal_models,74 formal_nonvacuity,1 reactor_projection passed |
| Formal-tool tests | 0;12 passed |
| Full native/TLC graph and expected-verdict comparisons | 0;520 models,105 contracts,1 crate |
| Final current-source evidence validation | 0 |
| Final git push origin HEAD:main | 0 |

Final serialized lane results, preserving configured ignored cases:

| Family | Passed | Ignored |
| --- | ---: | ---: |
| dream::bootstrap | 4 | 0 |
| dream::c1 | 40 | 0 |
| dream::fingerprint | 4 | 0 |
| headless fixture environment | 1 | 0 |
| mcp::source_tests | 4 | 0 |
| dream::read_wire | 3 | 0 |
| dream::worker::tests | 6 | 0 |
| prepared_qualification_tests | 0 | 1 |
| selected_helper_process_tests | 9 | 9 |
| dream::read::helper_tests | 21 | 0 |

No ignored case was silently reported as executed. No separate private INPUT71 native qualification was rerun; its eight original records remain the qualification witness, as directed.

The final formal archive is:
/Users/feral/mydev/bboxy450-harness-dream-proof-tools/verification/evidence/d1da261e6e2c2894d5e3d29a14f096c1f490a56692645b63479889317648d959/evidence.json
3697485 B, SHA256d1da261e6e2c2894d5e3d29a14f096c1f490a56692645b63479889317648d959; source7b8ed04fe222c27970901811323592d7a23c73ae, dirty=false,520 complete matching graphs,105 contracts and560 current input hashes. The bounded-model result retains complete_tlc_replacement=false; it is not an arbitrary-native theorem.

The exact amended task verify remains the previously executed1703-character command, SHA25606ef21968405f6a617fb4d775313add4c6f4ec101c2622b20a2e54ff8295d745, actual exit0 at fe137. Its result transfers under the explicit Oracle currentness ruling and the byte-identity proofs; it was not rerun. This report's verify metadata preserves that exact task string. Current-source landing/formal gates above are fresh results for the landed source.

## Remote and source verification after push

A new git fetch origin main exited0. Observed origin/main=7b8ed04fe222c27970901811323592d7a23c73ae; git merge-base --is-ancestor landed origin/main exited0. Local HEAD matched; git status --porcelain was empty. H/remote-main-verification.json records this network-backed observation.

H/post-land-source.json independently recomputes the complete tracked-source digest c93be1ffff3bd71893c8405422ec6f7fd60e03f8786e6c74116338e734633f72, matching the canonical landing receipt. All39 owned entries still match the pre-rebase snapshot; no out-of-scope source edit was made.

Original31ff-era owned commits -> final landed commits:

```text
4e0e8a6e59b7191e36f0c1e93d089ed13047721a -> 6d76276903287d2021605a2dfc06a20a28a0ba1b
0c3919485b50545373dd0cd9b464ea20899c8759 -> 59b3f64d508122046bd3e0d5c8d6ffdb2dc138d5
76f851695f806d92a98a572c2c0df0f0d404d983 -> 55c7aac0f338905029c3bf99aa5f4b4a880595bd
cf18395cd1be5e184b5f791b0e325a82838801b8 -> 4542f20231378241343dd507ed13372c205e4454
192f13f921c763d652c2916e67beb43e127ab07c -> f12f83f9ac620373a5f6ded5c4e6c904267183f6
f0205fc1a360e043c7a2745f409392670042c66a -> e0d5b7498c1642fcb0d69b502362d9f3fcd76c69
1e4ef98b0fcb966741d19897991ed9d5763800bc -> e99b20fdc34f992d6bcd41eaa86d7e10eb4ffad9
2f85256229865bfddd89ef9c1f8fd5fcbcff409c -> 5cf1cb5a9327d2ca74fd00fd0f228d62b206ce5c
8d4fb45a025c7aba07eab849c05735a0b615bc16 -> da002a7e7e1755199e939adf704fb433efd1ebb2
5136c69dc9f14ad8266118066e3a39879a7adf65 -> b88e1915db1460d6da4b565b8783a937e76318ba
a54d2f5a51829f8fcf17f4268c9464266277432c -> b5e61c8c6e7ab2fd29689dc3da7c28e8f76399e0
c69b43d926c828d6a25ec77563569aab46cfb504 -> eb758fc972632a5a2045e1af9bf8b93a165b1170
43c6716c4f0d2cf257da427feaeea3962ff2c596 -> a4f44fb5bb36bc5815f7c921f376861b21b7a3e9
58d42ffa4baf9146094e95c73ede8d90ba1e37e9 -> bf8f15bd1eed6d97c1aa1c7dc53f281e5bcd9f51
0c93eb060268682c2e59fd5d76e963acb1135b95 -> 5c4e68fe877c9baacd87d2f795ffba4968ebe032
bf13a32408a34227bd67c91a2dae40215acc89cd -> 929be73bb46a3afdfc237779ad516301e8956696
e80d634cc4fa836fc74ac2472acffac3054cd99d -> bad13985c4e43091f01e6d137cfba23176671f27
9bf9bf626608b4e98593347df9859b64f295d04e -> 5b092df8a04d4fed887365b9475db3ead2b8018c
3b6a58a969ab760cd3dd1b35eb92eb56d52fa591 -> 487626317e3d7efbdf29b61b573b6ff5bb6742e6
ebd49321d6805fca1d9c973587e8ea2b77e95157 -> 1445ee34b00ebfe030770d02f41948623986a697
34fa40b6278139bada8b8ed2bdbf10dcd83b24b7 -> d83b5b9e195f9e34c3cbb0aaca045d46218b4f4e
e4e0c848066ea36329154e57590785a63b632e83 -> fce3a697d8ae8002af2dd858d7168d40f6d95429
af7ff4b5a1e8192ba5ce06e199369a5c3273f0cc -> d3ebe6f8458811dc622a45bd2c2d736bcaf15704
11c9abc959d023470361687a698ac171ae659a2f -> a9e016977866eea97234140875374ba7f53b6020
1e78610dd7dc7da75685a25d74e4e53b66a1cebf -> 3082e0695c72fd94c04b22ff07f77245a28ece35
b9a399f174bc537a97554fe1ca3b56d52a723d0d -> bb8412e01f7a2a3b0e2508d4ba16ea56ca605509
1f10c88d5694b79d2e6a52a5d81969625e94cef5 -> 298a0a2a1306144159467ba4135a67f7399cefb9
7a8d405cfd07d5220ab3f8148707d1f063477ff8 -> 521184b2ae36a1e75133ebcad20c9597a7819785
c75a6768eec1063d507ab9f83ec27f24cdf47bb7 -> 21bee88744ff87d1d2c2c6ef2fbd0b5f61e3a950
d2ea9c6427fef75d48fd83b9f671de3aec4c40b0 -> e46d155b302bc99637ecc3c8e9efa2e6f792033d
678a4fe144dd433eb4aa0673d732e3465ee82317 -> e918f9fc3fcacb48f8b82e20010bbf864da77146
063640165969ae1a268c3ef591238ce8b77b19e4 -> edff62a749214f3d40dfed72b287e396c86ae287
bda5b72658d654b42713afc06a1def84318304b7 -> c3275f43f5729b28d3036a48982c0defc79bc52d
ad58302f591738dafdbb37ef7dd9828b2f3f8838 -> 438a704c2732ca1a070200eed3b05a6bc05263da
cc8aaf10ad15e2e1cb93e23a5fc8d8a788db827b -> b85acb42ec498cb15f60f117c83fc1a94abf123d
09fc9dd80309e688366fd68a34cbae33efa68908 -> 0c18871fa4fdda061d1000144df9a5344b981177
9a35afd9f650888e2c1db4a805705562b5ad0ecb -> 8e044240872047fed4e6f35e1f15bda897aab64d
50c4b8fb0e077e5fd61e6ce3e3b2c5777ddf768d -> 653c4fb6bc1beb3ba7f3dff9d876a57aeafa6903
f4ad41941d978810449d3243a8d7c62a197ca7d1 -> cbec990879faf0a5fc5302db1dbcaa0e5a985206
eb67a8f46332344bab2ca8517e99d229f8b5a3e7 -> e210083bc852ddb2b3fe15412861907003106f97
6feeb22f84aba095d7bdd8b971c682b90746990a -> 0114efb4a31c1824d3a620f2f3734f538757b81b
f3c73d9f3f5a1acc63ae5a90798b2114311bf518 -> 22bae5fad18b8a18f3cbe4f3b2b025c66205ca59
31ff83c585dd1def7f68fbc38db5d92c12dc722e -> 7b8ed04fe222c27970901811323592d7a23c73ae
```

## Retained qualification, evidence and limits

Candidate prefix:
/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/candidates/01M70XWHWKFCD1W6ZZMQB50WG1

Qualification39083 B SHA25683365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335; receipt969 B SHA256452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c. Their exact bytes/device/inode, all157 qualification references, eight original native records/40 original artifacts,33 historical ledgers and53 preservation references remain intact. Original060/061 failures, unsupported terminal, earlier continuations and prior landing failures are unchanged.

The original native run remains valid under the recorded source/currentness rulings: positive/corrupt_proof/pause_resume each28/28, expected control cases accepted, actual wait4 exits preserved,50 clean shim receipts. Full per-case debt/margin/ordinary/pause/escape details remain in [[reports/2026-11-09-dream-proof-tools-input71-rebased-qualification]] and the original result-evidence records. No fresh native timing metrics were invented.

The five external INPUT71 custody paths remain absent after the Oracle's Q2-target cleanup, an availability condition explicitly accepted in the ruling. P retains matching C/probe/preparation/shim bytes and source bindings. No original external path was reconstructed, substituted or claimed currently available.

runtime_approved=false, conditional_on_bound_source_review and whole_command_lifetime_proof=false remain. The coalesced-fork and setsid-orphan limitations recorded under decision66459 remain explicit assumptions. Source landing and candidate review-readiness do not admit a runtime profile or publish an available lesson. No runtime admission, installation, crate release or peer cleanup was performed.

## Append-only handoff

H = /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/upstream-clippy-land-20261109-h529z8uh

H/handoff.json is31208 B, SHA256f44118a4340817e8cf5444a2b77a80bf355cad822a8d51554fb48547d4fa4158. It binds both internal rounds, all actual exits/commands, the final43-commit mapping, formal/remote/artifact hashes and preservation checks.

| File under H | Bytes | SHA256 |
| --- | ---: | --- |
| before.json | 6475 | 462afff3843ff467da6b4919d9a6d4f80b399e81705171a0ec499cc79e270865 |
| rebase-identity.json | 7659 | 798887542476a1fca2a319b74b2f11a98d7679104647569acb208a5e8d387a11 |
| land.intent.json | 605 | d1c0c56b69d8c36acb196a70be0dfc0cf6a42be4bfd3c5fec150d2713600583a |
| land.exit.json | 62 | d85e1f0bc0d0462ff1db15359cd39f7c297f22b05d1887909260ac390f5c18cd |
| land.stdout | 1437465 | dbaa8c03ee1f00baada8e3f0c4e860b846338b0c8cbffcca0a894a459651125d |
| land.stderr | 82521 | ff66e6e9c31036a9d557be3a2280d313d7d7f6e1be052b7e1193442a1bf12dad |
| land-receipt.json | 40078 | 66e42855e1c49b638ef8d320995413e5908d43eea36fd422795a41de9ab634eb |
| remote-main-verification.json | 485 | 2659138b31ecbb55da85589c950e276075574c0cf2ba3b606b169f6c09d54385 |

The canonical target/harness-land-receipt.json is copied without replacement to H/land-receipt.json; the previous failed receipt remains separately preserved. No report or prior receipt was overwritten.

Outcome: landed7b8ed04fe222c27970901811323592d7a23c73ae, Make0, final required gates0, source clean, remote containment verified. Scoped implementation/qualification/landing work is complete; acceptance and runtime admission remain Root authority.
