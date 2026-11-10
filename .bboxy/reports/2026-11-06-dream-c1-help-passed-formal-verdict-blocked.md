---
type: report
title: C1 corrected admission and Hello passed; source landing blocked by native/TLC verdict mismatch
task: '#93377961364376'
author: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
created: 2026-11-06T21:34:21.138Z
commits:
- 8d9cb55f2cf2c5edff4667aeba62b6ac52b7af93
- 2c16ef492a28a66b141d21010d9f764ac4c241bc
- f6d0443b87333a54f394f5e9787607e371708d83
verify:
  command: make test-formal FORMAL_REUSE=1
  exit_code: 2
supersedes: '[[reports/2026-11-06-dream-c1-keychain-file-empty-target-blocked]]'
id: 01M6SCHC54KQAHWSNCYA2CGWG7
updated: 2026-11-06T21:34:21.138Z
number: 93377961366128
created_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
updated_by: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
authorship:
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  kind: seat
  public_key: vTy8kxzALPVGVJxPquLZelvl5D6TNzgsirkLE/DA5ek=
  process_generation: '1'
provenance:
  digest: sha256:983aeae7abf90f029671f96d82a625c1853fd23082ca353cc6d5725c668f8f68
  frontmatter_digest: sha256:0fa6d4ed590732f4c88e9517b1d8ad3a27e9646d88af82a006bebbbfadedb703
  signer: seat:01M6RZZY4FR22NC0Q6DAZGHYKC
  sig: /h3gGQdXVVCucG1OxZEa9LzGSBo+wM/+QK/L9R3QWFTUcBeco7z9gMDfRnU6ugjtXxpcLqBfth6l0mvxGHNfAw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/149880
  event:
    id: 01M6SCHE5D9SZRDH8MTPRAVJQ6
    sequence: 149880
    hash: 081dfb6fbd279be6f788231c473a837e29dd3828f7f7263a04b6e1012167371d
---
Source landing is BLOCKED by a hard native/TLC verdict disagreement. Current clean signed source f6d0443b87333a54f394f5e9787607e371708d83, feat/dream-term-c1, /Users/feral/mydev/bboxy450-term-dream-c1. Nothing has been pushed or installed. This supplements [[reports/2026-11-06-dream-c1-persist-entry-landed]] and the intervening blocker reports.

Implemented and committed:
- Corrected decision66076 admission from --version to --help. The private first-launch/2 receipt and claim key include the exact argument, file path/device/inode/hash/mtime/ctime. Failed historical /1 version claims stay byte-identical; known reaped exit1 history permits the authorized new help namespace, unknown legacy owners refuse. Failed/unknown help attempts remain non-replayable. Runtime rejects old schemas/flags. Native7/7 and private profile controls passed; strict check-rich-host passed. The admission-before-Start model retains the historical failed-version state and its broken Start control.
- The operator-authorized pure tests cover keychain facade64KiB bound, file Secret bytes/redacted Debug, Windows unsupported marker without any Windows API/cfg gate, desktop action parsing, fixture knob encoding and binary default selection. No Keychain/OS prompt, new dependency, production behavior or general empty-target exemption.
- Latest operator order01M6SB05JX1VX66GF62XQ88B08 is implemented in signed f6d0443b: ONLY actual_private_process_rejects_blank_invalid_utf8_unknown_duplicate_and_oversized_frames has the specified ignore annotation, re-enable task93377961366081. Its assertions and original limits are unchanged. No quarantine row/task copy was added. landing.py remains untouched by this task.

The ONE actual corrected trial:
make test-dream-c1 on signed8d9cb55f2cf2c5edff4667aeba62b6ac52b7af93 exited2. /tmp/dream-c1-help-one-joined-run.log; evidence /var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/bboxy-dream-c1-gate-3tubkf94/.
--help admission PID42152 exit0/actual_reap=true,295519000ns wall,471stdout bytes, no stderr. Exact approved source2ce4359df034405043d419fe550b0f6cafe7887e/compiler-vV SHAe003fd3d385fcd0de16bc6b049124efb20207831dd810fd4ab39ff079bbb12a5/binary2b85c7114545f35a62842115ae91e08e1a22fbfcd24dfd27e3aa45fa1d3837cb. Canonical Cargo replaced the executable inode before this admission (179931016 to180209068); the admitted final file stayed exact through execution. No manual inode/claim reset.
Ordinary source-free Hello PASSED the original250ms clock. PID42175 exit0/actual_reap=true/settled=true/no quarantine, one128B permit, explicit null context, no Start/inspection. Entry spawn-return signed bounds4293000..6881000ns; observation uncertainty2588000ns; entry-to-prewrite50000ns; both entry/waiting observed before the original deadline. Auxiliary kernel counters remain raw/unverified.
Runtime phase then FAILED before the producer Store/export job could start: pinned approved_runner() requires bootstrap-support.txt and hashes.txt beside DREAM_C1_RUNNER, while term supplies its private .dream-c1.json. Producer tests/dream_c1.rs:1218, command-02.log, test exit100. Oracle01M6S8S6276F1C2HJ7H13YQYMR explicitly covers this source-only landing exception: run stays FAILED, runtime unapproved; no artifact bridge or continuation in this task. I did neither. Full four-job/resource/frame/latency acceptance remains unproven.

Hard gates:
- make test-formal-contracts exit0,43passed plus strict lint, /tmp/dream-c1-help-formal-contracts.log, on8d9cb55f before the upstream-only rebase.
- make land on signed/rebased2c16ef492a28a66b141d21010d9f764ac4c241bc exited2/internal77: affected all-target Clippy passed, private wake-up suite54/55passed; the now-ignored malformed-frame fixture missed its unchanged250ms window before its script/stderr began. /tmp/dream-c1-final-make-land-help.log.
- A material private-script help-admission experiment still failed55/56: EOF peer CPU555458ns/wall206934417ns after help admission176446708ns. It was ineffective and is NOT applied. Full patch /tmp/dream-c1-private-script-admission-experiment.patch SHA f0b3fee85764191c6cf01cd825629d696341672704951726f1ef9a4ca7fe3593 and manifest .json; log /tmp/dream-c1-private-help-transport.log SHA84764ed418318d525f1c165c30da8e639c554bff6984bbedf3838ace50b8ce6c. Own experiment was reverse-applied from the saved patch; all predecessor/committed work is preserved.
- Oracle's canonical quarantine attempt was refused BEFORE an isolated test or write: quarantine_task_not_unique93377961366081, /tmp/dream-c1-quarantine-admission.log. Root subsequently superseded quarantine/task-copy with the exact single-test ignore above.
- The independent current-source formal refresh on clean2c16ef49 then FAILED: make test-formal FORMAL_REUSE=1 exit2, /tmp/dream-c1-help-current-formal.log. Exact error: native/TLC verdict mismatch: BrokenHarnessDreamC1QuantumDebtAbaEffect: safety/liveness. No verdict relabel, model/cap/control deletion, backend skip, receipt fabrication, investigation or repeat followed. Raw model/backend artifacts remain in target/formal. Earlier source-bound green671/34/251 receipt4c239f0d5285ddecfd79ae84c01c38f6b0f746a5aa0ca3d0839946887d3afcc5 belongs to b2c20d82 and does NOT certify this failed refresh or current f6d0443b.
- The ignore was held until the running source-bound formal attempt ended, then committed exactly as ordered. make land has not been repeated against a known unresolved formal mismatch. No current-source formal-green or successful landing is claimed.

Exact remaining disposition: resolve the native/TLC verdict disagreement before canonical make land may pass. One-line blocker DM01M6SC8S0FPVR2KP9547KCAC9E was durably accepted immediately, as the operator requested; model consumption is not claimed. No additional ordinary run, suites or experiments are authorized/performed after this hard formal failure.

Known limits remain recorded in the full report: cold/warm Status1.7532575s then1.458710541/1.490842375s; pages3.1525905s then2.800326916/3.027203167s, with the exact per-phase CPU/wall table in /tmp/dream-c1-cold-warm-phase-summary.json. Existing1ms units/9x cuts remain; these latency limits are a non-blocking follow-up perRoot. Installed-bundle admission must eventually use private Application Support state keyed by final file identity; no bundle sidecar/recipe change or runtime installation seam was added (Root01M6S5792H2KG3BNGR3J5VADT8 explicitly defers it). Runtime remains unapproved.

Owner-safe headroom: lsof proved no holders, every removed compiler file was stat/SHA rechecked and recorded; all executable workers, formal tools/graphs/receipts/logs/source were retained. Own cold duplicate library outputs removed3225635717bytes (manifestSHAf3cf9bed5d0522de44ce4f0abc3ec282e702f6a774ade381a95deb2946de488c),1692008776bytes (manifestSHA57cf6f6d89e9b1f13f41e82a9f90ee176c5dcc1ad988165f1444455c5dd4857c), and completed consumed optimized fixture libraries1126525563bytes (manifestSHA471ee9f38690d95228e3e667836ad62579e000ae16aef0cdb32582cbd6179bb4). Both actual worker hashes were checked unchanged. No peer targets/worktrees or immutable evidence were pruned.

Native reports are append-only private-KB checkpoints, not shell-written/force-added source KB records. Task ownership works; task_report remains for an actual successful landing. The predecessor's unknown report operation01M6RYQ9VK9JSRHVTNMYH7PW0P remains unknown/unretried.