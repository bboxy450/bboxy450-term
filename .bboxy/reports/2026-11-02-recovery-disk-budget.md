---
author: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
authorship:
  kind: seat
  process_generation: '2'
  public_key: 8G8gIL1MJqmoRkKikaIqqCqCFW1VHfzquBV0bXH7xs8=
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
commits:
- d197fe3e6b5d935ee44ba876ec2eda1295a3204e
created: 2026-11-02T20:13:57.791Z
created_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
id: 01M6EYBBJGDJ03S1FRZCN5RDKB
number: 5002220
provenance:
  digest: sha256:7b5b7f784e5987548516bd94d3fca068b9882a5a47b281e60b58804625a475c1
  frontmatter_digest: sha256:d7fb1756d813032d26ae608e0956c75603bf2c58baa6f5e39b6086b9fbbd8fd5
  log: .bboxy/log/2026-11-02.jsonl#4477
  sig: GOLoiBOSl9Uhjty54M+tqG72sb4jO3yOPckpU16QGwpC7qfKIgMAPjeogV58Z0/Raj9Q+bFd5pUkFD6Oe7AQAA==
  signer: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
task: '[[tasks/2026-11-02-recovery-disk-budget]]'
title: Old worktree cleanup and bounded recovery disk usage
type: report
updated: 2026-11-02T20:13:57.791Z
updated_by: seat:01M6EW8HN9QTQJ7A7GD1ZYMWYR
verify:
  command: make test-recovery-disk
  exit_code: 0
---
ROLE: SOLO. The operator requested removal of worktrees at least 14 hours old, authorised the identified old app/test/server process stops, and requested a smarter disk budget for lean landing. Implementation worktree: `/Users/feral/mydev/bboxy450-term-chat-mute`, branch `fix/chat-mute-notices`. The preceding signed chat fix is `d197fe3e6b5d935ee44ba876ec2eda1295a3204e`.

The recovery fixture's former `check_disk` required 10 GiB free; it did not allocate 10 GiB. Its child stdout already uses `Stdio::null()`. The ring, tick journal, SQLite state and checkpoints are the recovery evidence and remain real files.

Changed files:
- `crates/bbt-mux/tests/recovery.rs:317` defines 128 MiB plus 1 MiB per seat of fixture headroom (132 MiB at four seats, 178 MiB at fifty). `fixture_bytes` measures retained logical file lengths recursively without following symlinks and tolerates disappearing atomic-rename entries. `check_fixture_size` rejects observed growth above the budget. Checks run at admission, every second before the controlled crash, and at completion; the final usage is printed. These are observations and a test guard, not an OS quota or proof of every transient peak.
- `recovery.rs:360` creates a 4097-byte sparse file and requires rejection under a 4096-byte limit. This exercises real filesystem metadata without allocating a large fixture.
- `Makefile` adds `make test-recovery-disk` using nextest for the normal recovery suite and the private 50-seat fixture.
- `LAND.md:55` replaces the blanket 10 GiB requirement with build headroom and the explicit recovery budget.
- The new task/report records retain the brief and evidence. No production persistence, lifecycle, model transition or recovery assertion was removed; SIGKILL, exact replay/screen comparisons, byte-debt bounds and child reaping still run.

| Command | Actual exit | Result |
| --- | --- | --- |
| `make test-recovery-disk` | 0 | Four normal tests passed, including the sparse growth control; fifty-seat recovery passed in 8.597s. Four seats retained 28,142,039 logical bytes (26.8 MiB), under 138,412,032 bytes; fifty retained 45,049,839 (43.0 MiB), under 186,646,528 bytes. |
| `make test-chat-mute-notices smoke test-licenses` | 0 | Projection/history test passed; private native chat/restart passed in 6.197s; private unsigned smoke passed its 10.005s idle probe and orderly reaping; license checks covered 33 crates and fixtures. |
| `git diff --check` | 0 | No whitespace errors. |
| `git worktree remove --force` over the reviewed old-worktree inventory | 0 for all 25 | All 25 aged linked worktrees found across 51 repositories were removed; a subsequent registered-worktree scan found zero aged linked worktrees in that inventory. |

Cleanup kept the primary checkouts and younger worktrees. Existing branches were retained and every removed HEAD also received a `cleanup-preserved/2026-11-02/...` ref. Local runtime/data directories were moved to `/Users/feral/worktree-salvage/2026-11-02` before deletion. The three old bboxy fixture owners were stopped after exact-identity checks; six named Node/esbuild processes were killed with SIGKILL under the operator's instruction. No unrelated process was signalled. Final cleanup measured 12.51 GiB available at that moment; free space changes with other builds.

Raw audit: `/tmp/nyx-old-worktrees-cleanup.json`, SHA256 `d965b53dd32296ea6e524dbd1b6d7d49645a92118530e0aa1288b79deb0e7edf`. Recovery log: `/tmp/nyx-recovery-disk.log`, SHA256 `24bac2980bc8edcf82d1ea2ba0914f1a479cba74d8c362ac0717b76c53703d09`. Chat/smoke/license log: `/tmp/nyx-final-checks.log`, SHA256 `8567de2422b06197b3d7b41966dbe35e912322361b08a8a030288bb5202f3496`. Raw logs, fixture accounts, captures and salvaged files remain outside Git.

Code-intelligence callers confirmed that `check_disk` is used only twice by this recovery test. No 967-seat calibrated performance claim is made; the native runs used four and fifty seats. UI geometry/reference code is unchanged, so parity is not selected. The clean-source formal refresh, pinned signature checks and canonical `make land` run after this report is committed; the final task_report records their actual result and landed SHA. The previous chat report remains an immutable historical record.