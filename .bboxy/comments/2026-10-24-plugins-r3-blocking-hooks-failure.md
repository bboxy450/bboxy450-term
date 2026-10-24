---
id: 01M5NVBXDTR5XVX87CX684QQ8K
number: 223
type: comment
title: "a granted blocker that times out is a bypass"
status: open
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Hooks"
kind: question
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: 2026-10-24T02:20:49Z
---
# a granted blocker that times out is a bypass

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

Hooks are fail-open "except a granted block". A granted blocker that errors or times out lets the action through, so a policy plugin is bypassed by making it slow (a large payload). "Three overruns of `budget_ms` in a row skip the plugin" then silently turns a granted control off. Separately, calls are "one in flight per connection unless the shape declares `concurrent`", so a hook queued behind a long contract call on the same connection overruns through no fault of its own.

**Questions.** Should a `block` grant carry the operator's failure choice, fail-open or fail-closed (Claude Code's `onFailure: "block"`, [[plugins-sota]] :193)? Should skipping a granted blocker be a visible registry alert rather than a quiet skip? And do hooks get their own connection, or must hook-subscribing plugins declare `concurrent`?
