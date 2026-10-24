---
id: 01M5NVBXE4VPGTT2K3GR9GFW6K
number: 227
type: comment
title: "decision #41 still says the shape prints on a bare run"
status: resolved
author: name:Seraph key:eec764b7d2f4b9b8
on: "[[plugins]]"
on_digest: "sha256:a3b7a2b411f819d0fbaa532d587847fe91891bf9a4b9b14e0c2419c9c65d9212"
anchor: "§1 Shape · §1 Protocol (against [[d41-plugins|#41]])"
kind: objection
links:
  - { to: "[[plugins]]", rel: on }
created: 2026-10-24T02:20:49Z
updated: "2026-10-24T02:24:35Z"
---
# decision #41 still says the shape prints on a bare run

Review of `.bboxy/specs/plugins.md` revision 3 at main 9c2022a7 (digest pinned above), fresh eyes from an idle Executor at the operator's request ("review plugin draft and send message to you with feedback!"). Author grade inferred: written outside a cockpit seat, signed only by the machine-key commit. Not repeated here: the 21 comments of 2026-10-23, all resolved.

The decision record [[d41-plugins|#41]] (updated 2026-10-24) still summarises the design as "each printing its JSON shape when run bare", lists six contracts (no `decision_model`), and gives the protocol as "shape → status/login → health → calls", with no trust step and shape before trust. Revision 3 says `--shape`, with a bare run only a courtesy the app never relies on; seven contracts; and trust → shape → setup → health → calls, with "no plugin binary runs, not even for its shape, before a trust record exists". The later paragraphs of #41 fold some of this in, but its first paragraph is what a reader quotes.

**Ask.** Amend #41's summary to revision 3 (`--shape`, seven contracts, trust first), or mark the first paragraph as the original ruling superseded by the revision-3 notes below it.
