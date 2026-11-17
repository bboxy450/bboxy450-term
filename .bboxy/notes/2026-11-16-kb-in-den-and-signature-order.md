---
type: note
title: KB converted to Den; 472 signatures were falsely invalid (key order)
id: 01M7KCV4DN2BD2QBVNY78Y53WF
updated: 2026-11-16T23:59:55.663Z
created: 2026-11-16T23:59:55.663Z
number: 1794292861627516
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:51104bacfc3ae4266a5e7ec0bbf6ae5a5976e0be44051a571d04c75163ade676
  frontmatter_digest: sha256:31ec6832c382b575b9929988912b0138acd0214567c8b587fc713046ad4cf35a
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: /gFCdxvcCKo9iu8EHW4kdKJ3v5cpd0w+dn/70VPiCMtui0r6VlbcCK04vUSa6Wo5wLZ4JUk3PVpqP0dUBCtwBg==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151817
  event:
    id: 01M7KCV5WCQ64BBXJR4KESMWZ1
    sequence: 151817
    hash: ed0ef3794c0b773705ffd2acecad1275094058862296db8bde7fd1878753e92d
---
## Outcome

The knowledge base thinks in Den. 315 tracked records (plus 53 machine-local reports) are
`.den`, byte-exact: ids, numbers, bodies and signature status are unchanged, and nothing was
refused. Records that cannot convert without losing their signature stay Markdown, still
signed, and convert after the host re-signs them.

Repairing the old records turned up a provenance bug: **472 of 482 signed records were
falsely reported "signature invalid"**. The installed app (built Nov 11 with
`serde_json/preserve_order`) signs the header in insertion order, but the verifier hashed a
sorted re-serialisation. Only 10 tasks were actually edited after signing. Fixed and landed.

## Landed on main (all signed, `make land` exit 0)

| Commit | What |
|---|---|
| 1a948aa77 (+ 8 cherry-picked Den commits) | Den conversion keeps history; e2e night writes `e2e/<ID>.den`; catalogue and e2e-seed read `.den`; `den::markdown` lowering |
| 4892daeef | Verifier accepts the header as written (`Document.written_header`) |
| 5fa15766e | kb-convert skips written-order-signed records instead of breaking their signature |
| e187c377b, fa36cd509 | KB conversion and spec intents; Den lowering applies the host `managed` block; corpus tests read `.den` |

## Mechanisms

- `crates/bbt-mux/src/index.rs`: `Document.written_header`, the sha256 of the YAML header text minus its top-level `provenance:` block, set by `Document::parse`.
- `crates/bbt-mux/src/provenance/queries.rs` `document_signature`: accepts the declared `frontmatter_digest` if it equals either the sorted re-serialisation or `written_header`. Edits still fail. Test `a_header_signed_in_written_order_verifies_and_still_detects_edits` uses a real fixture, with a broken control (`written_header = None` must give invalid).
- `crates/bbt-mux/src/knowledge/den.rs` `convert_file`: an already-invalid signature converts and is flagged (it never gets better or worse). A record valid only in written order is skipped with its reason. `render` stamps `number` before checking, so host-numbered history is exempt from new-record fields (X354, `bboxy450-den/src/record.rs`).
- `crates/bboxy450-den/src/render.rs`: the lowered header applies `managed` overrides (JSON written as YAML flow).
- `crates/bboxy450-term/src/e2e_night.rs`, `knowledge.rs`; `scripts/e2e-seed.sh` (through `scripts/kb_record.py`); `crates/bbt-doc/tests/corpus.rs`.

## KB repairs

- 53 specs gained `intent` through the host. Bodies are byte-identical to git (checked file by file).
- 55 `artifacts/*.md` records (machine-local, gitignored) claimed numbers that git-merged records also held, which made `number_conflict` block 3 specs. I moved them to `~/.bboxy450-quarantine/2026-11-16-artifact-number-collisions/` (reversible).
- 23 closed tasks lack `verify`. The host refuses to amend a terminal task, which is correct, and Den's history exemption covers them.

## Removed

- The converter's refusal of already-invalid signatures, and its dead X354 skip branch.
- The sorted-only signature check.

## Commands

- `cargo nextest run -p bboxy450-den`: 53/53.
- `-p bboxy450-term-mux -E 'den|lowering|convert|signature'`: 112/112.
- `-p bboxy450-doc --test corpus`: 7/7.
- `make test-licenses`: 0. `scripts/layering.sh`: held.
- `make land`: exit 0 three times (1a948aa77, 5fa15766e, fa36cd509).
- `kb-convert` real run: 0 refused for every kind; a second dry run converts 0.

## Gaps

- **Most records still Markdown**: 167 reports, 119 tasks, 55 specs and 46 decisions are written-order-signed. They convert after a sorted-writing host re-signs them, which needs the new build installed and the app restarted (`make install`, then `make restart-recover`).
- **The running app can't read `.den`**: the Nov 11 install can't fetch converted records (`knowledge_fetch #59` gives not_found) until the restart.
- **Ignored night test fails on main too**: `e2e_night::the_runner_turns_script_exit_codes_into_verdicts_and_a_summary` fails at line 123 (T1 screen timeout) identically on origin/main 95927bb24, so it predates this work.
- **One spec unwritable through MCP**: `specs/e2e-script-map.md` (79 KB) is over the MCP write-argument bound, so it can't be rewritten that way.
- **Number collisions can recur**: numbers allocated in different worktree roots can collide after a git merge. A per-root lock can't prevent it.
- **`other` dropped from lowered headers**: the Den lowering doesn't expand `managed other = json{...}` into header keys.
