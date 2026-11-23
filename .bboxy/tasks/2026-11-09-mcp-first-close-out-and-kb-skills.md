---
acceptance:
- task_accept refuses a report whose commits are not reachable from the integration ref
- task_close moves accepted to done, refuses other states, and hints seat_close then workspace_release
- workspace_audit lists git worktrees, records and task status with orphan and stuck findings
- skill KB type at skills/<name>/SKILL.md served by skill_list/skill_read with origin kb
- skill_search returns matching skill URIs across builtin, kb and plugin skills
- .agents/skills symlinks to .bboxy/skills in the repository
- AGENTS.md close-out uses task_close and workspace_release
allowed_paths:
- crates/bbt-mux
- crates/bboxy450-term/tests/formal_contracts
- verification
- AGENTS.md
- .agents
- .bboxy/skills
assignee: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
authorship:
  kind: seat
  process_generation: '1'
  public_key: u1uV2w0O7KVM2p/s9nTh9h0So2DmefovlUQiVjF0K3M=
  signer: seat:01M85CCK3PTGF5WS0SZ6AY8TGG
closed:
  by: seat:01M85CCK3PTGF5WS0SZ6AY8TGG
  op_id: 01M85CDK8N4F5HYQ86Y448NXPD
created: 2026-11-09T18:51:52.543Z
created_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
delegated_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
id: 01M70TE2D8CVV40Y5EQGMMP8SP
number: 93377961366485
report:
  commit: f6dee9b38d3e98d8c2667bb3c3f68d53fea72971
  op_id: 01M70WQD6HD5P8XWDQ6DVFFW8J
  report: '[[reports/2026-11-09-mcp-close-out-kb-skills-remote-link.md]]'
  reported_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  summary: 'Landed 69a2eb19b and f6dee9b38 (both signed) on origin/main. 69a2eb19b: task_close (accepted -> done), with hints to seat_close, then workspace_audit, then workspace_release; acceptance refuses commits not on origin/main; workspace_audit tool; workspace_prepare fixed for claimed tasks; KB skill type, skill_search, the .agents/skills symlink and the AGENTS.md close-out text. f6dee9b38: remote supervisor link cut 1 (frames; Yarn RemoteDelivery model checked natively and by Java TLC, 52 states each; runtime Outbox/Inbox/FileJournal with a witness test). The verify command exited 0. Not run: make land or the full suite (operator bypass). The installed app needs a rebuild before the new tools are live.'
  verify_revision: 0
role: solo
status: done
swarm: 01M6ABW0CX6MY0D5WGCSQ3ADTG
title: MCP-first close-out, worktree audit and KB skills
type: task
updated: 2026-11-23T23:38:51.486Z
updated_by: seat:01M85CCK3PTGF5WS0SZ6AY8TGG
verify: cargo nextest run -p bboxy450-term-mux --test it -E 'test(task_close) | test(skill) | test(workspace_audit) | test(mcp_task_accept_requires)'
verify_result:
  command: cargo nextest run -p bboxy450-term-mux --test it -E 'test(task_close) | test(skill) | test(workspace_audit) | test(mcp_task_accept_requires)'
  evidence:
  - criterion: task_accept refuses a report whose commits are not reachable from the integration ref
    reference: crates/bbt-mux/src/mcp/work_items.rs integrated(); test acceptance_requires_origin_main_commit_and_audit_finds_unmanaged_worktrees
  - criterion: task_close moves accepted to done, refuses other states, and hints seat_close then workspace_release
    reference: work_items.rs Action::Close; hints.rs task_close/workspace_audit; test mcp_task_accept_requires_matching_zero_verification_and_declared_criteria_only
  - criterion: workspace_audit lists git worktrees, records and task status with orphan and stuck findings
    reference: crates/bbt-mux/src/mcp/workspaces.rs workspace_audit; test acceptance_requires_origin_main_commit_and_audit_finds_unmanaged_worktrees
  - criterion: skill KB type at skills/<name>/SKILL.md served by skill_list/skill_read with origin kb
    reference: knowledge/types.rs skill; mcp/skills.rs kb_skills; test kb_skills_are_listed_searched_and_read_with_kb_origin
  - criterion: skill_search returns matching skill URIs across builtin, kb and plugin skills
    reference: mcp/skills.rs search(); test kb_skills_are_listed_searched_and_read_with_kb_origin
  - criterion: .agents/skills symlinks to .bboxy/skills in the repository
    reference: commit 69a2eb19b .agents/skills -> ../.bboxy/skills
  - criterion: AGENTS.md close-out uses task_close and workspace_release
    reference: commit 69a2eb19b AGENTS.md Close out landed work
  exit_code: 0
provenance:
  digest: sha256:f646ef5572091145d5da6abf158c9f513459812cfe4542859e438cf9d354b016
  frontmatter_digest: sha256:b86b77b71fd98e58b03f9af7133bee50868fc010b192abe657a9ef34bcd270e3
  signer: seat:01M85CCK3PTGF5WS0SZ6AY8TGG
  sig: aCqqS+XfmZ7FoSb6EkegKl7SaFYPk25TNQcrX++cSegZlNLubA6NTaTtr7m+wwt6+aQgbvtySHTDfCY13ywCCg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/1346
  event:
    id: 01M85CDMBP4A6E2Y5201EN9B8H
    sequence: 1346
    hash: d36fefca961331bc952befe4ad64dccf75b9183244ccbfc5d24bd625a158ce3b
---
# MCP-first close-out, worktree audit and KB skills

Operator direction, 2026-11-09: MCP is the future, skills live in the KB at `.bboxy/skills` and are symlinked into the repository's `.agents/skills`, hints guide the harness, and worktrees and task closing need rigor. Design: [[research/2026-11-09-mcp-first-skills-worktrees-review]].

Operator rulings:
- **`task_close`** moves accepted → done. Its hints then guide the harness to close the seat and release the worktree; it does not perform those steps itself.
- **Skill location:** repository-specific skills are stored as KB documents at `skills/<name>/SKILL.md`, with `.agents/skills -> ../.bboxy/skills`.


## Report from seat:01M70FBP1RV5AWZ5AE3ZZWKNHB

Landed 69a2eb19b and f6dee9b38 (both signed) on origin/main. 69a2eb19b: task_close (accepted -> done), with hints to seat_close, then workspace_audit, then workspace_release; acceptance refuses commits not on origin/main; workspace_audit tool; workspace_prepare fixed for claimed tasks; KB skill type, skill_search, the .agents/skills symlink and the AGENTS.md close-out text. f6dee9b38: remote supervisor link cut 1 (frames; Yarn RemoteDelivery model checked natively and by Java TLC, 52 states each; runtime Outbox/Inbox/FileJournal with a witness test). The verify command exited 0. Not run: make land or the full suite (operator bypass). The installed app needs a rebuild before the new tools are live.

```json
{
  "op_id": "01M70WQD6HD5P8XWDQ6DVFFW8J",
  "commit": "f6dee9b38d3e98d8c2667bb3c3f68d53fea72971",
  "report": "[[reports/2026-11-09-mcp-close-out-kb-skills-remote-link.md]]",
  "verify_result": {
    "command": "cargo nextest run -p bboxy450-term-mux --test it -E 'test(task_close) | test(skill) | test(workspace_audit) | test(mcp_task_accept_requires)'",
    "exit_code": 0,
    "evidence": [
      {
        "criterion": "task_accept refuses a report whose commits are not reachable from the integration ref",
        "reference": "crates/bbt-mux/src/mcp/work_items.rs integrated(); test acceptance_requires_origin_main_commit_and_audit_finds_unmanaged_worktrees"
      },
      {
        "criterion": "task_close moves accepted to done, refuses other states, and hints seat_close then workspace_release",
        "reference": "work_items.rs Action::Close; hints.rs task_close/workspace_audit; test mcp_task_accept_requires_matching_zero_verification_and_declared_criteria_only"
      },
      {
        "criterion": "workspace_audit lists git worktrees, records and task status with orphan and stuck findings",
        "reference": "crates/bbt-mux/src/mcp/workspaces.rs workspace_audit; test acceptance_requires_origin_main_commit_and_audit_finds_unmanaged_worktrees"
      },
      {
        "criterion": "skill KB type at skills/<name>/SKILL.md served by skill_list/skill_read with origin kb",
        "reference": "knowledge/types.rs skill; mcp/skills.rs kb_skills; test kb_skills_are_listed_searched_and_read_with_kb_origin"
      },
      {
        "criterion": "skill_search returns matching skill URIs across builtin, kb and plugin skills",
        "reference": "mcp/skills.rs search(); test kb_skills_are_listed_searched_and_read_with_kb_origin"
      },
      {
        "criterion": ".agents/skills symlinks to .bboxy/skills in the repository",
        "reference": "commit 69a2eb19b .agents/skills -> ../.bboxy/skills"
      },
      {
        "criterion": "AGENTS.md close-out uses task_close and workspace_release",
        "reference": "commit 69a2eb19b AGENTS.md Close out landed work"
      }
    ]
  }
}
```
