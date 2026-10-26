# Aster release resume

Production smoke completed on the Oracle's signed 0.1.0 archive.
Central handoff: https://github.com/bboxy450/bboxy450-term/issues/62
Evidence: `.bboxy/runs/release-app-file-smoke.json`.

## Latest result

Oracle message `3014d5fd-d7a0-419e-bbd7-8ec7d3edb649` superseded the duplicate
build with a smoke-only task. The duplicate build had already ended when that
message was read. No duplicate ZIP or detached signature was produced.

Oracle archive: `/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/3544d975-706b-4ce4-ba8c-d9734c0db856/scratchpad/out/bboxy450-term-0.1.0-macos-arm64.zip`
SHA256: `f241b55e7f55838b812ea4fb57bac980ac63495358595e9743dbb3c470ff7c8c`
Source reported by release owner: `d02f6135d734bfe812aa071b4621b203ead360bc`.
Bundle version: 0.1.0. Archive checksum, machine-key GPG signature and strict
codesign verification passed before executing it.

Production smoke: exit 0 in 2.201 seconds, file custody,
headless with zero windows, one real zsh pane drew the nonce at frame
29, orderly stop, all five recorded PIDs absent.
Raw log: `/Users/feral/mydev/bboxy450-term-exec4/scratchpad/terminal-t5-20261026/oracle-release-file-smoke.log`
Private run: `/tmp/aster-oracle-smoke-77243b981ab1` (contains private test custody; keep local).
No owned test app remains. Oracle owns tags and publication.

## Other preserved work

- Original shutdown/release checkpoint: `3ef861b6` on
  `aster/release-resume-20261026`; detailed historical state is in
  `.bboxy/runs/aster-release-resume.json` on that branch.
- #18 R2 and #14 S4: candidate `fd0a8b52`, remote
  `aster/r2-s4-ready-20261026`. Focused rows passed; S4 remains partial for
  independent heap accounting. Selective enclosing groups exited 1 because
  unrelated scripts were absent. Author interrupted; fixes NOT landed.
- #54 Finder startup was not implemented here; Kestrel's separate cut is in
  the release source. Do not duplicate its edits.
- Original environment-tested source `cde1a811` is preserved on
  `aster/night-env-tested-20261026`. Accepted fix `e2398084`, evidence
  `10feb6f1`: recorded Codex 9/9 and independent Claude 9/9.
- Markdown corpus guard `0e19c69a`, evidence `6c41ad73`; MA-8 `a6710ffa`,
  evidence `e61afaef`. These are already landed.

Local scratchpad contains raw logs and private accounts; never stage it wholesale.
No new feature or publication was performed for this smoke task.
