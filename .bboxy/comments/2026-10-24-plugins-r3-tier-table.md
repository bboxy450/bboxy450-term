---
id: 01M5NVBXD5B9AWZ0EPKXWD6E2D
number: 215
type: comment
title: "the tiers are named but no contract is assigned one"
status: open
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Contracts"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: 2026-10-24T02:20:49Z
---
# the tiers are named but no contract is assigned one

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

§1 says "each contract carries a trust tier (`read < write < export < transport`)", and P-0 types the trust record, but no contract is given its tier. Several are not obvious. `decision_model` sends app state (instructions, criteria, the state being judged) to a hosted API, so it is `export` even though it only "returns one choice". `data_source.write` to Notion is `export` of the app's content, not `write`. `session` (attach and detach an external session) carries keystrokes, so it is `transport`, like `provider`. `repo_graph` is `read`.

**Ask.** A table in §1, one row per contract (seven, including `decision_model`): tier, and a one-line reason naming the data that leaves the machine. P-0's trust-record type takes it as data, so it cannot be skipped.
