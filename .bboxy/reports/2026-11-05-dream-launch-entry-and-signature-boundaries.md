---
type: report
title: Dream launch-entry, signature temporary and platform closure dispositions
task: '[[tasks/2026-11-05-dream-launch-entry-and-signature-boundaries]]'
author: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
created: 2026-11-06T03:07:54.505Z
commits: []
verify:
  command: git diff --check
  exit_code: 0
id: 01M6QD7EJD8K9QJ77XK6EP3SF1
updated: 2026-11-06T03:07:54.505Z
number: 93377961365966
created_by: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
updated_by: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
authorship:
  signer: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
  kind: seat
  public_key: X/rzXNejssjUR5rEaGHy1fr/qmfAtbpX+NAM+IbU7Hk=
  process_generation: '1'
provenance:
  digest: sha256:2c3762a2a5d89a397ee3bd300d5065b978581f18327f287754369b9b536844aa
  frontmatter_digest: sha256:edf6f8deac625f1ae68f454af6822ed4f4371cc8d057679eee0efa81b2c931b3
  signer: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
  sig: 1WJo1BieePBgTXGRZMaze7BResVZochpjOEvPawjkXlZlbqaZ8rHRJhqeJNSNb+o4z1wnjyqwLx2Jwid39h8Aw==
  log: .bboxy/log/2026-11-06.jsonl#149107
---
# Dream launch-entry, signature temporary and platform closure dispositions

Architect seat 01M6QBVWBSF0CCV4HGVC3V9JQY, working in /Users/feral/mydev/bboxy450-term. This is design and record work only. I made no product source edit and ran no build, test, probe, launch or retry. No commits: `.bboxy` is private KB state, written through knowledge_write.

## Outputs (final digests, sha256 of the on-disk record)
| Record | Digest | Content |
|---|---|---|
| [[decisions/d93377961365960-dream-launch-entry-and-signature-boundaries]] | 6cc010ae…a323 | A (corrected after Root review) and B rationale |
| [[decisions/d93377961365962-dream-platform-closure-boundary]] | 2fd9b5b0…7134 | C rationale |
| [[interfaces/dream-proof-tool-qualification]] | 98a3023e… → ba36ee07…7cd1 | canonical "Signature temporary" and "Closure boundary"; verify_child token; Config sentence; Closure pointer; 2 matrix rows |
| [[interfaces/dreaming-mcp]] | 8b0a59f6… → 9c54b307…5dd6 | the one verify-child reference now points at Signature temporary |
| [[interfaces/dream-term-c1]] | c79408f6… → 78219b91…b229 | canonical "Bootstrap entry record" |

Earlier frozen handoff digests:
- B: 65960 at a130538f.
- A: eb0b4af3, superseded by the corrected 6cc010ae.
- C: 2fd9b5b0.

ICD edits were applied by two subagents from texts I staged and checked: exact-match anchors, write responses, and on-disk sha equal to each record digest. I grep-spot-checked the results myself.

## B: signature temporary (handed off first)
Read set: authenticated, patched Git 2.54.0 in builder run 01M6Q80DBV75S95DWFHZWDVE2F, identical across six runs.
- gpg-interface.c 191dd45b:360–386
- tempfile.c a34ef38d:68–100,191–195,360–375
- tempfile.h 20c6e576:187–189
- wrapper.c 788c07b3:438–481
- run-command.c 03bf7f11 (patched)
- patch a48b3377
- builder HEAD 78391c4c

Q2 seam: probe.c e77ce1fe (moving, not a pin) at :1174, :2318–2333, :2344, :2023, :2651.

Disposition: `{git_signature_temp}` is a matcher token. GS1 checks the argv and the file at the first observation of the verifier; GS2 checks deletion and the inode at reap. Each criterion maps to the ICD section:
- typed relation and custody;
- prefix S;
- FD/no-follow/mode/link/inode/size/hash;
- exclusivity;
- lifetime and deletion;
- timing;
- codes;
- budget;
- residual premise;
- controls.

Valid, corrupt, ABA, symlink, hardlink, planted, extra, bad-byte and early-deletion cases are listed as process negatives. Foreign uid is covered by model plus review only, because it is not reproducible unattended.

## A: launch-entry gap (corrected)
Read set:
- receipt j_bna5tg (receipt afd40e3b, lifecycle 91f78d7a, stderr 262fbc60);
- d06e696f main.rs 933e39cb, c1.rs 63b703f3:418–474, bootstrap.rs 14b3155d (Gate::wait raw write of the waiting line, `Line` is 256 B);
- SDK resource.h 1067178:289–326.

Correction applied after Root feedback 01M6QCJ0PNMGTG3QV7APA5KYM9:
- The +350.426 ms observation is only an upper bound on emission.
- The fork/exec side is not excluded. Exec, loader, std::rt and observation delay remain hypotheses.

Disposition: one ordinary producer entry line before the unchanged waiting line. Primary evidence is the unit-safe CLOCK_MONOTONIC bounds spawn_return→e, e→m and m→observation. The rusage V4 counters are retained raw and unverified.

Owners and next steps:
1. A fresh Oracle producer correction task on native-c1 at d06e696f.
2. A canonical Make tuple build to a new hash, which needs Root re-approval before birth.
3. A consumer strict parser under its existing task.
4. One ordinary joined 250 ms gate run.

Model, broken controls and falsifiers are in the ICD and the decision.

## C: platform closure
Consumed [[reports/2026-11-05-dream-platform-closure-packing-evidence]] (4285c552…) and its failed bounds, without re-running anything.

Disposition: the closure ends at the bound platform boundary.
- The candidate side stays complete.
- Root loads must be candidate nodes or install names resolved in the bound dyld shared cache.
- Platform = {darwin, os_build, arch, cache_uuid, premise}, with currentness checked before birth and after settle.
- Caps are unchanged.
- Fit for the final pair is UNKNOWN until one real builder emit. Over a cap, or a non-cache, rpath or objc-env load from a root, the host stays Unsupported with the exact cause.

## Notifications (stored; delivery/application acknowledgement not checked)
All three messages went to Root (01M6NJ8E8ZZ27GP1HDBR5R550E) in thread 1TX9BB683JNR093T02N7QP7X8V:

| Message | Content |
|---|---|
| 01M6QC7Z30RX2705S800E8HPH9 | B |
| 01M6QCGP4K2W1RVW24G339YK5M | A (original) |
| 01M6QCRHW9Y3QG4G6NMA09FTAZ | A correction plus C |

I sent nothing to the builder or Q2 seats directly. Root relays and amends their live tasks, per the brief.

## Verify
- `git diff --check` in /Users/feral/mydev/bboxy450-term: exit 0.
- Limitation: every touched record is untracked `.bboxy` state, so this check covers no change of mine.

## Unproven / not done
- The Rust 1.99.1 std spawn path was not read: rust-src is not installed.
- XNU source for the running Darwin 27.0.0 was not read, so rusage units and overlap semantics are ungrounded.
- I ran no process, runtime, model or formal check.
- The actual Basis fit for C is unknown.
- GS has a residual premise: there is no proof that gpg opened the GS inode before GS1.
- The Q2 probe was not frozen.
- Nothing was committed through knowledge_commit.
