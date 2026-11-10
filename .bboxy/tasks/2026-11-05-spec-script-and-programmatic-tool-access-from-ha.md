---
type: task
title: 'Spec: script and programmatic tool access from harness logs'
verify: Validate the draft against the frozen corpus summary, source locators, current MCP bridge contract and acceptance matrix
status: done
assignee: none
role: executor
id: 01M6PQE3GGT0FMXHGDVZQ5XNMB
delegated_by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
updated: 2026-11-05T20:58:54.446Z
created: 2026-11-05T20:47:03.446Z
number: 93377961364424
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
updated_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
authorship:
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  kind: local-dev
  principal: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  device_id: 01M62F1BBJT8MW0F3YMZJ8BENM
  public_key: a8bbsPrJyl7hZraHSoUBkc76MQiRVYXx/kgj6IWVFt0=
reconciliation:
- kind: historical
  fresh_accepted: false
  previous_status: open
  disposition: implemented
  reason: Delivered the requested log-informed local draft spec; no runtime implementation. Draft is outside the watched KB because native publication must allocate its record number. Record completion of the unassigned external Executor without waking another seat.
  evidence:
  - /Users/feral/.local/state/bboxy450-term/programmatic-tools-study-20261105/spec.md
  - 80 trace files;4303 logged tool requests;349 Python heredocs; researcher history excluded
  - 'analyze.py, review_cases.py, inspect_mcp.py and validate_spec.py: exit0; ten source/call/script hashes verified'
  - /Users/feral/.local/state/bboxy450-term/programmatic-tools-study-20261105/report.md
  by: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  op_id: 01M6PR3SP6GBVJW1414RWTFZDJ
provenance:
  digest: sha256:88a31808fb3bebbfc3347d2bced141f1e15d3a65b959f8ef0b5d7e0a48d55f06
  frontmatter_digest: sha256:f2fa36a1a7c3d8780af552ea8984e30d6235a215b76e73da17fcfaea78f800b4
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: ZhxQ/9b5csmC2JZ/NtMfCRu3579XV+2kJcSvDXihRudYJuAd/C8gM40cwgesfsYSh4XH89mrbmZ7BiQJ5jfEBg==
  log: bboxy://activity/index/384128
  event:
    id: 01M6PR3T5G04JKNAG1Q9QFD5TZ
    sequence: 384128
    hash: cb0e68972a3ea2cfabcfb76257c01d3a5c5aa134abfa35ac34edbaed38db73d0
---
# Spec: script and programmatic tool access from harness logs

Direct operator request: inspect Codex and Claude tool logs, including python3 heredocs; derive an implementation-ready spec from observed needs. Private corpus and analysis live in ~/.local/state/bboxy450-term/programmatic-tools-study-20261105; draft spec belongs in .bboxy/specs/programmatic-tool-access.md. No runtime implementation or backend restart requested.
