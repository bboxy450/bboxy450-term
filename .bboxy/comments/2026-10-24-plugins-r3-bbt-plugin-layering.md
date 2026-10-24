---
id: 01M5NVBXDW1HCPDPCDD94ECJZY
number: 224
type: comment
title: "below bbt-proto, bbt-plugin cannot name the types its contracts carry"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Where the host lives"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# below bbt-proto, bbt-plugin cannot name the types its contracts carry

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

`bbt-plugin` is "placed below `bbt-proto` in the layering order". But `event_sink` delivers envelopes, and the envelope type is `bbt_proto::messaging::Envelope` (`crates/bbt-proto/src/messaging.rs:100`). `task_system.report`/`accept` carry `verify_result`, whose type is `TaskVerification` in the same module (`messaging.rs:482`, `:507`). `scripts/layering.sh` fails any crate that depends on a later one (`ORDER`, line 7), so the contract schemas cannot reference those types from below.

**Ask (decide before P-0).** Either the contracts are JSON Schema documents with no Rust types (and a test in `bbt-proto` proves its types serialise to them), or the envelope and verification types move down into `bbt-plugin` (or a new crate below both) and `bbt-proto` re-exports them. The first keeps the kit crate tiny; the second avoids two definitions of one shape.
