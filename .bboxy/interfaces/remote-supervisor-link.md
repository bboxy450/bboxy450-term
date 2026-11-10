---
type: interface
title: 'ICD: local mux ↔ remote supervisor'
status: baselined
links:
- to: '[[interfaces]]'
  rel: index
- to: '[[supervisor-logic]]'
  rel: extends
id: 01M70W8XWKZ31TRJ3PK9YZZ39K
updated: 2026-11-09T20:45:04.172Z
number: 93377961366486
created_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
updated_by: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
authorship:
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  kind: seat
  public_key: SkH0jBJLMgNQ8eyjFw4BxnIDzq9wMmgZUeBUslwMcCw=
  process_generation: '1'
provenance:
  digest: sha256:acc1a22d02d99d886fe45b0eaa343a9b75f34643430053cb23f685ae030eeb86
  frontmatter_digest: sha256:93098c2760b3cf7d6a80ec21fa4870ff8aeb0db608b632566b378220ca0f4360
  signer: seat:01M70FBP1RV5AWZ5AE3ZZWKNHB
  sig: 1bs1p3EsUUAvA1XMnbfV2tRM8EESkASwpBEsOSitu7MHBGy7qhsfVoxuw65cwpR53NLxHBP2bSFq6+B/CXRGCw==
  log: bboxy://activity/index-93ad8c65834c983167272adcebd708d5016d7d1135673e1090720e78289f698a/151175
  event:
    id: 01M710XBHK10GANMXZQC59B309
    sequence: 151175
    hash: 73ec1c1b76907831a4fa6188f5be1750a5c7bda9b4f98ab91a2ea4de0b47dd25
---
# ICD: local mux ↔ remote supervisor

| | |
|---|---|
| Side A | a local seat: `bboxy450-harness` on an `acp-agent` profile whose agent is `bboxy450-term-remote attach` |
| Side B | `bboxy450-term-remote daemon`, one per remote state directory, on a cloud VM or SSH host, running the real `bboxy450-harness acp` per seat |
| The agreement | operator, 2026-11-09: a whole swarm runs on one cloud or SSH host; the remote hosts **only the embedded bboxy450-harness, over ACP**; KISS, less is more |
| Crate | `crates/bboxy450-term-remote` (landed through 38728611e). Mux is unchanged: the harness profile is the seam. Network and SSH stay out of mux ([[invariants/local-first]]) |
| Formal | `RemoteDelivery` and `RemoteResume`, each with a broken control, checked by native Rust TLC and Java TLC (identical state counts: 52 and 30) |
| Inspired by | herdr-core (Apache-2.0, https://github.com/herdrdev/herdr-core, read at a9298a2): any-stdio carrier, banner-tolerant handshake, pinned identity, redial and backoff, liveness from received bytes, uname-matched install. No code copied. |

## 1. Architecture

```
local harness (acp-agent profile) ⇄ attach ──(ssh -T | via CMD | local)── serve ── link.sock ── daemon ⇄ bboxy450-harness acp (per seat)
                                     └── local MCP servers ⇠ relay ⇠ remote `mcp` processes ┘
```

Commands:

| Command | Runs where | Job |
|---|---|---|
| `profile` | here | Prints the harness profile. Save it as `~/.bboxy450-harness/profiles/<name>.md` and spawn seats with `harness_profile <name>`. The session defaults to `BBOXY_SEAT_ID`. |
| `install` | here → remote | `uname -sm` must match. Streams this build, and optionally a harness, into remote `~/.local/bin`. |
| `attach` | here | The ACP agent command: stdio ACP ↔ the remote session. |
| `serve` | remote | A stdio↔socket byte pump. It writes the handshake line and starts the daemon in its own process group. |
| `daemon` | remote | Owns sessions, the durable applied journal, agent stderr logs and MCP channels. |
| `mcp` | remote | A relayed MCP server, started by the remote harness. |
| `stop` | remote | Ends the daemon and its harnesses (the whole process group). |

## 2. Link

1. **Connect:** the carrier runs `PATH=~/.local/bin:$PATH exec bboxy450-term-remote serve --state DIR`.
2. **Banner:** `serve` writes `bboxy450-remote 1\n` first. The client skips up to 64 KiB of MOTD or banner before it.
3. **Frames:** a u32 big-endian length plus JSON, at most 1 MiB. The protocol is additive-only, and an unknown type is skipped.
4. **Handshake:** `hello{version}` → `welcome{version, supervisor}`. `supervisor` is the persistent `state/supervisor.id`. The client pins it; a daemon restart keeps it, while a cloned disk or different host is refused.
5. **Frame types:**
   - `spawn`, `op{message_id, envelope}`, `applied`, `refused` (agent failed to start or exited, with its last stderr line);
   - `event{epoch, seq, envelope}`, `resume`, `reset{from_seq}`, `close`;
   - `ping`;
   - `mcp_open`, `mcp_data`, `mcp_close`.
6. **Payload:** ACP lines, as exact text.

## 3. Guarantees

- **Ops are applied exactly once.**
  - The outbox resends unacked ops on reconnect.
  - The daemon journals each id (fsync) before writing it to the harness, and answers duplicates with `applied` only.
  - A crash between journal and write loses that op (unknown). It is never doubled.
- **Events resume in order.**
  - Each session keeps a 4096-event tail and is read by cursor.
  - A gap or unknown epoch gets `reset` plus the tail.
  - A new `attach` starts "from now" (`after_seq = MAX`), so old replies are never replayed into a fresh harness.
  - A second reset in one `attach` (the daemon restarted) exits 3, "session lost". The harness then reports that the agent is gone.
- **Only `session/new` is rewritten:**
  - `cwd` becomes the remote directory;
  - each local-socket MCP server (`nc -U`) becomes a remote `mcp` relay, and the original local command runs here per channel.
  - Everything else passes byte for byte, and `attach` answers nothing itself.

## 4. Liveness

- **Pings:** the daemon pings every 15 s after `welcome`.
- **Dead line:** the client declares a line dead after 45 s without any bytes. It does not use per-call timeouts.
- **Redial:** immediately, then backing off from 250 ms, doubling up to 30 s.
- **MCP channels** end with the link and are not queued.

## 5. Deliberately not done (KISS)

- No holder split: a daemon restart ends its harnesses, and the next `attach` sees a new epoch.
- No queued keystrokes or MCP traffic across a drop.
- No API authentication beyond 0700/0600 modes and SSH.
- No remote KB or database.
- No downloaded releases: a mismatched `uname` refuses with instructions.
- No mux code: placement is a harness profile per host.

## 6. Next

1. A swarm-level `where` that selects the profile automatically.
2. Cross-supervisor routing: each hop is an outbox/journal pair, modelled before it is built.
3. A holder split if daemon updates need to keep seats alive.

## 7. Verification

- **Tests:** `cargo nextest run -p bboxy450-term-remote` runs 14 tests unattended:
  - codec;
  - both model checks with broken controls;
  - the relay over a Unix socket;
  - the daemon across a dropped link, with identity pinning (restart accepted, changed id refused; broken control included);
  - the attach rewrite;
  - install into a private HOME;
  - dead-line detection with SIGSTOP;
  - profile output;
  - the MCP relay end to end.
- **Live:** `make test-remote-live` also builds the harness with `acp` and runs a real `bboxy450-harness acp` turn across the link: initialize, session/new, then session/prompt → `end_turn` with streamed updates. It uses the scripted provider: no network, no tokens, no operator.
- **Formal:** `make test-formal` runs both remote models through native and Java TLC.
