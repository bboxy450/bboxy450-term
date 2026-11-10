---
type: decision
title: 'canvas becomes a document type: a Markdown record plus its append-only op log'
status: decided
decided: 2026-11-09
links:
- to: '[[d13-document-types]]'
  rel: amends
- to: '[[canvas]]'
  rel: decides
id: 01M71S19TS9AHTA7Z2SE0RQR9E
updated: 2026-11-10T03:46:39.955Z
created: 2026-11-10T03:46:39.955Z
number: 1794274420276169
created_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
updated_by: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
authorship:
  signer: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
  kind: seat
  public_key: nbRk4SVAPLzaJtFzHXqpxa4bNlUvBeSuNZNXX7KMBSk=
  process_generation: '1'
provenance:
  digest: sha256:ca4876e58a8c0a692ce0c897af095adc1ba5d754c9565700b595951fd3318d20
  frontmatter_digest: sha256:9b0be02d5e431ce4b49bbbbb9ed5fdf7b229c6e041353c53b779e3a732565044
  signer: seat:01M71RQWJ3C0DW2E1QHRTKKT3V
  sig: LmKQa3Exv9d1AbQ1cLND8HXK9XzcFhEbpEnQWn1DaWognlvi/gIZrYtCXd7lEMBPDzkMhC6EsWWk9lsrFtCpBw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151320
  event:
    id: 01M71S1A9ZBN4BPDT9ERSS6KB1
    sequence: 151320
    hash: 18152ebc30c6d14084576df35e49bb8a997b33f8d773464bd0b6fd01cfb833f0
---
# canvas record type

**Alternatives.** (a) Store canvases in the account store outside the repo. (b) A canvas as an `artifact` record with an attached file. (c) A new `canvas` type: `.bboxy/canvases/<slug>.md` (frontmatter: type, title, swarm, tags, creator seat, task) beside `.bboxy/canvases/<slug>.log.jsonl`, the append-only op log.

**Choice.** (c).

**Reason.** The spec's rule "the log is the canvas" needs the log as the single truth; a Markdown record gives it a number, title, tags, search and provenance like every other record (d48), and Git carries it to the team with the code it explains. (a) hides it from the repo; (b) bends the artifact type, which describes committed outputs, to hold live mutable content.

Decided by the Oracle (Vesper Greywind) on the operator's delegation, 2026-11-09. Amends [[d13-document-types]]. Spec: [[canvas]].
