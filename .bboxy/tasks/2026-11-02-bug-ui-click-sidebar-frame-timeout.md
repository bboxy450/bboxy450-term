---
authorship:
  kind: seat
  process_generation: '1'
  public_key: 8Z98n+ZPGXrjDmyFEtHSdpaYVDPkslEugfQLauRGmJc=
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
created: 2026-11-02T18:20:15.814Z
created_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
delegated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
id: 01M6EQV4G2SPVFSVXWSMTPC1K3
number: 5002213
provenance:
  digest: sha256:8c7e3e935429fe910bcd86e820a1255a09653840d905a9bb5a79f8bce4ed6b18
  frontmatter_digest: sha256:84d68840f0e7a9fa0176f1d7a782f7c7a89d1a7a2b2f66356e85beb964748de7
  log: .bboxy/log/2026-11-02.jsonl#439
  sig: QA8pG0LVcDpPLYkTVGfB4T7sGMwfaGjWhXWlzfMk2dN7IweaYcE0wNrpACLYKIJB8xnPH0UkNA8mXsE6RD6wCQ==
  signer: seat:01M6EQFA2G00BFT3TVPY6AMXE6
role: executor
status: open
swarm: 01M64ADP8CDYN9VX5CR31SZ629
title: 'Bug: ctl ui click toggling the Tasks sidebar section times out (UI frame incomplete)'
type: task
updated: 2026-11-02T18:20:15.814Z
updated_by: seat:01M6EQFA2G00BFT3TVPY6AMXE6
verify: cargo nextest run -p bboxy450-term --test ctl
---
**Observed (2026-11-01 16:12).** Two `ctl ui/click` failures:
- `unknown: timeout: UI frame incomplete; target=None, last=104, pending=0, effects=["SidebarSection { key: \"panels/Tasks\", open: false }=Ok(true)"]`
- the same, with `last=120` and `open: true`.

**Reading.** The click took effect (`Ok(true)`) and nothing was pending (`pending=0`), yet the acknowledgement frame never satisfied the wait. Because `target=None`, the click had no frame-sequence target to wait for. As a result, ctl reported `unknown`/timeout for a successful edit.

**Fix.** When an applied effect leaves no pending work, acknowledge on the next presented frame, or immediately when no redraw is needed. Do not wait for a target that was never set. Add a ctl test that toggles a sidebar section twice and expects two `ok` replies.

Source: [[reports/2026-11-02-error-journal-and-task-triage]].
