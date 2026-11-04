---
type: report
title: Reclaim accepted startup build output without losing evidence
task: '[[tasks/2026-11-03-reclaim-completed-startup-build]]'
commits: []
verify:
  command: test ! -e target/debug
  exit_code: 0
author: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
created: 2026-11-03T22:07:12.005Z
id: 01M6HQ7D40GY40SDBPPZK80GCZ
updated: 2026-11-03T22:07:12.005Z
number: 19013
created_by: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
updated_by: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
authorship:
  signer: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
  kind: seat
  public_key: 7az818RFHIs+iUEUkggIsxBJE6C9BR33qwcjDZteyJo=
  process_generation: '1'
provenance:
  digest: sha256:1408f58690c159d99132238520dd8a66af3eb318d18015ecbd2d02233a260665
  frontmatter_digest: sha256:b53a0e7e34dbc58a729be78e01805b582122088abc43c6ba8a8776d05ba50371
  signer: seat:01M6HGCVKFA0E7CEGE8E7Q5D5P
  sig: XO6nSx6D3mu8dh2T945TR4pSOX9+zFN4xKKcKsxq7tfh/11NcvWlPhqD3NFaxazHsW+hWQje4G2wdTkSwx+WCA==
  log: .bboxy/log/2026-11-03.part-86213.jsonl#105111
---
# Completed startup build output reclaimed

Task: [[tasks/2026-11-03-reclaim-completed-startup-build]] (#18977). Worktree: `/Users/feral/mydev/bboxy450-term-startup-input-fix`, branch `startup-input-fix`.

Before deletion, HEAD was accepted commit `7eb6ead826ff104acf52f1f9bc383e4575ec848b` and `git status --porcelain` was empty. Neither target path was a symlink, and no tracked file occurred under target/debug. The final process guard found no build/test cwd in this worktree; `lsof -nP +D /Users/feral/mydev/bboxy450-term-startup-input-fix/target/debug` exited 1 with empty stdout/stderr, confirming no open files there.

Only `rm -r -- target/debug` executed for deletion: exit 0. Required verification `test ! -e target/debug`: exit 0. No build, suite, push or install ran.

| Measurement (KiB) | Before | After |
| --- | ---: | ---: |
| `du -sk target/debug` | 6,732,184 | absent |
| `du -sk target` | 6,732,252 | 68 |
| `df -k .` used | 428,614,516 | 422,240,840 |
| `df -k .` available | 23,340,912 | 29,714,584 |

The removed tree measured 6.42 GiB. Filesystem free-space change is a shared-machine observation. A SHA256 manifest comparison of all eight retained target files passed (exit 0), preserving every startup-input log and other target file. Source/Git state, reports, peer outputs, sccache and formal evidence were untouched by deletion.

An initial cwd guard exited 1 because it classified this seat's live bboxy wrapper as a test; the corrected guard distinguishes build processes and checks actual target/debug open files. A forced-rm command was rejected before execution; the guarded non-force command above completed.

The record-only containing commit carries this report's trailer; exact SHA, final verification exit and clean status are supplied through task_report. No source recompilation or additional test is required by this cleanup brief.
