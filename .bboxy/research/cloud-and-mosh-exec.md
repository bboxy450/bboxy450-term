# Mux daemon, mosh and cloud — independent Executor pass

2026-10-21 · Nyx Copperwing <EXECUTOR> · assignment E.

[[vision]] · [[roadmap]] · [[cloud-and-mosh]] · [[terminal-core]] · [[agent-messaging]] · [[harness-touchpoints-exec]]

## Evidence boundary

Read [[cloud-and-mosh]] first. This pass follows the decided daemon-side VTE and viewport-plus-byte-ring model; it investigates the missing protocol and recovery mechanisms. No product code or spec is implemented here. The repository currently specifies **one binary, GUI and --daemon modes**, so the separate `bboxy-agent` name in the earlier research is illustrative, not another selected deliverable. Cloud provisioning remains a future item.

Local aliases:

| Alias | Absolute root |
|---|---|
| `Clone/` | `/private/tmp/claude-501/-Users-feral-mydev-bboxy450-term/3544d975-706b-4ce4-ba8c-d9734c0db856/scratchpad/clones/` |
| `Registry/` | `/Users/feral/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/` |
| `Probe/` | `/var/folders/zd/mv_mr2d92p5_1r8yghzbs5vw0000gn/T/nyx-mux-e-qvm927dm/` |
| `MacMan/` | `/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk/usr/share/man/man2/` |

Reused the five existing clean clones rather than cloning duplicates. `git rev-parse HEAD` and `git status --porcelain` returned 0 for each; no local modifications at inspection:

| Tree | Verified revision / upstream |
|---|---|
| mosh | `decd9b705eb81626f694335b8d5940538beb06da` · [mobile-shell/mosh](https://github.com/mobile-shell/mosh/tree/decd9b705eb81626f694335b8d5940538beb06da) |
| tmux | `9c0c29f7a02d39aa1473d291dd531010920c0a0d` · [tmux/tmux](https://github.com/tmux/tmux/tree/9c0c29f7a02d39aa1473d291dd531010920c0a0d) |
| zellij | `ccabe137c4c81b0cd29ca406ec8d69763452b82d` · [zellij-org/zellij](https://github.com/zellij-org/zellij/tree/ccabe137c4c81b0cd29ca406ec8d69763452b82d) |
| wezterm | `e2a98e1b8af156988ad0ac778658ac27b60a55fb` · [wez/wezterm](https://github.com/wez/wezterm/tree/e2a98e1b8af156988ad0ac778658ac27b60a55fb) |
| EternalTerminal | `7ee361a75fbb3e0900445b110188996fbe950823` · [MisterTea/EternalTerminal](https://github.com/MisterTea/EternalTerminal/tree/7ee361a75fbb3e0900445b110188996fbe950823) |

The clone and probe directories are temporary; pinned upstream revisions preserve reference identity. Measurements below came from this pass, not the earlier agent's reported runs. No transcript content, credential value, remote host or provider account was accessed.

## 1. Mosh: synchronize state, preserve input history

`Clone/mosh/src/protobufs/transportinstruction.proto:7` defines old/new state numbers, acknowledgment and discard boundary, plus a diff. `Clone/mosh/src/network/transportsender-impl.h:306` fills those fields from the sender's retained receiver baseline. This is an application-level state protocol, independent of the transport's packet numbers.

`Clone/mosh/src/statesync/completeterminal.cc:69` compares terminal framebuffers and generates terminal bytes from the difference (`:86`), including resize and echo-ack changes. These are regenerated display bytes, not an unbroken recording of the application's original output. Dropping an intermediate display is therefore possible without dropping input. `Clone/mosh/src/statesync/user.cc:44` subtracts only a known input prefix; `:59` emits all later input events. Do not coalesce typed bytes as if they were replaceable screen updates.

Prediction is separate again: `Clone/mosh/src/frontend/terminaloverlay.h:73` keeps an overlay tentative until its confirmation epoch; `:242` stores prediction/confirmed epochs. Prediction epochs describe confidence in local echo. They must not be reused as daemon-restart or terminal-resize generations. A predicted character is not authoritative screen state or proof that a command ran.

Bootstrap is concrete: `Clone/mosh/src/frontend/mosh-server.cc:450` emits its connect record; `:457` ignores SIGHUP; `:462` forks; `:504` redirects standard descriptors; `:526` creates the shell PTY. This demonstrates how a server can outlive the SSH bootstrap process. It does not make the PTY survive that server's own death.

For our Quinn transport, retain the baseline/diff/application-ack idea and input-prefix distinction. QUIC handles encrypted transport, congestion and path validation; application state still needs explicit resumption after a new connection. [RFC 9000 §9/§10.1](https://www.rfc-editor.org/rfc/rfc9000.html#section-9) allows connection migration but also permits idle expiry and discarded connection state. `Registry/quinn-0.11.12/src/endpoint.rs:247` explicitly limits rebinding: incoming connections and unreachable servers can be lost. `Registry/quinn-proto-0.11.19/src/config/mod.rs:288` enables client migration by default; that is not a guarantee of indefinite offline survival.

**Gap:** no mosh session, WAN impairment, local-echo predictor or Quinn migration was exercised. Prediction should remain a separate implementation concern, especially for arbitrary agent TUIs, dialogs and password entry.

## 2. Existing muxes: what owns state and how a client returns

| Reference | Independently read mechanism | Transfer to our daemon/GUI boundary |
|---|---|---|
| tmux | `Clone/tmux/client.c:470` passes duplicated tty descriptors to its server. `Clone/tmux/server-client.c:2705` handles wakeup and starts/redraws the tty at `:2721`. | Server state survives a client disconnect; a VT redraw is not a structured GPU scene protocol. Do not copy fd-to-client-terminal assumptions into the GPU GUI. |
| zellij | `Clone/zellij/zellij-utils/src/ipc.rs:270` advertises structured rendering; `:273` acknowledges frame sequence numbers. `Clone/zellij/zellij-server/src/background_jobs.rs:1272` discovers a dead session from its saved layout file. | Explicit rendering capability and frame acknowledgments are useful. Discovering/restoring a layout is separate from preserving the original processes. |
| wezterm | `Clone/wezterm/mux/src/localpane.rs:124` keeps the terminal model, process and master PTY together. `Clone/wezterm/codec/src/lib.rs:915` sends dimensions, dirty stable-row ranges, cursor, title/cwd, modes, input serial and sequence; `:933` requests lines by stable-row range. | Closest model for authoritative daemon state and a GUI line cache. Screen updates need modes and dimensions as well as cell text. |
| wezterm reconnect | `Clone/wezterm/wezterm-client/src/pane/renderable.rs:425` marks cached lines stale. `Clone/wezterm/wezterm-mux-server-impl/src/sessionhandler.rs:89` computes dirty rows since its previous sequence and includes changed viewport/cursor rows. | Invalidate/revalidate caches on reconnect; fetch history lazily instead of mirroring 10k rows for every seat. |
| Eternal Terminal | `Clone/EternalTerminal/src/base/BackedWriter.hpp:25` documents an in-memory recovery buffer capped at 64 MiB. `Clone/EternalTerminal/src/base/Connection.cpp:183` retrieves the peer's missing sequence suffix before catch-up I/O. | Byte replay has a bounded retained window; an unavailable prefix needs an explicit reset/gap, not silent continuity. This buffer is not durable storage. |
| ET scrollback | `Clone/EternalTerminal/src/terminal/SessionScrollback.hpp:14` exposes raw bytes, next cursor and truncation. `:50` advances absolute offsets and evicts chunks. | A byte cursor is useful for loss detection and transcript fetch. It is not a rendered-row identifier. Its cap is chunk-based: `:58` retains at least one chunk, so one oversized append can exceed the nominal cap. |

**Gap:** these are source inspections at the pinned revisions, not interoperability tests. The earlier subagent's pointers are not all line-stable: zellij's resurrection function is now at `background_jobs.rs:1249`, with the layout check at `:1272`; this pass uses the actual tree.

## 3. VTE placement and measured memory/CPU

Use the already-decided **VTE in the PTY-owning daemon**. The GUI keeps render/cache state. It can reconnect without replaying every application's original byte stream, and remote daemons can serve MCP screen reads without a GUI. Moving the parser to the GUI would relocate memory rather than solve the full-history cost.

Independent fixture: `Probe/Cargo.toml:6` pins `alacritty_terminal = 0.25.1`; `Probe/src/main.rs:19` allocates 80×24 terms and feeds a fresh parser per term. RSS comes from `ps -o rss=` before/after allocation, with terms retained until after the sample. Host: Apple M4, 25769803776 bytes RAM; rustc 1.99.0; optimized offline build. This measures terminal models, not PTYs, harnesses, GPU resources or connections.

| Actual command, all exit 0 | Delta RSS / seat | Other actual output |
|---|---:|---|
| `target/release/nyx-mux-e 1000 10000 nofill` | 100.5 KiB | Total delta 100512 KiB (98.16 MiB), retained history 0; allocation 0.010199 s. |
| `target/release/nyx-mux-e 32 0 fill` | 105.5 KiB | Retained history 0; 59904 input bytes. |
| `target/release/nyx-mux-e 16 2000 fill` | 4190 KiB (4.09 MiB) | 2525952 bytes, 0.015555 s, 154.9 MiB/s, retained history 2000. |
| `target/release/nyx-mux-e 8 10000 fill`, two fresh runs | 20520 KiB (20.04 MiB), both | 6254976 bytes, 0.040601 / 0.040608 s, 146.9 MiB/s, retained history 10000. |

`size_of::<Cell>() = 24`, `size_of::<Term<Noop>>() = 1728`. `Registry/alacritty_terminal-0.25.1/src/term/cell.rs:134` contains the character, colors, flags and optional shared extra; `Registry/alacritty_terminal-0.25.1/src/grid/row.rs:17` owns a cell vector. Actual occupied history matters: configuring 10k history on an empty seat does not allocate the filled-history footprint.

**Extrapolations, not measurements:** 1000 filled 10k-history terms at this workload imply 19.57 GiB RSS delta; 2k history implies about 4.00 GiB. A 2 MiB payload ring per seat is another 1.95 GiB at full occupancy, plus roughly 0.10 GiB for viewport models; a 4 MiB ring doubles the payload term. These omit checkpoints, indexes, uncommon attributes, queues, caches and harness memory. Ten KiB/s from each of 1000 seats is 9.77 MiB/s; against this synthetic 146.9 MiB/s parse rate that is about 6.7% of one core **for this parsing workload only**. Neither output rate nor end-to-end CPU use was measured.

**Gap:** the selected fork's checkpoint/index overhead and real agent-output behavior are unmeasured. No claim that 1000 PTYs, harnesses or Quinn connections fit follows from allocating 1000 empty terminal models. No renderer/fps benchmark was run.

## Daemon ↔ GUI protocol: proposed minimum contract

This is a research recommendation for [[mux-daemon]], not an implemented API. Start with reliable framed streams, application acks and bounded pending state. Datagram screen updates are optional after a demonstrated need; they are not required merely because mosh uses UDP.

| Message / identity | Required content and invariant |
|---|---|
| `Hello / Attach / Resume` | Protocol/version and capabilities, authenticated device, seat ID, daemon boot epoch, PTY/process generation, last applied view generation/sequence. A durable seat ID does not imply the old process still exists. |
| `Snapshot` | Full subscribed viewport, dimensions, cursor, palette/attributes/hyperlinks, relevant input/display modes, title/cwd and history availability. Apply atomically and acknowledge the applied sequence. |
| `Delta` | Seat/process/view generations, baseline sequence, new sequence and changed rows/cells plus changed metadata. Apply only to the exact retained baseline in the same generations. Otherwise request a snapshot; never guess. |
| `Applied` | GUI acknowledges successful state application, not merely packet/stream receipt. Retain a bounded baseline until it is acknowledged or replace it with a snapshot. |
| `GetHistory / History` | Stable semantic row range or explicit raw-byte cursor, history generation, retained range and truncation. Do not label byte offsets as row IDs. Resize/reflow or cleared history must invalidate incompatible references. |
| `Input / InputAdmitted` | Separate ordered writer/session generation and monotonic input IDs. An admission ack means handed to the queue/PTY boundary, not read by the model or command execution. Deduplicate across reconnect while the admission state survives; preserve unknown outcomes after owner failure. |
| `Subscribe / SeatState` | Visible viewport/range and control-state subscription. Hidden seats still update authoritative VTE/history/state, but need not push intermediate frames. Durable peer messages and permissions are not replaceable screen frames. |

Locally use the same framed semantics over a permission-restricted Unix socket. Remotely use authenticated Quinn streams: keep control/input and bulk history/snapshots from sharing one ordered byte queue; a bounded per-seat render stream is a simple initial mapping. Frames carry lengths and negotiated limits; validate before allocating. Coalesce dirty state before enqueueing the next frame rather than building a FIFO of obsolete renders. Separate streams avoid per-stream ordering stalls across seats, but still share connection congestion/resources.

For optional datagrams, [RFC 9221 §5](https://www.rfc-editor.org/rfc/rfc9221.html#section-5) specifies unreliable delivery, application demultiplexing, packet-size constraints and no datagram flow control. Transport acknowledgments do not prove application processing. `Registry/quinn-0.11.12/src/connection.rs:440` also documents reordering, loss and eviction of older queued datagrams; `:484` exposes a connection/path-dependent maximum. `send_datagram_wait` (`:468`) favors queued old data under congestion, which is undesirable for replaceable stale frames.

If enabled, send only complete bounded updates that fit `max_datagram_size`, or use an explicitly bounded fragment protocol. A missing fragment must not partially mutate a frame. Large/full snapshots and recovery use streams. Every datagram still needs seat/generations/base/new sequence. Discard stale generations, duplicate/older results and updates with a missing baseline. Resize or alternate-screen resets need a new view generation and snapshot. Do not assume a 1200-byte path budget can carry an arbitrary screen.

Connection resumption is not input replay permission. `Registry/quinn-0.11.12/src/connection.rs:120` warns that outgoing 0-RTT can be replayed and must not invoke non-idempotent operations. Keep spawn, role changes and terminal input out of replayable early data. The input guard in [[harness-touchpoints-exec]] still applies.

**Gap:** codec, queue bounds, scheduler policy, snapshot consistency, loss recovery and input journaling are proposed only. No network protocol or multi-client writer arbitration was built/tested. Decide those limits in the spec; no pre-emptive frame-rate tuning is justified here.

## Crash persistence: byte history, terminal state and process life differ

Two local counterexamples and a limited storage probe:

| Probe / actual result | What it disproves or establishes |
|---|---|
| `target/release/nyx-mux-e replay` — exit 0; original 4101 bytes, last 128 retained; full foreground `Named(Red)`, tail replay `Named(Foreground)`, `match=false` | Exact fixture is `ESC[31m` plus 4096 `x` bytes. Evicting the color prefix loses rendering state even though the tail contains printable text. `Probe/src/main.rs:12` compares actual alacritty terminal cells, not a hand-written ANSI model. |
| `python3 crash_probe.py` — exit 0; owner SIGKILL exit -9, child received SIGHUP, observed after 0.43 ms | `Probe/crash_probe.py:12` creates an isolated forkpty child with a HUP reporter, then `:31` kills only its synthetic master owner. The original PTY/harness cannot be promised alive after its sole owner dies. No user harness was touched. |
| Same script — toy writer exit -9; 1059 bytes on disk, valid record `[1]`, ignored incomplete tail 19 bytes | `Probe/crash_probe.py:4` writes/fsyncs one length/sequence/payload/CRC record and begins another; `:45` kills that writer; `:49` validates complete records. A restart reader can reject a torn suffix when records have integrity/framing. This is a toy append log, not our ring or terminal recovery implementation. |

Follow the selected bounded byte ring, but give it explicit guarantees. A raw ring records output and truncation; it does not contain a complete terminal snapshot, a semantic line index or a live PTY. Starting at an arbitrary retained byte can also start inside UTF-8, an escape or a mode transition. A newline byte alone cannot describe wraps, cursor addressing, carriage-return overwrites or resize/reflow.

For reproducible rendered scrollback, the fork needs recoverable VTE/parser state at retained replay boundaries, plus semantic row indexing. Retain a checkpoint at/before the replayable window and enough bytes to advance from it; evict on a valid boundary or expose a gap. A newest-only checkpoint cannot reconstruct earlier discarded screen states. Checkpointing must cover parser state, both screen buffers, cursor/modes/palette and required attribute/hyperlink references. Historical replay must suppress external effects and terminal replies; it must not re-run clipboard operations or re-emit control events as live ones.

For process-crash persistence, a candidate file format needs version/generation, absolute base/head offsets, complete-record checksums and an independently valid committed boundary/checkpoint. Publish a checkpoint only after its data is written/flushed; retire old data only when the committed checkpoint no longer needs it. Validate length, checksum and sequence at restart and surface lost/uncommitted tails. `mmap` alone provides neither transaction ordering nor an atomic wrap/eviction protocol. `MacMan/fsync.2:56` distinguishes host flush from drive persistence; `:76` describes F_FULLFSYNC for stronger ordering. The SIGKILL probe did not test power loss.

| Failure | Recovery that the evidence supports |
|---|---|
| GUI crash, daemon alive | Reattach to the same PTY/model; snapshot and lazy history. GUI caches can be discarded. |
| Network loss, remote owner alive | New authenticated connection; validate epochs and resume/re-snapshot. The remote process continues independently of that connection. |
| Sole PTY-owning daemon dies | Saved state/history can be recovered; original child may have hung up. Automatic harness resume creates a new process generation while preserving durable seat/harness-session IDs. This is not uninterrupted process survival. |
| OS/VM restart | No live PTY survives. Recovery depends on durable records and each harness's resume support, with uncertain in-flight input preserved as uncertain. |

An input journal and PTY write cannot generally be committed as one storage transaction. Crash before/after the write can leave an outcome unknown; do not retransmit an arbitrary command merely because the new daemon lacks an ack. This is the same boundary already recorded in D.

**Gap:** checkpoint format, memory/storage budget and daemon-death semantics need an Oracle decision before implementation. If uninterrupted child survival across daemon death is required, some PTY owner must survive; a disk ring cannot provide it. No broker, process restoration, power-failure test or real harness resume was implemented in this pass.

## 4. Remote SSH bootstrap and cloud seats

The bootstrap should use the decided binary's daemon mode; any commands below describe a future protocol, not existing flags. `Clone/wezterm/wezterm-client/src/client.rs:677` bootstraps a mux proxy through SSH, using a no-auto-start variant on later attempts (`:693`). [OpenSSH ssh(1)](https://man.openbsd.org/ssh) confirms remote command execution and that command arguments are joined before remote-shell execution. Local argv separation alone therefore does not solve remote shell quoting.

Recommended bootstrap sequence within the existing key/grant model:

1. Use the system SSH client and the user's host alias/configuration. Keep bootstrap control I/O free of a PTY (`-T`); the remote daemon creates the harness PTY later. Separate diagnostics from a bounded versioned control record; banners/noise must not become an accepted enrollment record.
2. Probe OS/architecture through fixed commands. Select the exact compatible release locally, verify it, then upload to a user-owned versioned location. Stage and atomically rename rather than `curl … || true` followed by executing an uncertain binary. Detect unsupported OS/libc instead of silently choosing a build. No machine was probed here.
3. Invoke a fixed bootstrap entry point, with enrollment material in a framed stdin payload or protected descriptor, not interpolated shell text/argv. Generate the remote device key there; return only its public key and bootstrap/protocol metadata over the verified SSH channel. Bind it through [[agent-messaging]]'s owner→device→seat grants; an email or claimed role is not the authority. Never ship the owner's private key to the VM.
4. Lock the user's daemon/socket state so retries discover the existing daemon. An enrollment/launch request ID must not accidentally create a second seat on bootstrap retry. Persist the pinned remote device identity and bind fresh connection authentication to it. Owner-only files and verified SSH host identity protect the initial binding.
5. Detach/close inherited descriptors as the mosh source demonstrates, then establish the normal direct/punch/relay transport. Retain an SSH stdio proxy path when UDP connectivity is unavailable. Detachment alone may not bypass host logout/session-manager policy; installation under a persistent user service is a separate deployment choice.
6. On reconnect send seat, process/view generations and applied sequence/history cursor, then resume or snapshot. Do not download/replace the executable or respawn the harness merely because a network connection timed out. A new host/daemon process must advertise a changed generation.

The transport baseline is **published ICQ 0.20.2**, already checked in [[agent-messaging]], not the older `icq-connect` repo cited by the original cloud pass. Punch and encrypted relay paths there are design evidence; this project still implements its selected Quinn transport. An outbound dial from a VM is not proof it can reach a NATed laptop without a reachable direct address, rendezvous/punch or relay.

| Target | Verified bootstrap hook; provider work remains future |
|---|---|
| Plain SSH | Existing user-accessible host + uploaded compatible binary. SSH transport can remain a fallback; no need to provision a provider integration first. |
| Fly.io | [Official console reference](https://docs.fly.io/flyctl/cmd/fly_ssh_console) exposes `-C`, `--machine`, container selection and optional PTY. Treat this as a provider bootstrap adapter, not identical to a plain public-IP SSH host. Persistence/network policy must be verified for the actual machine. |
| Hetzner | [Official hcloud-go, pinned source](https://github.com/hetznercloud/hcloud-go/blob/d1326ee6d5a4292285afd85c486ed858e5927203/hcloud/server.go#L299) declares SSHKeys and UserData at lines 303/305. This supports a future provisioning hook; no API call, default-public-network claim or data-size limit was validated here. |
| AWS EC2 | [Official user-data documentation](https://docs.aws.amazon.com/AWSEC2/latest/UserGuide/user-data.html) supports shell scripts/cloud-init; raw user data is limited to 16 KiB. That is a bootstrap-script budget, not a channel for bundling the binary or a guarantee of UDP access. |
| GCP | [Official startup-script overview](https://docs.cloud.google.com/compute/docs/instances/startup-scripts) says the guest agent reads script content/URL from metadata; custom images need the guest environment. A future metadata hook can install/start the same daemon; it is not an SSH or firewall guarantee. |

**Gap:** no SSH enrollment, binary upload, host trust prompt, system service, NAT punch, relay, provider VM or VM lifecycle was exercised. Bootstrap request/response schema, signing/distribution verification and remote service policy remain implementation work. No claim that every provider permits inbound QUIC follows from these hooks.

## 5. Harness switching: structure only, future work

Metadata enumeration only, with pathlib directory/glob operations; no rollout, transcript, database or live registry content opened:

| Local structure | Actual observation |
|---|---|
| `~/.codex/sessions/` | Exists; recursive extension count 229 `.jsonl`, 75 `.jsonl.zst`. |
| `~/.codex/thread_history_1.sqlite`, `thread-writer-locks/`, `history.jsonl`, `app-server-daemon/` | Exist with file/directory types as named. Presence does not establish which store is authoritative. |
| `~/.claude/projects/` | 8 project directories, 6 top-level JSONL files across those directories, 2 subagent directories. These counts exclude nested JSONL for the top-level count. |
| `~/.claude/sessions/` | Live-registry directory exists; content not read in E. |

Keep durable seat identity separate from a provider/harness session ID. A cross-harness handoff can be a user-approved context conversion, but copying filenames or a foreign transcript is not proof of a resumable target session. This pass does not endorse the earlier report's transcript schemas as independently inspected facts.

**Gap:** no transcript schema/import compatibility or lossless harness migration was tested. Native resume versus a summarized new target session needs its own future assignment; it does not solve PTY process survival.

## Verification and measurement provenance

| Command/read | Actual result |
|---|---|
| `date`, `git status --short`, `git log`; task, vision, roadmap and existing research reads | Exit 0. Read-only baseline established. Other agents modified `lab/web/showcase.js` during this pass; it was not edited here. |
| Five clone revision/status checks; targeted `rg` and numbered source reads | Exit 0; five clean pinned trees as above. No new clone or upstream mutation. |
| `cargo build --release --offline` in Probe | Initial exit 101: generic Processor type needed an annotation. Added explicit `Processor` annotations; final build exit 0, 1.30 s. Dependencies remained pinned by the fixture lockfile. |
| Six fixture invocations (five memory runs plus replay) | All exit 0; full numeric output retained in `Probe/measurements.json`. No inference/model calls. |
| `python3 crash_probe.py` | Exit 0; synthetic PTY HUP and partial-log results above. Only children created by the probe were signalled. |
| `sysctl`, `rustc --version`, local manual/header reads | Exit 0; host/toolchain and flush semantics verified. |
| Primary web reads | RFC 9000/9221, OpenSSH, Fly, AWS and GCP fetched. Initial GCP Linux page timed out; canonical overview fetched. Hetzner rendered API page exposed no fields; pinned official SDK source was fetched instead. No provider credentials/API mutations. |
| Local session structure enumeration | Exit 0; counts above. Transcript contents deliberately untouched. |
| Required `ls` / section-count command from Task E | Exit 0; file exists, nine level-two sections. |
| Source-pointer / document validation script | Exit 0; all 30 qualified pointers exist within line ranges, all seven topical sections and four required wikilinks present, no trailing whitespace/control characters, final newline present. Range validation is not semantic proof. |
| `git diff --check`; `git status --short` | Exit 0; no whitespace errors. Only this deliverable was modified at final verification; the Oracle had already committed an earlier draft. No commit by this Executor. |

Fixture hashes (SHA-256, whole file): `Probe/src/main.rs` = `8a0b6fe6909fe9a496d4214c28fff6f47ed50d08da3b5c25d16ef4ba1d08579a`; `Probe/crash_probe.py` = `3cffeaa042c6cb1034bff2b2f61a391c432ecff4bfce018367284694edb11b1d`; `Probe/Cargo.lock` = `4a54662389ecac2d44b80612a9d5316d10a437f1c3a71020321b3b9539da7c75`; measurements = `10a051bb8afa18f02b645cda36ff452acd277ee541ce9e880fa1e6b8e41ecfd1`.

Only this repository deliverable was authored by this Executor; fixtures are isolated temporary research files. No commit, provider resource, application configuration or task-board edit was made. Oracle owns convergence into [[mux-daemon]].

— Nyx Copperwing <EXECUTOR>
