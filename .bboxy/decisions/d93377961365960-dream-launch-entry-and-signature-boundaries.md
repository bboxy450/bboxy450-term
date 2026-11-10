---
type: decision
title: Dream launch-entry witness and Git signature temporary binding
status: decided
date: 2026-11-06
task: '[[tasks/2026-11-05-dream-launch-entry-and-signature-boundaries]]'
derived_from:
- '[[decisions/d93377961365882-dream-c1-bounded-launch-diagnosis]]'
- '[[interfaces/dream-proof-tool-qualification]]'
- '[[decisions/d93377961365126-dream-q2-helper-creation-and-lifetime-budget]]'
id: 01M6QC6ZDY6SDAXS3FMXJKNFM4
updated: 2026-11-06T03:00:38.448Z
number: 93377961365960
created_by: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
updated_by: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
authorship:
  signer: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
  kind: seat
  public_key: X/rzXNejssjUR5rEaGHy1fr/qmfAtbpX+NAM+IbU7Hk=
  process_generation: '1'
provenance:
  digest: sha256:feaa5070900522a92f04cb2be4e987133b7ae4f7e8a54acd6e81f4b608e0f46b
  frontmatter_digest: sha256:38ff5a2ed86804b8f596d89ea32799d2531ced208bda40f63dfd0e5998161e34
  signer: seat:01M6QBVWBSF0CCV4HGVC3V9JQY
  sig: pvtZf3HeSbhmk2e/szDUKY38kB9xtz4GsP86yTIvsswvxv8XMdDGRepRwNVDdSx4nSMjvpfk0EijhELv8bvQDA==
  log: .bboxy/log/2026-11-06.jsonl#149016
---
# Dream launch-entry witness and Git signature temporary binding

Date 2026-11-06. Architect seat 01M6QBVWBSF0CCV4HGVC3V9JQY. Part B was decided first and handed off on its own (B digest a130538f…7c32); part A was added after it. The canonical normative text lives in the interfaces: B in [[interfaces/dream-proof-tool-qualification]] "Signature temporary", A in [[interfaces/dream-term-c1]] "Bootstrap entry record". This record keeps the evidence, the alternatives and the reasons.

## A. Launch-entry gap: one producer entry record (corrected 2026-11-06 after Root review)

### Evidence read (exact)
- Receipt `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-j_bna5tg`: receipt.json afd40e3b…aa4, hello.lifecycle.json 91f78d7a…5a2, hello.stderr 262fbc60…6b0.
- Producer `bboxy450-harness-dream-native-c1` at d06e696f: src/main.rs blob 933e39cb…ca9, src/dream/c1.rs 63b703f3…471, src/dream/bootstrap.rs 14b3155d…b05.
- SDK `sys/resource.h` 1067178…382961, lines 289–326: the `rusage_info_v4` layout only.

### What the sample does and does not show
- bootstrap.rs `Gate::wait` emits the waiting marker with one raw `write(2)`, before any permit read. c1.rs:418–474 samples `entry_ns` and the prelude CPU first, then performs borrowed recognition only; neither sample is emitted.
- The parent first **observed** the waiting marker at +350.426 ms after spawn_return, against a deadline of +250 ms, with lease retirement at +251.027 ms and EOF at +351.113 ms. That is an **upper bound on emission**, not an emission time. There is no child emission timestamp and no drain-latency bound. The sample does **not** prove that the gate was reached at ~350 ms.
- The preexec marker at −0.775 ms precedes exec. On its own it cannot causally exclude the fork/exec side.
- Exec, loader, std::rt, scheduling and marker-observation delay all remain **hypotheses**. The Rust 1.99.1 std spawn path was not read (no rust-src is installed). No cause is established.

### Alternatives
| Option | Verdict |
|---|---|
| Wrapper-only rerun | Rejected: it has no child clock, so it is the same blind spot. |
| Different launch primitive | Rejected now: no evidence implicates it, and switching would assume an equivalence nobody has shown. |
| Parent-only `proc_pid_rusage(child)` at the deadline | Named fallback only, not authorised. |
| Feature-gated diagnostic binary | Rejected: a different hash cannot certify the ordinary binary. |
| **Ordinary producer entry record** | **Chosen.** |

### Choice
The canonical law is in [[interfaces/dream-term-c1]] "Bootstrap entry record". It adds one fixed, non-allocating line before the unchanged waiting line, carrying:
- the child's own entry sample e and a pre-write sample m, both CLOCK_MONOTONIC;
- raw rusage V4 counters and mach_absolute_time, retained verbatim as **unverified auxiliary** values.

The primary evidence uses only the unit-safe bounds `spawn_return→e`, `e→m` and `m→observation`. Raw counters are never converted, subtracted or classified until a source-grounded premise for the exact kernel is recorded. That XNU source has not been read; this is an explicit gap.

### Owners, order, eligibility
1. **Oracle:** a fresh producer correction task on native-c1 at d06e696f, scope `src/dream/bootstrap.rs` plus its tests and model. Build through the canonical Make worker tuple ([[decisions/d93377961365736-dream-c1-canonical-make-worker-tuple]]). Root approves pre-birth eligibility of the new hash; the 12ba approval does not carry over.
2. **Consumer (existing task):** a strict entry-line parser. The entry evidence is unknown unless the line is exact.
3. **Run:** one ordinary joined 250 ms gate, with no wrapper, warm-up or retry. A pass certifies that run only. A failure stays failed and places the gap before entry, inside recognition, or in observation.

### Falsifiers
- (a) the waiting line is present without the entry line;
- (b) e or m is reversed, or e lies before spawn_return minus the preexec-to-return interval;
- (c) the new build's load commands or initialiser set differ from d06, checked offline on the exact bytes.

## B. Git-generated signature temporary (GitSigTemp)

### Premise correction
The ICD at 98a3023e treated `{signature}` in `verify_child` as "the selected fixture signature path". That is wrong. In the real Git child, the signature operand is a **temporary file that Git creates and owns**, filled with the signature bytes Git extracts from the commit object. The fixture path never reaches the child's argv. The law binds **Git's own temporary to the pinned bytes**. It does not rewrite argv, and the path is not proof of anything.

### Source read set (authenticated, patched snapshot)
Snapshot `bboxy450-harness-dream-proof-tools/target/dream-proof-tools/runs/01M6Q80DBV75S95DWFHZWDVE2F/src/git-2.54.0`. The same hashes are in runs 01M6Q7263A…, 01M6Q76F3V…, 01M6Q7ST…, 01M6Q7VP…, 01M6Q7WS…. Builder HEAD 78391c4c. Patch `tests/fixtures/dream-proof-tools/git-2.54.0.patch` a48b3377…18b8.

| File | sha256 | Fact |
|---|---|---|
| gpg-interface.c | 191dd45b…3aed | :360 `mks_tempfile_t(".git_vtag_tmpXXXXXX")`; :363–364 write signature then close **before** the child; :371–376 argv = program, `--keyid-format=long` (:52–53), `--status-fd=1`, `--verify`, temp path, `-`; :379–381 guard then `pipe_command` (start + wait); :386 `delete_tempfile` **after** reap |
| tempfile.c | a34ef38d…79d2 | :191–195 path = `getenv("TMPDIR")` (else `/tmp`) + `/` + template; :68–100 atexit/signal unlink only for owner pid, and SIGKILL leaves the file; :360–375 delete = close + unlink |
| tempfile.h | 20c6e576…1a53a | :187–189 mode 0600 |
| wrapper.c | 788c07b3…3b04 | :438–441 six characters `[a-zA-Z0-9]` (csprng); :481 `O_CREAT\|O_EXCL\|O_RDWR` |
| run-command.c (patched) | 03bf7f11…a67f | guard nr==6, v[0]==DREAM_PROOF_GPG, fixed v[1..3], v[5]=="-", no shell/env/dir; one attempt, consumed before fork |

Q2 seam (moving, **not a pin**): `bboxy450-harness-dream-read-q2` 1fecec75 with a dirty tree; probe.c 231326 B, e77ce1fe…8e39. In it, :1174 holds the literal `{signature}`; :2318–2333 expands it to `repo/<fixture Ref>` (the defect); :2344 verify_child is never spawned directly (correct); :2023 verifier recognised by image only; :2651 one descendant. There is no argv or temp check.

### Choice
The law is the canonical ICD section "Signature temporary". In summary:
- token `{git_signature_temp}`, a matcher that is never expanded;
- GS1 at the first observation of the verifier incarnation, before ack/continue: argv via KERN_PROCARGS2 bracketed by pidversion, and the temporary via `openat(scratch_fd, O_NOFOLLOW)` checked against the selected Ref bytes;
- GS2 after leader reap: ENOENT plus the same inode, nlink 0;
- codes `signature_unexpected`/`coverage_unknown`;
- no new process, slot, DTO, wire or limit;
- a new private commands hash, so old tuples are invalid.

Alternatives rejected:
- substituting the fixture path changes the actual trace;
- allowing any temp file is no binding;
- treating the path as proof is forgeable;
- copying the bytes out adds a second store.

Residual premise: GS proves file identity, custody and content at two points, not that gpg opened that inode before GS1. That rests on the private 0700 scratch having no other writer.

### Controls (both canonical owners)
- Model: tmp {absent, git(i), planted, swapped} × argv {temp(i), fixture, other} × gs {none, bound, settled} × leader {running, reaped, killed} × case {valid, corrupt}. Broken controls that must fail: accept the fixture path; skip nlink; skip GS2; skip exclusivity; use the wrong-case hash.
- Process: real valid/corrupt with the actual argv, plus negatives: fixture-path substitution; ABA swap; symlink; hardlink; planted-before-birth; extra-after-birth; swapped Ref; early deletion via the canonical control hook; SIGKILL leftover cleaned on dev/ino match only. Foreign uid is covered by model plus review only, because it is not reproducible unattended.

### Handoff
- **Q2:** refuse the token in the expander; add GS1/GS2. Stay review_unbound pre-birth until then.
- **Builder:** corpus `gpgsig` bytes equal each case's Ref.
- **Root:** freezes the Q2 source after this change.
- **Oracle:** amends both live tasks.
