---
type: e2e
title: GitKB repository task conventions and numbered reference verification
function: Discover root-bound task slug families and guide numbered references
group: knowledge
status: passed
coverage: Focused plugin, real GitKB, host compatibility, installed hash, approval and live provider readiness
last_run: 2026-11-06T05:53:59.051Z
expected: 'The selected repository supplies task prefixes; native and GitKB numbers remain #N locators'
result: Signed main2847e213 installed and approved for Harmony; both checked swarm bindings healthy; application owners unchanged
run_by: operator
evidence:
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/check-gitkb-final.log
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/knowledge-before.json
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/land.log
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/approval.json
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/knowledge-ready.json
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/knowledge-ready-other.json
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/owners-before.json
- /Users/feral/.local/state/bboxy450-term/task-slugs-20261105/owners-after.json
id: 01M6QNQ9C0VXA7N50XQ3H6RRQF
updated: 2026-11-06T05:53:59.211Z
number: 93377961364439
created_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
updated_by: device:01M62F1BBJT8MW0F3YMZJ8BENM
authorship:
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  kind: local-dev
  principal: operator:01M62F1BBJT8MW0F3YMZJ8BENM
  device_id: 01M62F1BBJT8MW0F3YMZJ8BENM
  public_key: a8bbsPrJyl7hZraHSoUBkc76MQiRVYXx/kgj6IWVFt0=
provenance:
  digest: sha256:e1a2f5a28d83bdf1cdd2c761c6b8bbafa3a7a7af63c8e98ff56e2eb93aa4ae06
  frontmatter_digest: sha256:2ac2856b3cffc3ac6827d40e76c102b3555f9995461a8b49bbc30c35e9087086
  signer: device:01M62F1BBJT8MW0F3YMZJ8BENM
  sig: 8wOBLgj7N/8QA2BEwmr5VNmaNSU2IFKBDqee70cHs7Wlhy980ZKeAcr9Aov6UsBGWJ0ROgOIm7klfBXOYyj8Cw==
  log: bboxy://activity/index/384209
  event:
    id: 01M6QPQJFBHBGDBJDEQEWJDTBC
    sequence: 384209
    hash: 8f949e4e8c0ff304ac9098923d442f1220582358a85737178744b75dcee6843d
---
# GitKB repository task conventions

Task: [[#93377961364438]]. Source candidate: `2847e213386fbd7dcfa2cdf289e40552ccc4755d`.

Harmony declares `prefix = "tasks/harmony"` in `.kb/config.toml` under `slug_authority.families`. The new read-only `plugin_tool_gitkb_task_conventions` obtains only that public policy through the pinned GitKB config command. The policy remains native TOML; no second TOML parser or dependency was introduced. It is read afresh from the exact provider-bound repository and cannot be redirected by tool arguments.

The skill loaded by `knowledge_orient` directs harnesses to read conventions before using `plugin_tool_gitkb_create_next`. The allocator retains native slug/authority/sync semantics. A task such as `tasks/harmony-3096` can be fetched by that full slug or `#3096`. The built-in KB also resolves `#N`; its number is metadata, independent of its descriptive filename. Live lookup of `#93377961364438` returned the exact native task ID and path.

Verification: `make check-gitkb-plugin test-licenses PROFILE=release TARGET_DIR=/Users/feral/mydev/bboxy450-term/target GITKB_BIN=/Users/feral/.cargo/bin/git-kb` exited 0. All 27 plugin unit tests passed (one benchmark ignored); real private GitKB conformance passed, including missing policy, rejected root redirection, public configuration isolation and a changed prefix without restart. The offline doctor passed with one explicitly skipped PATH-child observation because the backend uses an absolute pinned path. License checks passed.

The initial conformance attempt expected `invalid_arguments` for a forged root; the SDK correctly rejected it earlier as `host_field_in_arguments`. The expectation was corrected, preserving the stronger boundary. No application, lifecycle, dependency, numbering migration or supervisor change is part of this patch.

Final deployment: signed main `2847e213386fbd7dcfa2cdf289e40552ccc4755d`; the lean `make land` gate passed, including all 386 host library tests and the application compatibility check. Canonical main was fast-forwarded to the same commit.

`make install-gitkb-plugin-prepared` installed the conformance-tested release artifact into `/Users/feral/work/gitkb/harmony/.bboxy/plugins`. Its SHA256 is `6b5d7055eba9d27b674cc9c7739c00bdbfcb8a904cbfec3858efdebbd8d6620e`, matching the conformance receipt and installed file. One `ctl plugin approve` call pinned that exact hash, checked it and enabled it with the existing export tier. The new tool and skill are in the approved shape.

The existing provider selection required rebinding to the current checked version. The first cold health check timed out; a same-provider warm reselect succeeded without changing or inflating deadlines. Both checked Harmony swarms report revision20, the new hash, a live provider process and no refusal. Supervisor, logic and GUI process identities match the before snapshot exactly. No application restart, seat restart, or real Harmony document mutation was performed.

Refresh `knowledge_orient` and the MCP tool catalogue to load the new provider skill and `plugin_tool_gitkb_task_conventions` in an existing harness. Existing full slugs and numeric references remain valid.
