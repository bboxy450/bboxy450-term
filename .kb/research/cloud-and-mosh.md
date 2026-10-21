# Research — seat persistence, mosh-style remote seats, cloud harnesses

Status: RESEARCH. Written 2026-10-21 by a research subagent for Vesper Greywind <ORACLE>. Facts are `file:line — mechanism`, `command — result`, or `source: <url>`. Unknowns are **Gap:**.

Clones read (shallow, HEAD as of 2026-10-21) live under `/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/3544d975-706b-4ce4-ba8c-d9734c0db856/scratchpad/clones/` — `mosh/` (decd9b7), `EternalTerminal/` (7ee361a), `tmux/` (9c0c29f), `zellij/` (ccabe13), `wezterm/` (e2a98e1). Paths below are relative to that directory. The scratchpad is session-scoped; re-clone to re-verify.

Measurement harness: `scratchpad/gridmeasure/` (Cargo project against `alacritty_terminal 0.25.1`, `vte 0.15`). Machine: Apple M4, 24 GB (`sysctl -n machdep.cpu.brand_string hw.memsize`).

---

## 1. mosh — State Synchronization Protocol

source: `scratchpad/paper/mosh-paper.txt` (pdftotext of https://mosh.org/mosh-paper.pdf, Winstein & Balakrishnan, USENIX ATC 2012).

### What is synced

- Paper §1 — mosh runs a **server-side terminal emulator** and synchronizes **screen state**, not the octet stream. SSH conveys bytes; mosh conveys frames.
- Paper §2 — SSP runs in both directions on two object types: client→server = **history of user input**; server→client = **contents of the terminal window**. Diff semantics are per object type: input diff contains every intervening keystroke; screen diff is "only the minimal message that transforms the client's frame to the current one."
- `mosh/src/statesync/completeterminal.h:45-82` — the `Complete` object: `diff_from(existing)`, `init_diff()`, `apply_string(diff)`, `subtract()` (no-op for screen), `set_echo_ack(now)`. This is the entire object interface SSP needs.
- `mosh/src/statesync/completeterminal.cc:86` — `display.new_frame(true, existing.get_fb(), terminal.get_fb())`: the screen diff is a **VT byte string** computed by comparing two framebuffers (`mosh/src/terminal/terminaldisplay.h:92`). The wire payload is therefore still ANSI, but generated from state, so intermediate states are skipped.
- `mosh/src/statesync/user.h:71-90` — `UserStream` object for keystrokes; `subtract(prefix)` drops acked input.
- `mosh/src/protobufs/transportinstruction.proto` — `Instruction { protocol_version, old_num, new_num, ack_num, throwaway_num, diff, chaff }`. One datagram = one idempotent diff from state `old_num` to `new_num`, plus the receiver's `ack_num` and a `throwaway_num` telling the peer which old states it may discard.
- `mosh/src/protobufs/hostinput.proto` — server→client `Instruction` extensions: `HostBytes.hoststring`, `ResizeMessage`, `EchoAck.echo_ack_num`.
- `mosh/src/protobufs/userinput.proto` — client→server: `Keystroke.keys`, `ResizeMessage`.
- Paper §2 — explicit non-goal: **scrollback is not synchronized** ("Future versions of Mosh will allow the user to browse the scrollback history"; still true in HEAD). This is the known mosh limitation; see §3 for how we fix it.

### Datagram layer, crypto, roaming

- Paper §2.2 — UDP; each payload gets an incrementing sequence number, is encrypted, sent as one datagram. Idempotency means no replay cache is needed.
- `mosh/src/crypto/crypto.h:91` — `unsigned char key[16]` — **AES-128-OCB**, one shared key for confidentiality and authenticity. `crypto.h:146` — 16 bytes added per packet (final OCB block).
- `mosh/src/crypto/crypto.cc:169-175` — nonce is 12 bytes: 4 zero bytes + 8-byte big-endian value.
- `mosh/src/network/network.cc:88` — nonce value = `(direction << 63) | (seq & SEQUENCE_MASK)`; direction bit distinguishes TO_CLIENT/TO_SERVER so one key serves both directions without nonce reuse.
- `mosh/src/network/network.cc:480` — `dos_assert(p.direction == (server ? TO_SERVER : TO_CLIENT))` prevents reflected packets.
- `mosh/src/network/network.cc:482-487` — out-of-order (seq < expected) packets are decrypted and returned but not used for timing or **targeting**; `expected_receiver_seq = seq + 1` is the only anti-replay state.
- `mosh/src/network/network.cc:526-531` — **roaming**: if `server` and the authenticated packet's source address differs from `remote_addr`, the server adopts the new address. Only the client roams. No handshake, no session resumption message.
- `mosh/src/network/network.h:124,139-140,151` — `DEFAULT_SEND_MTU = 500`, `MIN_RTO = 50 ms`, `MAX_RTO = 1000 ms`, `CONGESTION_TIMESTAMP_PENALTY = 500 ms` (ECN-ish: penalize the echoed timestamp to make the peer slow down, `network.cc:493-498`).
- `mosh/src/network/network.cc:503-519` — RTT estimator: 16-bit ms timestamps and timestamp-replies in every packet; SRTT/RTTVAR with TCP's α=1/8, β=1/4, initial SRTT 1000 ms.
- `mosh/src/network/network.h:142-143` — server UDP port search range **60001–60999**.

### Sender timing (frame rate)

- `mosh/src/network/transportsender.h:49-54` — `SEND_INTERVAL_MIN = 20 ms` (50 Hz cap), `SEND_INTERVAL_MAX = 250 ms`, `ACK_INTERVAL = 3000 ms` (heartbeat), `ACK_DELAY = 100 ms` (delayed ack), `ACTIVE_RETRY_TIMEOUT = 10000 ms`.
- `mosh/src/network/transportsender-impl.h:61-71` — frame interval = `ceil(SRTT/2)` clamped to [20, 250] ms, so about one Instruction is in flight.
- `mosh/src/network/transportsender-impl.h:55` — `SEND_MINDELAY = 8 ms` collection interval (paper §4 chose 8 ms as the minimum of the delay curve); `mosh/src/frontend/stmclient.cc:265` sets it to 1 ms for keystrokes.

### Prediction / speculative local echo

- Paper §3.2 — predictions grouped in **epochs**; an epoch starts tentative (predictions computed but not shown); once one prediction in the epoch is confirmed by the server frame, the whole epoch and future predictions in it are displayed. Arrow keys and control characters bump the epoch (lose confidence).
- Paper §3.2 — server-side **echo ack**: the synced terminal object carries the latest keystroke that has been in front of the application for ≥50 ms; client has no timeouts of its own, so jitter does not cause false negatives.
- `mosh/src/frontend/terminaloverlay.h:48-53` — `Validity { Pending, Correct, CorrectNoCredit, IncorrectOrExpired }`.
- `mosh/src/frontend/terminaloverlay.h:218-225` — `FLAG_TRIGGER_LOW = 50`, `FLAG_TRIGGER_HIGH = 80` ms (underline predictions when SRTT > 80 ms), `GLITCH_THRESHOLD = 250` ms, `GLITCH_FLAG_THRESHOLD = 5000` ms.
- `mosh/src/frontend/terminaloverlay.h:242-250` — `prediction_epoch`, `confirmed_epoch`, `flagging`, `srtt_trigger`.
- Paper §4 — 70% of keystrokes displayed instantly; 0.9% wrong predictions, repaired within one RTT; median 5 ms vs 503 ms for SSH on EV-DO.

### Bootstrap over ssh and handoff

- Paper §2 — unprivileged server started over SSH, listens on a high UDP port, prints a random key; SSH connection is then closed.
- `mosh/scripts/mosh.pl:409-410` — `exec ssh -n -tt user@host -- mosh-server new -p PORT …`.
- `mosh/src/frontend/mosh-server.cc:450` — `printf("MOSH CONNECT %s %s\n", port, key)`; `mosh.pl:429-433` — parent parses `^MOSH CONNECT (\d+?) ([A-Za-z0-9/+]{22})` (22 base64 chars = 128-bit key), then closes the ssh pipe.
- `mosh/src/frontend/mosh-server.cc:452-510` — ignore SIGHUP/SIGPIPE, `fork()`, parent prints `[mosh-server detached, pid = N]` and exits, child dup2's `/dev/null` onto stdio (needed for old sshd to release the session), then `forkpty` (`:526`) for the shell. The daemon outlives the ssh session with no systemd/tmux.
- The client receives the key through the `MOSH_KEY` environment variable, never argv (`mosh.pl` after `:445`).

### What transfers to our QUIC transport

We own a QUIC stack already: `~/mydev/icq-connect/src/transport.rs:46-93` — quinn 0.11.12 + rustls/ring, ephemeral TLS cert, identity proven at application layer by Ed25519 over a TLS exporter (`docs/protocol.md` "icq-auth-v2", exporter label `EXPORTER-ICQ-v2`). `transport.rs:70-74` sets `max_idle_timeout 15 s`, 1 bidi stream. `transport.rs:49` STUN mapping via `stunclient`. `docs/protocol.md` states plainly: **"There is no relay fallback or general NAT hole punching"** — vision.md assumes both; that is a build item, not a reuse item.

| mosh piece | Keep | Replace with |
|---|---|---|
| AES-OCB + seq nonce + key-over-ssh | drop | QUIC/TLS 1.3 crypto; identity = Ed25519 key bound via exporter (icq pattern) |
| UDP datagrams, idempotent diffs, no replay cache | **keep the idea** | QUIC **datagram frames** (RFC 9221, `quinn` `send_datagram`) for screen frames; a reliable stream for keystrokes and control |
| Roaming by adopting the newest authenticated source address | drop | QUIC **connection migration** (path validation, preferred address) does this natively; quinn supports client migration |
| SRTT-driven frame rate, 8 ms collection, 50 Hz cap, 100 ms delayed ack | **keep** | QUIC gives RTT for free; keep the frame scheduler as policy on top |
| `Instruction{old_num,new_num,ack_num,throwaway_num,diff}` | **keep** | Our own frame type; the state-object interface `diff_from / apply / subtract / init_diff` is the design to copy |
| Screen diff encoded as ANSI bytes from framebuffer compare | **keep as one encoding** | optional cell-level binary diff when the client is our own GUI, not a VT |
| Prediction overlay + epochs + server echo-ack | **keep** | identical; the echo-ack field lives in our server state object |
| `MOSH CONNECT port key` line over ssh | keep shape | agent prints `BBOXY CONNECT <quic-addr> <pubkey> <session-id>` over ssh, then dials home (see §4) |
| No scrollback sync | **fix** | see §3: scrollback is a separate, lazily fetched object (wezterm style) |

**Gap:** quinn datagram size vs mosh's 500-byte MTU — QUIC datagrams are bounded by path MTU (~1200 bytes default); frames larger than that need the stream path or fragmentation like `mosh/src/network/transportfragment.h`. Decide at design time.

---

## 2. Eternal Terminal, tmux, zellij, wezterm mux

### Eternal Terminal (et)

Byte-stream relay with reliable replay; no server-side emulator.

- Split: `etserver` daemon (root or user) on TCP `port = 2022` (`EternalTerminal/etc/et.cfg:5`) ↔ per-user `etterminal` process that owns the PTY. `src/terminal/TerminalServer.cpp:110-111` — `UserTerminalRouter` on a FIFO; `src/terminal/ServerFifoPathUnix.cpp:24-27` — `/var/run/etserver.idpasskey.fifo`. `src/terminal/UserTerminalHandlerUnix.cpp:25-27,98` — `etterminal` parses `id/passkey`, registers with the router; `TerminalServer.cpp:153-156` — server learns `(id, key)` from the router.
- Bootstrap: `src/terminal/SshSetupHandler.cpp:98-99` — client runs over ssh: `echo 'id/passkey_TERM' | etterminal …`; `:19` marker `IDPASSKEY:`. Same shape as mosh's `MOSH CONNECT`, inverted (client generates the secret, pushes it through ssh).
- Resume: `src/base/BackedWriter.hpp:32` — `MAX_BACKUP_BYTES = 64 MiB` replay buffer per direction, `:110-116` `deque<Packet> backupBuffer` + `sequenceNumber`. `src/base/Connection.cpp:119-199` — `recover()`: exchange `SequenceHeader{sequenceNumber, reset, resetSalt}` (`proto/ET.proto`), each side replies with `CatchupBuffer` of everything after the peer's last seen seq. `proto/ET.proto` — `ConnectRequest{clientId, version, resetIntent, supportsChallenge}`, `ConnectStatus{NEW_CLIENT, RETURNING_CLIENT, INVALID_KEY, …}`. `src/base/ServerConnection.hpp:134` — `recoveryGraceSeconds = 60` after server restart.
- Reconnect: `src/base/ClientConnection.hpp:16,77` — dedicated reconnect thread; TCP, so no roaming beyond "reconnect with the same clientId".
- Scrollback on reconnect: none in the terminal path — bytes are replayed, the local terminal re-renders them. New in HEAD: `src/terminal/SessionScrollback.hpp:43` — a `2 MiB` ring of raw output bytes with absolute byte cursors and `truncated` flag, multi-reader, non-destructive. That is the cheapest "reconnect scrollback" design in the set: **bytes + monotonic cursor**. `src/terminal/SessionStore.hpp` — `~/.et/sessions/<name>` stores `{host, port, id, passkey, title}` so a client can reattach by name.
- Verdict: et's replay-by-sequence is what you want for the **keystroke** direction and for a dumb-pipe mode; its weakness (every byte must be delivered, buffers fill on slow links) is exactly what mosh's frame skipping fixes for the **screen** direction.

### tmux

- Split: one server per socket owns all PTYs and grids; thin clients attach. `tmux/client.c:105-164` — `client_connect()` tries `connect(AF_UNIX)`, on failure with `CLIENT_STARTSERVER` calls `server_start()`; `tmux/server.c:177-190` — `proc_fork_and_daemon`. Protocol = OpenBSD `imsg` over the Unix socket (`tmux/proc.c:84-106`), `tmux/tmux-protocol.h:23` `PROTOCOL_VERSION 8`, messages `:27-70` (`MSG_IDENTIFY_*`, `MSG_COMMAND`, `MSG_DETACH`, `MSG_RESIZE`, `MSG_READ/WRITE_*`). `client.c:453-487` — client ships flags, `TERM`, ttyname, cwd, terminfo, and **passes its stdin/stdout fds** (`MSG_IDENTIFY_STDIN/STDOUT`) so the server writes the tty directly.
- Resume: the server redraws the whole client from the grid: `server-client.c:2693,2721-2722` — on `MSG_IDENTIFY_DONE` / `MSG_WAKEUP`, `tty_start_tty` + `server_redraw_client`; redraw goes through `screen-redraw.c` → `tty_draw_line`. Server-side VTE, client is a VT.
- Scrollback: server-owned `grid` with `hlimit` (`grid.c:331-341`), history trimmed in chunks of `hlimit/10` (`grid.c:405-415`). Cell storage is compact: `tmux.h:900-911` — `struct grid_cell_entry` is **5 bytes packed** (4-byte union + flags), extended cells spill to `grid_extd_entry`. A client only ever sees the visible rows; scrollback is browsed server-side (copy mode) or via `capture-pane`. Control mode (`control.c:619` `control_write_output`, `%output` lines) is the structured client channel; iTerm2's tmux integration is built on it.
- Verdict: the memory model to copy (5 B/cell, extended attrs out of line). The attach model (hand the server your tty) does not fit a GPU client.

### zellij

- Split: `zellij-server` owns PTYs and grids; `zellij-client` is a thin VT frontend. IPC over `interprocess` local sockets (`zellij-utils/src/ipc.rs:8,39`), framed **prost** protobuf (`ipc.rs:22,570,636`; schemas in `zellij-utils/src/client_server_contract/*.proto`). Server spawned by the client as a detached process (`zellij-client/src/server_launch.rs:70-76`).
- Messages: `ipc.rs:166-280` `ClientToServerMsg { AttachClient{cli_assets, tab_position_to_focus, pane_to_focus, is_web_client}, AttachWatcherClient, Action, Key{key, raw_bytes, is_kitty_keyboard_protocol}, TerminalResize, SubscribeToPaneRenders{pane_ids, scrollback, ansi}, RenderFrameAck{seq}, StructuredRenderSupport, … }`; `:283-330` `ServerToClientMsg { Render{content: String}, RenderFrame{frame: Vec<u8>}, Connected, Exit, SwitchSession, … }`.
- Rendering: server renders every client's whole screen to **ANSI text** per client (`zellij-server/src/output/mod.rs:2127` `serialize() -> HashMap<ClientId, RenderPayload>`), or a structured binary frame when the client advertised `StructuredRenderSupport` (`lib.rs:4487` `RenderPayload::Frame → RenderFrame`), acked with `RenderFrameAck{seq}`. Flow control by ack seq is the mosh idea with a reliable transport underneath.
- Resume: `zellij-server/src/lib.rs:4062-4105` — `AttachClient` registers the client, computes `is_a_reattach`, screen then force-renders (`screen.rs:1774 set_force_render`). Session **resurrection** after a server death is layout-only: `background_jobs.rs:1230-1249` `find_resurrectable_sessions` from the serialized layout cache; scrollback is lost unless `scrollback_lines_to_serialize` (`zellij-utils/src/input/options.rs:340`) wrote it into the layout.
- Scrollback: server grid, `DEFAULT_SCROLL_BUFFER_SIZE = 10_000` (`zellij-utils/src/consts.rs:14`). Clients see only the viewport; `SubscribeToPaneRenders{scrollback}` lets a client (web/mobile) ask for N lines of scrollback as ANSI.
- Remote: `zellij-client/src/remote_attach/` — HTTP login with `auth_token` then WebSockets `/ws/terminal` and `/ws/control` (`config.rs:6-7`). It is a web-terminal bridge, not a roaming protocol.
- Verdict: closest existing Rust server/client split; the per-client ANSI re-render is CPU-heavy at 1000 seats and gives the GUI nothing structured. The `RenderFrame` + `RenderFrameAck{seq}` pair is the right wire pattern.

### wezterm mux

- Split: `wezterm-mux-server` owns `LocalPane { terminal: Mutex<Terminal>, pty: Mutex<Box<dyn MasterPty>> }` (`wezterm/mux/src/localpane.rs:124-128`). GUI is a client over unix socket (`wezterm-mux-server-impl/src/local.rs:88`), TLS (`pki.rs:10-16` — a fresh CA per server start, client certs issued on bootstrap), or ssh. Daemonizes with fork+setsid (`wezterm-mux-server/src/daemonize.rs:15-31`).
- Codec: `wezterm/codec/src/lib.rs:93-97` — frame = leb128 `tagged_len` (msb = zstd-compressed), leb128 `serial`, leb128 `ident`, then bincode body. `:445` `CODEC_VERSION = 46`. `:451-505` PDUs: `ListPanes, SpawnResponse, WriteToPane{pane_id, data}, SendKeyDown{pane_id, event, input_serial}, Resize, GetLines{pane_id, lines: Vec<Range<StableRowIndex>>} / GetLinesResponse{SerializedLines}, GetPaneRenderChanges / GetPaneRenderChangesResponse{dirty_lines, cursor_position, dimensions, bonus_lines, input_serial, seqno}, SearchScrollbackRequest, EraseScrollbackRequest, GetTlsCreds, SetClientId, GetClientList, PaneFocused, TabResized, …`.
- Resume and scrollback: the client does **not** mirror the grid. `wezterm-client/src/pane/renderable.rs:56-68` — `RenderableInner { lines: LruCache<StableRowIndex, LineEntry>, poll_interval, … }`; cache capacity = `scrollback_lines.max(128)` (`:108-110`), default `scrollback_lines = 3500` (`config/src/config.rs:1705-1707`). `:8-9` `BASE_POLL_INTERVAL = 20 ms`, `MAX_POLL_INTERVAL = 30 s` (backs off when idle). Flow: client polls `GetPaneRenderChanges`; server answers with `dirty_lines` since the client's last `seqno` plus `bonus_lines` it predicts the client wants (`wezterm-mux-server-impl/src/sessionhandler.rs:89-142`); client fetches the rest lazily with `GetLines` by **stable row index** (`renderable.rs:727-797`), and `make_all_stale()` (`:425`) on reconnect. Scrollback therefore survives any client crash and is fetched on demand, not pushed.
- ssh bootstrap: `wezterm-client/src/client.rs:677-693` — open ssh, run `<remote wezterm> cli --prefer-mux proxy` which bridges stdio to the remote mux socket; `:827-873` — TLS domains bootstrap credentials over ssh once, then reconnect directly over TLS. `:632-640` — unix domains are not reconnectable, ssh/TLS are.
- Verdict: the **client-side design** we want: stable row indices, dirty ranges since seqno, lazy line fetch, LRU cache, idle backoff. It already has a `Line` serialization (`codec/src/lib.rs:976-980` `SerializedLines{lines, hyperlinks, images}`).

---

## 3. Architecture sketch for bboxy450-term

```
 GUI (wgpu, sidebar, splits)            one process, crashes freely
   │  unix socket, our codec (seqno + stable rows)
   ▼
 local mux daemon  (bboxy-mux)          launchd/systemd user service; owns:
   ├─ seat table   seat id → {harness, role, key, session id, location}
   ├─ local PTYs   VTE + grid per seat (authoritative screen + scrollback)
   └─ remote links QUIC (quinn) to remote bboxy-mux daemons
        ▼
 remote mux daemon on ssh host / cloud VM   same binary; owns its PTYs + grids
```

### Where the VTE runs: in the daemon

Decision argued: **state machine and grid in the mux daemon (mosh/tmux/zellij/wezterm all agree); GUI holds a render cache, not a terminal.** Reasons, in order:

1. Crash survival is the requirement (vision.md "Seats survive terminal crashes"). If the GUI owns the grid, a GUI crash loses every screen and the daemon must replay raw bytes (et style) to rebuild — at 1000 seats that is 1000 × scrollback bytes re-parsed on restart, and intermediate states (`clear`, alt-screen flips) make byte replay produce the wrong screen unless you replay from the beginning.
2. Remote seats need the grid at the remote end anyway, to ship frames instead of bytes over a lossy path (mosh §1 argument). One code path: every seat, local or remote, is "a grid in some daemon; GUI fetches lines."
3. The Oracle MCP tool "read any seat's screen and scrollback" (vision.md) reads from the daemon, not the GUI. Agents are the users; the GUI is optional.
4. CPU is not the bottleneck; memory is, and memory lives wherever the grid lives. Putting it in the daemon lets remote machines hold their own seats' scrollback.

Bytes-to-GUI (et/ssh style) wins only on one axis: zero extra memory on the daemon. It loses crash recovery and the remote case. Rejected.

### Per-seat cost — measured

`command — result` (all `scratchpad/gridmeasure/target/release/gridmeasure <n> <scrollback> [fill]`, RSS via `ps -o rss=`):

| run | per-seat RSS | note |
|---|---|---|
| `1 10000 nofill` | 192 KB | empty Term, 10k history reserved lazily |
| `100 10000 nofill` | 101 KB | amortised baseline per empty 80×24 seat |
| `100 10000 fill` | **15.2–20.5 MB** (two runs: 15176 KB, 20504 KB) | 10 024 lines of 92-char text through the parser |
| `100 2000 fill` | 4.2 MB | 2k scrollback |
| `50 1000 fill` | 2.1 MB | |
| `50 0 fill` | 103 KB | no scrollback |
| `20 100000 fill` | 138 MB | 100k scrollback |
| `size_of::<Cell>()` | 24 bytes | `alacritty_terminal-0.25.1/src/term/cell.rs:134-139` — `c: char, fg, bg, flags, extra: Option<Arc<CellExtra>>` |
| `size_of::<Term<Noop>>()` | 1728 bytes | |
| `/usr/bin/time` of `100 10000 fill` | user 1.03 s for ~92 MB of input | ≈ 90 MB/s single-core parse+grid, release build |

Arithmetic check: 10 024 rows × 80 cells × 24 B ≈ 19.2 MB, plus `Row` headers (`grid/row.rs:17-25` `Vec<T>` + `occ`) — matches the 20.5 MB run; the 15 MB run is RSS granularity/allocator slack. **alacritty_terminal stores every scrollback row at full width, 24 B/cell, no compaction.** tmux stores 5 B/cell (`tmux.h:900-911`).

### At 1000 seats (80×24)

| config | daemon memory | basis |
|---|---|---|
| alacritty grid, 10k scrollback, all full | **15–20 GB** | measured ×1000 — does not fit a 24 GB laptop with the GUI, harnesses and browsers alongside |
| alacritty grid, 2k scrollback | 4.2 GB | measured ×1000 |
| alacritty grid, viewport only | 0.1 GB | measured ×1000 |
| tmux-style 5 B/cell, 10k scrollback | ~4 GB | estimate: 10 024 × 80 × 5 B = 4 MB/seat |
| **raw-bytes scrollback ring (et `SessionScrollback` style), 2 MiB cap + viewport grid** | **~2.1 GB worst case, typically far less** | estimate: 2 MiB/seat only if full; agent output is bursty, most seats idle |
| compressed line storage (zstd per 64-line block, text ~5–10× ) | ~0.5–1 GB | estimate, not measured |

Realistic shape: most of 1000 seats are **remote** (vision.md: "some local, some remote over ssh, some in the cloud"), so each machine holds its own seats' scrollback and the local daemon holds only local seats plus the GUI line cache. Local daemon budget with, say, 200 local seats at 2 MiB ring ≈ 400 MB. Fine.

CPU estimate (not measured end to end): agents emit maybe 1–10 KB/s each when active; 1000 seats × 10 KB/s = 10 MB/s, about 11% of one core at the measured ~90 MB/s parse rate. Frame scheduling at mosh's 50 Hz cap for 1000 seats would be 50k diffs/s if all were changing — cap frames per seat by SRTT and by GUI visibility (only visible seats get frames; hidden seats only get a state bit for the sidebar). 1000 QUIC connections in quinn: fine on one tokio runtime; icq-connect's 64-connection cap (`docs/protocol.md`) is a policy, not a limit.

### Recommendation

- **Daemon holds**: viewport grid per seat (alacritty_terminal is acceptable for the viewport: 100 KB), **scrollback as a bounded raw-byte ring with monotonic cursor** (et `SessionScrollback.hpp` pattern), plus a line index (byte offset of each `\n`/wrap) so the GUI can ask for stable row ranges. Re-parse rows on demand into cells when the GUI scrolls. Memory per seat ≈ viewport + ring; cost moves from RAM to CPU on scroll, which is rare.
- **Wire to GUI and between daemons**: one frame type carrying `{seat, old_seq, new_seq, ack_seq, viewport diff (cell runs or ANSI), cursor, modes, echo_ack}` per mosh `Instruction`, with wezterm's `GetLines{stable rows}` for scrollback and `RenderFrameAck{seq}` flow control. Same frames over unix socket locally and QUIC remotely.
- **Prediction overlay** in the GUI for remote seats only, mosh epochs, server echo-ack in the frame.
- **Gap:** `alacritty_terminal` vs `wezterm-term` vs own VTE for the viewport grid — not evaluated here; alacritty measured only because it was the quickest to build. Terminal-core research owns this.
- **Gap:** whether to persist the byte ring to disk so seats survive a **daemon** crash too (tmux/zellij do not; et keeps 64 MB in RAM). A mmap'd ring per seat under `~/.local/state/bboxy/seats/<id>/` would make it cheap; decide in the mux-daemon spec.

---

## 4. Cloud — run a harness transparently in a cloud of the user's choice

Need, in one page:

1. **One static binary** `bboxy-agent` (= the mux daemon in "remote" mode), Linux x86_64 + aarch64 musl, macOS arm64. The user drops it on any VM. Nothing else to install: no tmux, no mosh, no systemd unit required (daemonize like `mosh-server.cc:462-510`: fork, dup `/dev/null`, ignore SIGHUP; optionally write a user systemd unit when systemd is present).
2. **ssh bootstrap** (mosh/et/wezterm all do this; it is the only universal channel): local daemon runs `ssh host -- sh -c 'curl -fsSL <release-url>/bboxy-agent-$(uname -m) -o ~/.bboxy/agent || true; ~/.bboxy/agent serve --owner <user-pubkey> --home <local-daemon-addr-hints> --token <one-time-nonce>'`. The agent prints one line `BBOXY CONNECT <quic-port> <agent-pubkey> <machine-id>` (mosh `MOSH CONNECT`, `mosh-server.cc:450`), then detaches. The local daemon parses the line and closes ssh. Fallback for hosts without curl: `scp` the binary first.
3. **Identity bound to the user key.** Each remote agent generates its own Ed25519 key (vision.md: every seat/daemon has a key) and is **authorized** by `--owner <user-pubkey>`: it accepts QUIC connections only from a peer that proves the owner key, or a key carrying a certificate signed by the owner key (device keys). The one-time `--token` through ssh prevents a race where someone else dials the fresh agent first. This is the icq-connect auth pattern (`transport.rs:112-134` `auth_bytes`/`exporter`, Ed25519 over a TLS exporter) extended with one delegation hop. **Gap:** certificate format for device keys — a 3-field signed tuple `(owner, device, expiry)` is enough; decide with agent-messaging.
4. **Dial home through our transport.** The remote agent runs the same QUIC endpoint as the local daemon. Three paths, tried in order, same as vision.md "direct QUIC → simultaneous NAT punch → relay": (a) direct — cloud VMs usually have a public IP, so the local daemon just connects to `<vm-ip>:<quic-port>` learned from the CONNECT line; (b) the VM dials the user's laptop — laptops are behind NAT, so this needs the punch via the rendezvous the messaging layer already plans; (c) relay. **What exists today** (`~/mydev/icq-connect/docs/protocol.md`): STUN mapping observation, BEP44 address records on mainline DHT, no punch, no relay. Punch and relay are build items shared with agent-messaging, not cloud-specific.
5. **Seat semantics are identical**: remote agent spawns the harness in a PTY, owns grid + scrollback ring, ships frames; local daemon registers `seat id → (agent key, remote seat id)`; sidebar shows the seat with a location badge. Harness wake and MCP go through the same QUIC link (icq already taps Claude/Codex sockets locally; remote side runs the same adapter code against its local sockets).
6. **Reconnect**: QUIC migration + idle timeout tuned up from icq's 15 s to minutes; on a new connection the agent resumes from the client's last acked `seq` (mosh) and the GUI's line cache is marked stale (wezterm `make_all_stale`). Agent keeps running with no client attached, indefinitely.

Provider notes (what the bootstrap needs per target; keep provisioning out of v0.1 — "bring your own VM, give us ssh"):

| target | how we get ssh / bootstrap | notes |
|---|---|---|
| **generic ssh** | user's `~/.ssh/config` host alias; we exec the system `ssh` (mosh does: `mosh.pl:409`) so agents, jump hosts, `Match` blocks and 1Password/YubiKey all work | the only path in v0.1 |
| **Hetzner Cloud** | `POST https://api.hetzner.cloud/v1/servers` with `ssh_keys` and `user_data` (cloud-init `#cloud-config` that fetches and starts `bboxy-agent`), Bearer token | source: https://docs.hetzner.cloud/reference/cloud#tag/servers (not fetched; field names confirmed via the Ansible `hetzner.hcloud.server` module docs, https://docs.ansible.com/ansible/latest/collections/hetzner/hcloud/server_module.html). **Gap:** `user_data` size limit not confirmed. Public IPv4/IPv6 by default → direct QUIC path works |
| **Fly.io** | `fly ssh console -C "<cmd>"`, `--machine <id>`, `-u root`, `--pty` off when `-C` given | source: https://docs.fly.io/flyctl/ssh-console (fetched). ssh rides Fly's WireGuard private network; machines have no public UDP by default → agent must **dial out** (path b/c), or allocate a public IP with a UDP service. Machines stop when idle unless `auto_stop_machines = false` — a seat must keep the machine alive |
| **AWS EC2** | key pair on launch + `UserData` (cloud-init, 16 KB limit), or SSM `send-command`/`start-session` without ssh | source: https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/user-data.html (not fetched this session). Security group must open the QUIC UDP port for the direct path; otherwise dial out |
| **GCP Compute Engine** | `gcloud compute ssh` (OS Login or metadata ssh keys) + `startup-script` metadata | source: https://cloud.google.com/compute/docs/instances/startup-scripts (not fetched this session). Firewall rule for UDP ingress or dial out |

Common thread: every provider gives ssh in minutes and public UDP ingress only on request; so **the agent dialing home over our transport is the primary path and direct inbound QUIC is the optimization**, which is the opposite of mosh (server listens, client roams). That is why punch/relay belong in v0.x even for the cloud case.

**Gap:** cost control — stopping/destroying VMs from the sidebar needs provider APIs; out of scope for the bootstrap design.

---

## 5. Session migration between harnesses (future item)

Local structure, listed with `ls`/`find` only; no transcript content read.

**Codex** (`~/.codex`):
- `~/.codex/sessions/YYYY/MM/DD/rollout-<ISO-ts>-<uuid>.jsonl` — 229 `.jsonl` + 75 `.jsonl.zst` (older rollouts compressed). First-line shape (keys only, via `python3 -I`): `{ordinal, timestamp, type, payload}`; `type ∈ {session_meta, event_msg, response_item, …}`; `session_meta.payload` has `{id, session_id, cwd, git, cli_version, model_provider, base_instructions, history_mode, originator, source, thread_source, …}`; `response_item.payload` has `{type: "message", role, content, id, …}` — i.e. the OpenAI Responses API item format.
- `~/.codex/thread_history_1.sqlite` (+ `-shm/-wal`), `~/.codex/thread-writer-locks/` — `codex migrate-rollouts --help`: "migrate legacy local sessions to paginated thread history", so the sqlite is the new canonical store and the rollout jsonl is legacy/export.
- `~/.codex/history.jsonl` (3 MB) — prompt history; `logs_2.sqlite` (87 MB) — logs; `app-server-daemon/` — the shared daemon that `codex agents` browses.
- CLI: `codex resume [SESSION_ID|name] [PROMPT]`, `codex fork`, `codex exec resume`, `codex exec --json`, `codex queue` (queue a message into an existing session), `codex archive/unarchive/delete`. **No import command.**

**Claude Code** (`~/.claude`):
- `~/.claude/projects/<cwd-with-slashes-as-dashes>/<session-uuid>.jsonl` plus `<session-uuid>/` (tool-results, subagents) and `memory/`. 8 project dirs locally. Line types observed in one file: `user, assistant, system, attachment, file-history-snapshot, file-history-delta, queue-operation, mode, permission-mode, last-prompt, ai-title, pr-link, atis-latch`; `message` objects carry `{role, content, model, usage, stop_reason, …}` — Anthropic Messages API shape, with `parentUuid`/`uuid` forming a tree (forks are branches).
- `~/.claude/sessions/<pid>.json` + `<pid>.<hash>.key` — live session registry (12 entries), used for cross-session messaging.
- CLI: `claude --resume <id|name>`, `--continue`, `--fork-session`, `--session-id`, `--from-pr`, `-p/--print` with `--system-prompt`/`--append-system-prompt`, `--input-format stream-json`. **No import command.**

**Can a transcript be replayed into the other harness?** Not natively: both formats are provider-specific message items with tool-call ids, and neither CLI imports foreign history. Workable path: the terminal reads the source transcript (our own parser for both shapes: Codex `response_item.message` / `function_call` / `function_call_output`; Claude `user`/`assistant` `message.content` blocks), renders a **compressed handoff document** (goal, decisions, files touched, last N turns verbatim, open items — the same "handoff protects context" rule as CLAUDE.md), and starts the target harness with it as the first prompt (`codex exec`/`codex "<prompt>"` or `claude -p`/`--append-system-prompt-file`). Tool results are summarized, not replayed, since the target cannot re-bind foreign tool ids. Reverse direction identical.

**Gap:** Codex thread-history sqlite schema — not inspected (binary; would need a read). **Gap:** whether Claude Code's `--input-format stream-json` accepts a pre-seeded assistant/user history as the opening messages, which would allow lossless replay rather than summarization; untested. Record as future item "harness switching" in [[roadmap]].

---

## Commands run (for verification)

- `git clone --depth 1` × 5 into `scratchpad/clones/` — all exit 0; commits listed at top.
- `pdftotext -layout scratchpad/paper/mosh-paper.pdf scratchpad/paper/mosh-paper.txt` — 383 lines.
- `cargo build --release` in `scratchpad/gridmeasure` — `Finished release in 17.80s`; runs as tabulated in §3.
- `find ~/.codex/sessions -type f | sed 's/.*\.//' | sort | uniq -c` — `229 jsonl, 75 zst`; `ls ~/.claude/projects | wc -l` — 8; key-only inspection with `python3 -I`.
- WebFetch https://docs.fly.io/flyctl/ssh-console — flags confirmed; WebSearch for Hetzner `user_data`/`ssh_keys` — field names via Ansible docs, limits unconfirmed. AWS/GCP rows are from prior knowledge, marked not fetched.

Measured: everything in the §3 table with a `command — result`. Estimated and labelled: tmux-style and compressed storage per-seat figures, CPU at 1000 seats, cloud provider limits not marked fetched.

[[vision]] [[roadmap]] [[terminal-core]] [[agent-messaging]]

## Oracle note (2026-10-21, Vesper Greywind)

The "no relay fallback, no NAT hole punching" statement above is about `~/mydev/icq-connect`, an older separate repo. The published `icq` 0.20.2 crate is a different codebase: `src/transport.rs` has punch/hole-punch code paths (4 matches), `src/transport/websocket.rs` is a WebSocket relay carrier, and `stunclient` plus mainline DHT rendezvous are in its `Cargo.toml`. See [[transport-and-identity]] for those pointers and [[agent-messaging]] for the full dial path. The blueprint for our punch and relay is icq 0.20.2, not icq-connect. The build-item conclusion stands: we implement both ourselves on quinn.
