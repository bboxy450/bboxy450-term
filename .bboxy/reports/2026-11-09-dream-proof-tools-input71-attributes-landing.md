---
type: report
title: Exact signed-fixture attributes committed; additional landing blocked by disk
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T22:01:32.172Z
commits:
- 9674e87c214006dbe319efbe399b515f49e97785
verify:
  command: RUSTC_WRAPPER='' python3 -I -c 'import hashlib,importlib.util,json,tempfile;from pathlib import Path;p=Path("target/dream-proof-tools/input71-formal-rebase-20261109/rebased-publication-continuation.py").resolve();assert hashlib.sha256(p.read_bytes()).hexdigest()=="82689cd1f2d06673a32411279b1b21a3b0b618fdbfd6351a9427b2290240fd0d";s=importlib.util.spec_from_file_location("retained_verify",p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m);plan,gate,old,env=m.prerequisites();published=m.c.load(m.HERE/"qualification-published.json");assert published["locator"]["evidence_sha256"]=="83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335" and published["locator"]["receipt_sha256"]=="452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c";assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];intent,basis,selected,records,hashes=m.c.revalidate();assert hashes==plan["native_records"];m.HERE=Path(tempfile.mkdtemp(prefix="retained-verify-",dir=p.parent));status,result=m.bridged_read_check(plan);assert m.c.revalidate()[4]==hashes;assert m.c.pin(m.c.PREFIX/"qualification.json")==published["qualification"] and m.c.pin(m.c.PREFIX/"receipt.candidate.json")==published["receipt"];m.c.save(m.HERE/"verification.json",{"exit_code":status,"read_check":result,"rebase_proof":plan["rebase_proof"],"locator":published["locator"],"native_reruns":0,"runtime_approved":False});print(json.dumps({"exit_code":status,"evidence":str(m.HERE/"verification.json"),"native_reruns":0}));raise SystemExit(status)' && RUSTC_WRAPPER='' make test-formal-parity && RUSTC_WRAPPER='' make test-licenses
  exit_code: 0
id: 01M7159AWDA0WS35X721V7ECWM
updated: 2026-11-09T22:01:32.172Z
number: 205483610615682
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:0639b68aef852cd2709c24031ab849025a4a049b3bf2e7c2cf5f22e344e2903f
  frontmatter_digest: sha256:c48b314208144acec1aefd3f8a6c4e377bb43f6408a5ddc8c018399e70a8c25a
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: 6CKm9McN8biFrE75DzhYzwYrSvZJaDGPcivkCyBUI0L46lDpmlZyTRNvi2k9sGNd0OO4WeOkhPmyiET8WCFDAg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151203
  event:
    id: 01M7159BVKVE96JBYB7RZ29RHB
    sequence: 151203
    hash: f2324ea7e4fc24d8578eca96439357c19e17d3cb858fe77dcad1b2de965f9d11
---
The exact two-line .gitattributes change is complete and signed as 9674e87c214006dbe319efbe399b515f49e97785, parent e90728a1b095e136db9745519333fc44810cbcb9. Both frozen fixtures are unchanged. The newly authorized additional make land invocation has NOT started: its existing >8 GiB disk precondition failed. That landing allowance remains unconsumed. No verification rerun or native qualification occurred; runtime_approved=false.

This continues [[reports/2026-11-09-dream-proof-tools-input71-landing-whitespace-blocked]] (native checkpoint 1df858af8ed26288b79e86c2d34e6d8cb1422f5c) under the task's “ORACLE RULING (2026-11-09): whitespace exemption for the two raw commit fixtures; land e90728a1”, fetched at etag a2680aba09b7151b99fe9ae7efd7a48526464fcf66f3c60d4e41ae67f6deee8d. .gitattributes is now an allowed path.

## Change and fixture custody

Created .gitattributes containing EXACTLY these two lines, with LF endings and no other content:

```gitattributes
tests/fixtures/dream-proof-tools/corpus/valid.commit -whitespace
tests/fixtures/dream-proof-tools/corpus/corrupt.commit -whitespace
```

The file is 132 B, SHA256 4355695061f09cd9f23c77fbfe27d67f224d685e36912c37f46d29ba8104c481. This changes Git's whitespace treatment for only the two raw commit fixtures; their significant signature-continuation spaces remain intact.

Before and after SHA256 results (both707 B):

| Fixture | SHA256 before and after |
| --- | --- |
| tests/fixtures/dream-proof-tools/corpus/valid.commit | f0b48d875f9069efa0ad0f12540200b4cf568d59c31e5a6ecff36fadb698d8bd |
| tests/fixtures/dream-proof-tools/corpus/corrupt.commit | b5995d22949063f0e6b2cb6a323bc3a95a04ca32a7dd58f9a56cd4aecee20207 |

Actual git check-attr whitespace over the two fixtures, scripts/dream-proof-tools.py and README.md exited0:

```text
tests/fixtures/dream-proof-tools/corpus/valid.commit: whitespace: unset
tests/fixtures/dream-proof-tools/corpus/corrupt.commit: whitespace: unset
scripts/dream-proof-tools.py: whitespace: unspecified
README.md: whitespace: unspecified
```

The exact-content assertion and before/after fixture-hash assertions also exited0. No source/test/model/fixture byte change accompanied the attributes file. Commit9674e87c changes only .gitattributes; its Report trailer names this report. git verify-commit HEAD exited0 with a Good Boxy signature under 9B07E849480C68328CA20684844ECC0587DE7AB0. Worktree status is empty.

## Disk preflight and unconsumed landing

The pre-land read-only Python check exited1 on its disk assertion:
- available8338808832 B;
- required strictly greater than8589934592 B (8 GiB);
- shortfall to the threshold251125760 B.

The same check observed the expected signed9674e87c head, parent e90728a1, clean worktree and ordinary priority0. Those conditions passed; disk was the only failed condition. This was execution chunk bced0c. No make land command was invoked, no landing start/PID record exists under the new evidence directory, and no cleanup was attempted. No whitespace gate, full task verify or native timing attempt was rerun.

The earlier failed landing remains recorded as actual2 with no push. Its canonical receipt is preserved byte-for-byte before any possible future overwrite. There is no newly landed SHA.

## Verification result retained by the ruling

The Oracle explicitly ruled that the successful amended verify at fe137b88 stands for byte-identical e90728a1 plus this exact attributes commit. Therefore the report's verify metadata carries that actual prior exit0 and the exact unchanged task command. It is NOT a claim of a fresh execution at9674e87c. The last successful command's authoritative evidence remains:

/Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/retained-verify-and-land-20261109-3vbwvoeg/verify.intent.json and verify.exit.json.

That command is1703 characters, SHA25606ef21968405f6a617fb4d775313add4c6f4ec101c2622b20a2e54ff8295d745. It passed79 units, retained-candidate validation and pins,520 native/TLC models/105 contracts, and licenses. Native qualification reruns remain0.

The qualification83365f615269cc06a977699d25dc038f01821ca778df1a2e748ec190296ea335 and receipt452390f1bda2258333a0afc3673b7fe43664456066d044bcce264adeef87406c remain the previously qualified private artifacts; this change does not admit them to runtime. The task ruling is the explicit currentness binding, not a rewritten qualification, intent or receipt.

## Append-only evidence

H = /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/attributes-land-20261109-ugyrwtdp

| File under H | Bytes | SHA256 |
| --- | ---: | --- |
| before.json | 738 | 0cfee9778f70ffcc079e200d93f587747503759c031748724a29735dd49781a0 |
| after.json | 1140 | b6c4d7f4b24078c0ca488a281d12fb44d4cafca9280c2c05620e391e5edf624d |
| land-preflight.json | 639 | 251cf523a99813e79eea00484f0bd24c50a100e4cbd638daf7ff64b62916cf97 |
| predecessor-land-receipt.json | 10293 | a741881a90ebcbdd65c11c632f395c617a26064f0dd243082f3ca073c1df9c59 |

No previous records, ledgers, sources, prefixes, qualification/receipt or completion records were replaced. No reset, stash, global configuration, peer cleanup, manual merge or push occurred.

## Exact decision needed

Release shared space with margin above the existing8 GiB pre-build gate, or authorize exact disposable cache paths within scope. The two-line implementation is finished and reviewable; only disk prevents the additional authorized landing invocation. The allowance remains unused, and no threshold waiver is inferred. No operator interaction is needed. runtime_approved=false.
