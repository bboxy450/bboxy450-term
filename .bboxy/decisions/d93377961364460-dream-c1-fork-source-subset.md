---
type: decision
title: C1 explicitly excludes forked source logs until inherited-prefix custody is specified
id: 01M6NRK80T5KR7W99N42EE39M6
updated: 2026-11-05T11:48:06.274Z
created: 2026-11-05T11:48:06.274Z
number: 93377961364460
created_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
updated_by: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
authorship:
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  kind: seat
  public_key: UgGFd6fs0Ln9gJXlN2fw8kUmonpBkr4IBn/U5N0uOrY=
  process_generation: '1'
provenance:
  digest: sha256:e51a8657d1d43d8d9ac3f83d7fa171f7555b35cc42a344ecf49542d8787f382f
  frontmatter_digest: sha256:ebb37a2e297526546ba558179d56d9751c56f4f4a9a54081f9afb58c82502903
  signer: seat:01M6NJ8E8ZZ27GP1HDBR5R550E
  sig: okpIQ4sZve5OQGXFwUIkYFUMcPm9a1tgaTz73Lruo7R7hvZ6/usQvPB3eaP2XcpiSXGeIYpgmKsDwnsp/wO+Aw==
  log: .bboxy/log/2026-11-05.jsonl#134652
---
# C1 source subset: explicit fork exclusion

C1 currently supports exact closed non-native, non-forked harness session logs. A header with SessionMeta.fork != None must return unsupported_ir immediately. It must not be classified as malformed_ir, parsed as a same-session transcript, silently dropped, or published as a successful partial result.

Evidence: harness src/protocol/laws.rs:24-27,52-65 permits parent-session rows through meta.fork.at, and src/store/fork.rs:256-271 publishes that inherited prefix without rewriting it. The C1 frozen manifest/control contract currently selects one source session identity and has no inherited-parent custody/authorization mapping. Producer and Architect both identified this mismatch in live source review.

Choice: keep C1's narrow source subset and make the unsupported boundary explicit. Supporting inherited prefixes later requires a joint producer/consumer custody contract and canonical-law witnesses; deleting inherited handling from a copied validator is not general protocol-law equivalence. Ordinary harness fork behavior remains unchanged.

Verification: both producer and consumer add a real Store::fork → child close → C1 witness asserting unsupported_ir at the header, unchanged source bytes and no successful result. A parent-session row without an allowed fork header remains malformed_ir; a malformed post-frontier parent-row control must never be accepted. Existing non-forked good/broken law witnesses remain required.

This decision narrows source eligibility only. No new wire field/error, inference, adoption, source rewriting or wider scan is authorized. Required Make gates and runtime/closure/settlement requirements remain unchanged.

Governs [[specs/dream-term-c1]] and [[interfaces/dream-term-c1]]; implemented by [[tasks/2026-11-05-dream-c1-producer]] and [[tasks/2026-11-05-dream-c1-consumer]].