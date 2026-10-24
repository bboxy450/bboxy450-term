---
id: 01M5JQE3ZY6KYN1P0NA2M75X0E
number: 140
type: invariant
title: every commit on main carries a good signature
status: active
check: test "$(git log --format='%G?' HEAD | grep -c .)" -gt 0 && test "$(git log --format='%G?' HEAD | grep -vc '^G$')" = 0
cost: an unsigned commit breaks the chain from artifact to key; releases inherit the gap
links:
  - { to: "[[provenance]]", rel: constrains }
  - { to: "[[identity]]", rel: constrains }
created: 2026-10-22T21:14:24Z
updated: 2026-10-22T21:14:24Z
---
# Every commit on main carries a good signature

**Property.** Every commit reachable from `origin/main` verifies with a good
signature (`%G?` = `G`) from the machine key `844ECC0587DE7AB0`. The provenance
chain is signed commits → signed tags → detached signatures on published crates
([[provenance]], [[open-source]] §3); the first link is this one.

**Check.** `test "$(git log --format='%G?' HEAD | grep -c .)" -gt 0 && test "$(git log --format='%G?' HEAD | grep -vc '^G$')" = 0`
on the handed-off commit (HEAD of the clean worktree the local CI runs in, which
is what main becomes) — exit 0 held, exit 1 at least one commit unsigned
or with a bad signature. The format is quoted because `?` globs in zsh; the
first clause refuses an empty log, so the check never passes on nothing; list them with
`git log --format='%h %G? %an %s' HEAD | awk '$2!="G"'`.

**Cost of a violation.** An unsigned commit on main is an artifact nobody can
trace to a key; everything built on it, including a crate release, inherits
the gap. Fix the config, never paper over with `--no-gpg-sign`.

**Evidence.** `398 of 398 commits G at a9ab5c1` (Architect, 2026-10-22).

Owner: Corvin Duskmantle <ARCHITECT>.
