---
id: "01M5MHM2W8VHTMH889ZYT138NH"
type: "decision"
number: 30
title: "Three W2.6 rulings"
status: "decided"
decided: "2026-10-23"
operator: []
links:
  - { to: "[[2026-10-22-architecture-round-1#30]]", rel: "supersedes-as-index" }
  - { to: "[[provenance]]", rel: "decides" }
  - { to: "[[events]]", rel: "decides" }
created: "2026-10-23T14:11:17Z"
updated: "2026-10-24T00:10:38Z"
---
# Decision 30: Three W2.6 rulings

**Three W2.6 rulings (2026-10-23, Oracle, recorded in [[provenance]] and
[[events]]).** (a) Attribution ambiguity: several seats matching a
worktree and activity window → `produced_by: null`, `grade: inferred`,
bounded candidates, a named reason; an authenticated claim resolves it;
an owner is never invented. (b) Signing: signed rows plus the hash
chain are the acceptance; observer-made detections are attributed to a
seat but signed by the observer; daily GPG checkpoints stay pending
while the root is held, with no device-signed substitute. (c) Scope:
cockpit-local (filesystem, hooks, git metadata, local GUI events); GitHub
ingress, webhooks, cloud and hook installation deferred; split 2.6a
detection/claims/log and 2.6b GUI events lane after L8-D's version bump.
Rejected: picking the most recent seat as owner (a guess signed as a
fact); observer signing with a seat's key (a key the observer does not
hold); device-signed daily checkpoints standing in for the root
(changes what the checkpoint proves).
