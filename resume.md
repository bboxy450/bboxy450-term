# Aster release resume

Signed release checkpoint: `08cc3a7f6e0e7b1a5ea936e78526313e5cc4fd7b`.
Preserved branch: `aster/release-resume-20261026` on origin.
Central handoff: https://github.com/bboxy450/bboxy450-term/issues/62

The detailed checkpoint is `.bboxy/runs/aster-release-resume.json`.
No production release app, ZIP, SHA256 file or detached signature was produced.
No owned build, app or load remains active.

## Next release step

Oracle message `38b63b68-6287-4c7e-8609-20c63f1d5871` makes Nyx's landed
0.1.0 source SHA the release trigger; the Oracle will send that SHA. The real
operator Keychain works on the signed app, so Kestrel is not a dependency.
Sable's keyboard-layout fix is already landed as `f47ef1aa`.

Use the exact instructed main SHA, `sh scripts/bundle-macos.sh release`,
`ditto` to ZIP the app, SHA256 and a detached GPG signature. Run bundle help,
ctl help and `python3 -IB scripts/release-smoke.py BUNDLE_EXECUTABLE UNIQUE_SHORT_TMP_ROOT`.
The smoke helper's syntax and hash were checked; production smoke is still owed.
The Oracle owns tags and publication. Do not publish from this worktree.

## Unfinished work and existing issues

- #54 Finder cwd startup: not implemented in this worktree. Kestrel later
  announced a separate claim; avoid overlapping it. The original brief and
  acceptance checks are in the detailed checkpoint.
- #18 R2 and #14 S4: signed candidate `fd0a8b52`, pushed to
  `aster/r2-s4-ready-20261026`. Focused R2 passed fully; focused S4 passed
  partially with heap accounting still missing. Their selective enclosing
  groups exited 1 because unrelated scripts were absent. The author run was
  interrupted at security reprioritization, so neither fix is landed.
  Evidence: `.bboxy/runs/r2-s4-night-fixtures.json` on that branch.

## Completed; do not file again

- Private script HOME: `67f5d401`.
- Constructed app/script environment: `e2398084`, evidence `10feb6f1`.
  Recorded canonical terminal verdicts: Codex 9/9 and independent Claude 9/9.
  Original tested source `cde1a811` is pushed to `aster/night-env-tested-20261026`.
- Mixed-cut Markdown corpus guard: `0e19c69a`, evidence `6c41ad73`.
- MA-8 attention controls: `a6710ffa`, evidence `e61afaef`.

Local scratchpad contains raw logs and private test-account material. Keep it
local; never stage it wholesale. All product source and resume work is signed
and preserved on the named branches. No new task or run was started for wrap-up.
