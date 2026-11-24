---
allowed_paths:
- scripts/tool-usage.py
- Makefile
assignee: seat:01M8782S84QTEHAESNKCMXH5XG
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8ZJ1pn6ymQlhdn20cJBq2Rono7S6d/fLcgc3FK45CmU=
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
created: 2026-11-24T17:00:36.155Z
created_by: seat:01M8764DX3P3XKP4DF8R2DANYR
delegated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
id: 01M87813BQGVG3XZRGFW4ZC5J2
number: 1794292861627908
role: executor
status: claimed
swarm: 01M7P04CW1T6KY8CMSS6DQM12Q
title: 'make tool-usage misreads adoption: read Codex code-mode records, stop counting log plumbing as code reading'
type: task
updated: 2026-11-24T17:01:34.063Z
updated_by: seat:01M8764DX3P3XKP4DF8R2DANYR
verify: make tool-usage DAYS=2 && python3 -I scripts/tool-usage.py --days 2 --json > /dev/null
provenance:
  digest: sha256:32224bfa705e088c98035e111c1dfbc5d3a9a819007beccd6153cae16a92b19b
  frontmatter_digest: sha256:c757eab1514a6562fc75ba4d91664ee643cf30ae9f7790de5ee2e261ffd533c4
  signer: seat:01M8764DX3P3XKP4DF8R2DANYR
  sig: nFHqBmwYdvsqVojhpHlAOS+/ZpRTybgb/wBvjlNV2NhjDpawGkrq266BQ7q3qVh8RWVS+JfZWbz8z4dOe0QEDg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1833
  event:
    id: 01M8782X5HQW1HBBKYV1YW6WAE
    sequence: 1833
    hash: 6e547962dbde6705cbbf79183e83c71fe8c0ec78b4da643e495fd5822bce8a34
---
Our stand: subtract before you add; report what you removed.

## Evidence (2026-11-24, Oracle Vesper Greywind)
`make tool-usage DAYS=1` reports code reading "shell 212, bboxy_code 4 (1.9%)". That number is wrong in two ways:

1. **It misses where Executors read code.** `scripts/tool-usage.py:65` reads only `~/.bboxy450-harness/sessions/*/python/*.json` (python_execute records). Codex Executors work in Codex code mode: `~/.codex/sessions/**/*.jsonl`, `payload.name == "exec"`, whose JS `input` calls `tools.mcp__bboxy_code__search(...)`, `tools.mcp__bboxy_code__read(...)` and `tools.exec_command({cmd:"rg ..."})`. Over the last 2 days those records hold 3,801 exec calls with 146 `bboxy_code.search` + 113 `bboxy_code.read` against 200 leading `rg`, 85 `sed`, 30 `cat`, 14 `nl`: adoption is roughly 40%, not 2%. Direct MCP calls to bboxy_code (Claude seats calling `mcp__bboxy_code__*` outside python) are not counted either.
2. **It counts log and pipeline plumbing as code reading.** `CODE_READING` (`scripts/tool-usage.py:24`) includes `tail`, `head`, `ls`, `wc`, `awk`, and counts every program in a pipeline. The python_execute rows from the last day are mostly `... | tail -30` over build logs, `df -h | tail -1 | awk`, `pgrep | sort | uniq | head`.

## Do
Change only `scripts/tool-usage.py` (and its Makefile help line if the wording changes).
- Also read Codex code-mode records under `~/.codex/sessions` (add a `--codex-root` default `~/.codex/sessions`, same `--days` mtime filter): count `mcp__bboxy_code__<tool>` occurrences as `bboxy_code.<tool>`, and the program names of each `exec_command({cmd: ...})` string through the existing `programs()`.
- Count only the *first* program of a pipeline as the reader, and narrow `CODE_READING` to programs that read source: `rg`, `grep`, `git grep`, `sed`, `cat`, `nl`, `find`, `fd`. Drop `tail`, `head`, `ls`, `wc`, `awk` from the code-reading group (they still appear in the top list).
- Print the code-reading share per source (bboxy python, codex) and total.
Keep it one stdlib script, no new files beyond a small test if you add one.

## Accept
- `make tool-usage DAYS=2` exits 0 and prints a codex section with nonzero `bboxy_code.search`/`bboxy_code.read`.
- `python3 -I scripts/tool-usage.py --days 2 --json` emits valid JSON with per-source shares.
- Signed commit landed on origin/main via `make land`; report lists what was removed.
