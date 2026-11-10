---
type: report
title: Revised Phase A witness and exec-path; original preserved
task: '[[tasks/2026-11-05-dream-proof-tools-provisioner.md]]'
author: seat:01M6W6K5SWZB5AEBAAAA73FF4M
created: 2026-11-09T16:39:56.840Z
commits:
- 33c7a896bedd80968a6a1dafcccb64eec0fbe605
supersedes: '[[reports/2026-11-09-dream-proof-tools-shim-phase-a.md]]'
verify:
  command: /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109-b/bin/git --exec-path
  exit_code: 0
id: 01M70JWF9F0TYD0XRRMCT5NBCE
updated: 2026-11-09T16:39:56.840Z
number: 93377961366474
created_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
updated_by: seat:01M6W6K5SWZB5AEBAAAA73FF4M
authorship:
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  kind: seat
  public_key: sgvX14zJzDwL2mgTPZPL+f9/hzNh69ZAiJTfrsi+mqY=
  process_generation: '1'
provenance:
  digest: sha256:00de0fd8265059483c1b3b373f9a2a6d56a6ac0011b3a737c15bf583c54ff5f4
  frontmatter_digest: sha256:27bca2987671dcabda7fc38869797fd9a9a6102f0a6447b6de7482b5c0ca0a5c
  signer: seat:01M6W6K5SWZB5AEBAAAA73FF4M
  sig: Ov1gv1ur6AS4+Vx5gLrBoguI6rUYLBUCPhVWqqDaVYvIRtFPaPNFmWfy598FVnxWdam8E4IiMWMXBZC3FrB3Dg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151068
  event:
    id: 01M70JWGRJH0TFE30PJXSPVHXT
    sequence: 151068
    hash: bf23fe5293c8ebc41be8626e9ac195ddde88421289a77bc2a834406a5630d626
---
The authorized revised Phase A witness is complete. One additional Git was built in the fresh -b directory, and its only execution was the authorized --exec-path diagnostic (actual exit0). The first witness remains untouched. The shim executor received the revised path/hash and exact exec-path stdout. Code33c7a896bedd80968a6a1dafcccb64eec0fbe605 is unchanged and signed; runtime_approved=false.

This report records BOTH witnesses and supersedes the active handoff in [[reports/2026-11-09-dream-proof-tools-shim-phase-a.md]] (#93377961366470); it preserves that first build's evidence. No further witness build is authorized without a new task ruling.

## Authority and preserved scope

Task [[tasks/2026-11-05-dream-proof-tools-provisioner.md]], etag2dd5db6d49cf3776c852b8bfc48043b2b1bcaadfb071863963df0234f570622a, section "ORACLE RULING (2026-11-09): one revised Phase A witness build plus an --exec-path diagnostic" explicitly supersedes the earlier one-build limit for exactly this additional build. It specifies:
- fresh target/dream-proof-tools/shim-witness-20261109-b/;
- the revised supplied witness-current/dream-exec-shim target from message01M70J9H1FSP0CFG9500NZ9PX6;
- preserve the first1955c95c… output;
- allow only new-Git --exec-path, with no fixture/probe/qualification/candidate/source-lock/receipt change.

Oracle messages01M70JE2G85ZJEH51P3KD0GQER and01M70JJXG9BWR7254STWY4Y951 link the ruling and recorded scope. Implementation proceeded after refetching that amended task. No source change or third build occurred.

Own root W=/Users/feral/mydev/bboxy450-harness-dream-proof-tools.
A=W/target/dream-proof-tools/shim-witness-20261109.
B=W/target/dream-proof-tools/shim-witness-20261109-b.
E=W/target/dream-proof-tools/shim-phase-a-20261109-b.

## Both retained witnesses

| Witness | Git path below W | SHA256 | Outcome |
| --- | --- | --- | --- |
| Original | target/dream-proof-tools/shim-witness-20261109/bin/git |1955c95c2daa1ea5ebb8914fd32d0efe2f89d8fec6c64b1982fded31a1f0da81|build0, retained; not executed by me|
| Revised | target/dream-proof-tools/shim-witness-20261109-b/bin/git |5981ccf53ca713c4c80030f2a872f31696e00a03729e6fc463e31a8f17cd0265|build0, --exec-path0|

Both files are3766096bytes/mode0700. Original inode189483646 remains unchanged; revised inode189515452 is distinct. E/first-witness-before.json and B/witness-build-complete.json preserve the first binary, recipe, source-ready, completion-record and GIT-CFLAGS length/hash/device/inode/mode. All26 historical component ledgers also remain unchanged.

The original compiled target was the supplied frozen preparation path:
    /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/prepared-witness/dream-read-c-preparation/compile-t01d4h8x/dream-exec-shim

The revised compiled DREAM_PROOF_GPG target is exactly:
    /Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/witness-current/dream-exec-shim

The revised target is the executor-supplied staging PATH literal. This build does not approve mutable staging bytes as an INPUT70 shim artifact. No shim was executed by this work.

## Build and exact define evidence

Fresh directories0700/data0600 were created exclusively; no original output, source tree or candidate was reused for mutation. Seven cached source archives were independently authenticated before fresh extraction. B/authenticated.json has7auth_exit0 rows. The same six patches passed exact pre/postimage checks and the complete source snapshot was retained.

B/logs000–005 are patch commands;006–007 locate compiler/SDK;008 is the ONLY additional make -j2 ... git invocation;009–010 are otool metadata reads;011 is the ONLY revised-Git execution. All12 actual exits are0, and retained stdout/stderr hashes match their exit records. Each build command uses the existing nice10/bounded explicit environment. Pre-build disk gate recorded16425448KiB>8388608KiB.

Exact CFLAGS include:
    -DDREAM_PROOF_GPG='"/Users/feral/mydev/bboxy450-harness-dream-read-q2/target/dream-exec-shim-decision-witness/witness-current/dream-exec-shim"'
B/logs/008.command.json contains the full actual make argv/cwd/env. B/src/git-2.54.0/GIT-CFLAGS926bytes SHA2560e78b7ad04265c4df4d5f0cf446dc76e7d70f3ed8bf6258d895411874e6e00a0 retains that same define. The exact NUL-terminated shim literal is present in the linked revised Git; its unused B/bin/gpg default literal is absent. This is build binding evidence, not a signature or lifetime execution witness.

Unchanged common inputs:
- Signed code33c7a896bedd80968a6a1dafcccb64eec0fbe605.
- Recipe SHA256ad3fd381408f358edf6417bbaa2988c55d50d30d8d8ddf4ddd03c2727a96d4fc.
- Git patch SHA256a48b33776ae5a88308e096ea62640487706e360c374ec7eaf64d50adf47818b8; all other guards unchanged.
- The previous69unit-test pass applies to the identical recipe. It was not rerun after changing only the supplied build target. No lifecycle/model action changed.

Final actual git verify-commit: Good Boxy key9B07E849480C68328CA20684844ECC0587DE7AB0/exit0; git diff --check0; git status --porcelain0/empty. There is no new code commit to manufacture.

## Exact diagnostic result and handoff

Executed once with the bounded private environment recorded in B/logs/011.command.json:
    /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109-b/bin/git --exec-path

Actual exit0; stderr empty. Unmodified stdout, including one trailing LF:
    /Users/feral/mydev/bboxy450-harness-dream-proof-tools/target/dream-proof-tools/shim-witness-20261109-b/libexec/git-core

B/logs/011.stdout120bytes SHA2563cc8afae3ae2c884700923b489bd2ca55859272ac50a0400cef4d751c581f7d2. B/exec-path.json retains stdout, stderr, exit and hashes. This prints the compiled exec path; it does not establish that directory is installed or qualified. No fixture, Git proof form, shim, native probe or qualification was run.

Result DM01M70JRXTHK8VPHXNS19TQKG27 was sent to seat01M70FJGXV7W40W80200Z0TDN5, replying to its corrected-path request. It includes the new Git absolute path/hash, exact exec-path output, target/define evidence, patch/recipe hashes and second-and-last-build limit. Delivery evidence is native-queued, native_id01a1c12c-77d4-71d3-9d86-79f4acef90bf; model consumption remains unverified. No ACK wait or repeat send was used.

## Evidence and remaining boundary

E/handoff.json8977bytes SHA25690eb46d27108d3a8982e33babf316b681e984edbc25676ebac96f4441aff0273 binds actual build/diagnostic exits, original/revised artifacts, exact logs and unchanged code checks.
B/witness-build-complete.json2798bytes SHA256df7db1738b2b1c075d530fe91e22c6356f93291e55672e03d2b8678b91cd14c3 binds the compiled target, both witness identities and preservation facts. Independent read-only review is separate from these retained command records.

Phase A has exactly two authorized builds total: original plus this revised build. Source-lock fixtures, old candidates, all prior runs, receipts, budgets and runtime laws remain unchanged. No full canonical command was rerun; INPUT69's actual exit2/failed qualification remains historical. Phase B installation at candidate/libexec/dream-exec-shim and new qualification remain deferred to the INPUT70 freeze.
