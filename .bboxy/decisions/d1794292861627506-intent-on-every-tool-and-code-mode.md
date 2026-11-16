---
type: decision
title: Every tool takes intent; python_execute is the code mode; bboxy_code as built
status: decided
date: 2026-11-13
id: 01M7B7KG11BXJ2XVRWRVP2GC32
updated: 2026-11-13T19:54:26.181Z
created: 2026-11-13T19:54:26.181Z
number: 1794292861627506
created_by: seat:01M79HGAW321GYNK8468FYN4K9
updated_by: seat:01M79HGAW321GYNK8468FYN4K9
authorship:
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  kind: seat
  public_key: O/GLv3wEDMQc2X2CJwZN8QH5avRWsmx3x4SdZZC6lKU=
  process_generation: '1'
provenance:
  digest: sha256:9b2c7fadb77856b53268b1c42a3939ca6a760da1d866522cbe219b66d2f7edc1
  frontmatter_digest: sha256:fa5161bc8cb95d7dc15cf44a1df4dfe09ec1a4aaecf3cc3ac9a11de4c7d2bf59
  signer: seat:01M79HGAW321GYNK8468FYN4K9
  sig: ahb6ok8hYu+dKawd7Xqg3/LeTNfdC+nc3xRcw5ymqlVk29SOCVX8oaIqbJSYuFojk1OkcUiA1vGhpG8iOsVQCA==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151672
  event:
    id: 01M7B7KGTDN5GJ022K41MAM4AH
    sequence: 151672
    hash: f32ff09b16aae7796510ee5345198c648b3e0240aaefa572925d7d1771e09ddc
---
# Every tool takes intent; python_execute is the code mode; bboxy_code as built

Decided 2026-11-13 by Lumen Foxglove (solo) on the operator's requests of the same day. Specs: [[code-query]], [[gaps-and-improvements]], [[programmatic-tool-access]].

## 1. `intent` on every tool

**Asked:** MCP calls should carry a field where the caller types out why it is calling, so use can be analysed.

| Question | Choice | Reason |
|---|---|---|
| Mandatory or optional? | Optional. A missing, empty or malformed intent is ignored, never refused. | Mandatory would break every existing caller, test and replay in one step, and blocks a call on a field that does not affect it. Measure the fill rate first. |
| Cost per call | One string, cut to 160 characters; the schema asks for "a few words". | About 15 tokens. No reply bytes. |
| Does the tool see it? | No. The bridge removes it with `op_id`, before size checks and dispatch. | A tool's result and a retry's identity must not depend on wording. A retry worded differently is the same operation. |
| Where is it kept? | An event on the seat's control stream, `seat/<id>/control`, source `mcp`, payload `{kind: "intent", op_id, tool, intent}`. | Beside the call's own audit event; the two join on `op_id`. Signed and append-only like every seat event. |
| Which tools? | All: the host's (`mcp/tools.rs`), the python runtime's (added at the bridge) and `bboxy_code` (accepted and ignored; its calls are in the transcripts). | One rule. |

Code: `crates/bbt-mux/src/mcp/mod.rs` (`intent_schema`, `take_intent`), `mcp/bridge.rs` (strip and forward `Action::Intent`), `seat_socket.rs`, `logic.rs` (`Command::Intent`). Test: `intent_is_recorded_beside_the_call_and_never_reaches_the_tool`.

**Next, from the data:** the fill rate per harness and role; then intent against tool and outcome (which intents end in a refusal, which need three calls). Requiring it on mutating tools is the first candidate if the fill rate is low.

**Not recorded this way:** calls a script makes inside `python_execute`. The run's own intent covers them; each nested call keeps its receipt.

## 2. python_execute is the code mode

Model: one program does many tool calls and returns only its final value (https://arize.com/blog/code-mode/).

Evidence: all 142 historical calls came from one session; 134 loaded a private bridge script to reach the bboxy tools after a stale catalogue; 136 of 136 returned a handle and cost 177 polls. Agents meanwhile wrote about 21,700 shell python heredocs, 12,000 of them guarded file edits.

| Choice | Before | Now |
|---|---|---|
| Tools | `await mcp.call("bboxy", "task_status", {...})`, a raw envelope | `bboxy.task_status(task="#12")`, `tools.<server>.<tool>(...)`; returns the tool's data; `await` optional |
| Discovery | read `python_execute_orient` first | `tools.search("close seat")`, `<server>.describe(tool)` inside the program |
| Result | a handle, then polls | the call waits 25 s by default; a final bare expression is the result |
| Large result | `output_limit` error after the effects | spilled to a file: `{spilled, bytes, head}` |
| Shell and edits | `commands.run([...])`, async | `sh("...").out`; `fs.replace(path, old, new)` refuses unless the old text occurs exactly once |
| Limits | 10 min run, 128 nested calls, 64 KiB source | 24 h run (default 1 h), 4096 nested calls, 1 MiB source |
| Errors | worker frames, raw JSON | the script's own frames with source line; `server.tool: code: message`; unknown argument named |

The per-call wait stays at 25 s because clients drop a tool call at about 30 s; a longer run returns a handle. Term's bridge read the python socket with an 8 s limit; it is now 30 s.

Not built: interpreter state kept between calls; a smaller result envelope; isolation (the run has ordinary host authority, as before).

## 3. bboxy_code as built

Differences from [[code-query]] as first written:

- It is served by the app itself, `bboxy450-term code`, rooted at the seat's cwd. No second binary to install.
- Search and walk use the `regex` and `ignore` crates directly, not the `grep-*` crates.
- `rev` is on `read` only. `search` at a revision is not built.
- A file over 2 MiB is searched but not parsed; `symbol` lists it under `skipped`.
- Results are one copy, compact JSON text, with no `structuredContent` mirror.
- No rust-analyzer, as specified.
