---
type: task
title: Correct the trusted prepared-bundle requirement check and reuse its built GUI
status: done
role: executor
worktree: /Users/feral/mydev/bboxy450-term-codex-rich
allowed_paths:
- Makefile
- .bboxy/reports/**
- .bboxy/runs/codex-trusted-handoff.json
verify: make install-prepared BUNDLE=target/release/bboxy450-term.app INSTALL_DIR=target/trusted-install-probe
parent: 01M6HCH1SJHDSB6SDAC2ZF1BXX
id: 01M6J7NRAC1B36W8MGNFW2GTBY
delegated_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
swarm: 01M649102FBS60QWNPZS5VAGF9
updated: 2026-11-04T14:10:58.710Z
created: 2026-11-04T02:54:39.285Z
number: 20111
created_by: seat:01M6H97SAX3GN1BYB4YG76P3P0
updated_by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
authorship:
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  kind: seat
  public_key: JlpXQZ4/Qy0yh5dcFR+iK7RgP5q83vzdNaLijybKm+Y=
  process_generation: '1'
assignee: seat:01M6HCYVGE7CJ8KK6PB20SPF3D
artifacts:
- '[[01M6J7V7VQN8V578FZB1BNWEWF]]'
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: claimed
  disposition: implemented
  reason: Historical trusted prepared-bundle fix is delivered; its old assignee and report-index registration must not keep it active. Exact Make requirement syntax and canonical reopen-prepared routing are in published/installed main ca7cde968dfee7b60937fd1a7086fe9bc5f96c15. Focused private install and wrong-identity refusal proofs were inspected; GUI-only production activation was subsequently observed. This is historical closure, not fresh full-suite acceptance.
  evidence:
  - Published main ca7cde968dfee7b60937fd1a7086fe9bc5f96c15 includes term2f56b3d6/18a51a43 and Make reopen-prepared; current installed source matches ca7cde96.
  - 'ca7cde96:.bboxy/runs/codex-trusted-handoff.json: task01M6J7NRAC1B36W8MGNFW2GTBY, sourcee22f6a5b, private task verify exit0, baseline2, wrong identifier2, destination and prepared artifact unchanged, approved Apple teamUQB3368A84.'
  - '/tmp/bboxy-oracle-takeover-01M6H97SAX3GN1BYB4YG76P3P0/refinements-final-delivery-verification.json: make bundle0, make reopen-prepared0, running2f56b3d6, exact supervisor/Logic PID-births and112 seat identities retained.'
  - Old unknown report operations01M6J7WJBJD042R157SF12Z94K and01M6J8718GYVWXB8BCQSXCDGPS were never redispatched; no fabricated indexed report or fresh verify_result is asserted.
  by: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  op_id: 01M6KEC48CVZWT7AXCS8422QNR
provenance:
  digest: sha256:504ecb5ad5ecc9f45c47119f17688d9636106ca3010a188844fbaa1c7036f40b
  frontmatter_digest: sha256:e9aad6937e965c36c29cf3ad765684055f2f5a48c9291bf04ff48a1761d31a94
  signer: seat:01M6KDR7TE4H4B4AF0PZYK0Q1T
  sig: 8Tj6R+FFGU6cz3m0cncF1uvZ8ecS0N81e3Y/ZZwgMhzBzuAlLaGgfaNEdpJoXyVuDyHp/tkv4b00zQMgOkY9Bg==
  log: .bboxy/log/2026-11-04.jsonl#114021
---
ROLE: EXECUTOR. Tiny deployment fix in your same worktree /Users/feral/mydev/bboxy450-term-codex-rich, branch codex-rich-pane-20261103, starting accepted4b8cd49cdb64ccb440213db8617a334c8e2fdd87. Preserve that source/ref and the existing trusted prepared bundle target/release/bboxy450-term.app (SourceRevision4b8cd49c). Oracle's make install-reopen compiled release and Apple-signed successfully, then failed BEFORE installing or replacing the real GUI. Exact log target/release/gui-reopen-7ld3pgkj/install.log: requirement expression treated as a filename, "No such file or directory / invalid requirement specification". Current installed app remains1affb986 and its GUI is untouched.

Fix Makefile install-prepared's codesign -R requirement syntax by using the same literal-expression form already used by the existing privacy target (leading '= '), preserving exact approved identifier/team requirement. No weakening/removing signature checks, no unsigned fallback, no key access or global configuration changes.

Expose a minimal canonical Make action reopen-prepared that runs the EXISTING scripts/reopen-gui.py / GUI_OPTIONS with the already-built BUNDLE, without compile/re-sign/repackage; reuse it from install-reopen if appropriate. Do not create a second installer implementation or new framework. Oracle needs to activate the valid4b prepared artifact quickly without rebuilding solely for a Make/report change. Keep install-unsigned-reopen/private behavior intact; source unchanged outside Make.

Needed verification ONLY: use the existing approved-signed bundle to run the task verify command, installing into the named private target/trusted-install-probe directory you own; it must exit0 with the correct identity. Also give a deliberately WRONG identifier requirement and require nonzero with no destination mutation, using the private probe only. Do not install into ~/Applications, activate/replace the live GUI or sign/rebuild a bundle yourself. No full suites, unit/formal/visual corpus, Cargo build or unrelated cleanup. Source change is one command/Make routing, not runtime/lifecycle semantics; existing private GUI replacement/rollback witnesses remain prior evidence.

Approved signed commits, exact staging, retain primary red and green output/hash summary in scoped run JSON, one indexed append-only report linked to this task (prior report19886 is evidence, not a cross-task supersedes), then task_report with exact verify command/source/actual exits. Full suites deliberately omitted by operator instruction. Oracle will inspect the tiny patch and run the existing trusted prepared-bundle GUI handoff. No recruitment, push/merge/publication/trusted live install/credentials/OS input or other worktrees.