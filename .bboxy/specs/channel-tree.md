---
id: "0HHE5X8Q1K2CHYNSWF1C53SHGZ"
type: "spec"
number: 122
title: "Channel tree — pubsub as a filesystem of sinks"
created: "2026-10-22T01:04:13Z"
updated: "2026-10-24T00:10:38Z"
---
# Channel tree — pubsub as a filesystem of sinks

Owner: Vesper Greywind <ORACLE>. 2026-10-21. Operator, verbatim intent: "#bboxy is a channel; if I subscribe I add my id to folder bboxy: `bboxy450://channels/bboxy/<myid>`; broadcast to the channel is sending to `bboxy450://channels/bboxy/*`; nested folders `#bboxy/client`, `#bboxy/ui`, `#bboxy/infrastructure`; the Oracle knows what goes where and glues harnesses together by routing; routing becomes symlinking sinks." This is the core of the pubsub system. It sits on the bus in [[events]] and replaces the subscription model of [[messaging]] with something an agent can reason about with `ls`. Back to [[vision]] · [[events]] · [[messaging]] · [[mcp-control-plane]].

## 1. The model in one paragraph

There is one namespace, `bboxy450://`. A **folder** is a channel. A **sink** is a leaf: one delivery endpoint, usually a seat's inbox. **Subscribing** is creating a sink entry in a folder. **Publishing** is writing a message to a path; the path pattern decides which sinks receive it. **Routing** is symlinks: a link in one folder that points at a sink or at another folder. The Oracle edits the tree; everyone else mostly just writes to paths they were given. Listing a folder answers "who hears this channel".

## 2. Namespace

```
bboxy450://
  seats/<seat>/inbox               the seat's own sink (DMs land here)
  seats/<seat>/agents/<agent>      a subagent's sink (relayed via main if not addressable)
  channels/<name>/                 a channel folder; nesting allowed to any depth
    <seat>                         a subscription: a symlink → seats/<seat>/inbox
    ui/  client/  infrastructure/  subchannels: folders inside folders
  groups/<name>/                   a subgroup: a folder of symlinks to seat sinks
  swarms/<swarm>/                  a swarm's root; its stream and subchannels live here
  ext/<source>/                    external sources publish here (github, ci, cron)
  <account-fpr>/…                  another machine or account (phase 2, over Transport)
```

`#bboxy` is shorthand for `bboxy450://channels/bboxy`, `#bboxy/ui` for `…/channels/bboxy/ui`. A DM is a write to one sink: `bboxy450://seats/<seat>/inbox`.

## 3. Writing (publish)

| Write to | Delivered to | Meaning |
|---|---|---|
| `channels/bboxy/<seat>` | that one sink | direct, but visible as "in #bboxy" (thread context kept) |
| `channels/bboxy/*` | every sink **directly** in `bboxy/` | broadcast to the channel's own members |
| `channels/bboxy/**` | every sink in `bboxy/` and all subfolders, recursively | broadcast to the channel and every subchannel |
| `channels/bboxy/ui/*` | members of `#bboxy/ui` only | narrow broadcast |
| `groups/executors/*` | every member of the subgroup | fan-out as DMs with one thread id |
| `seats/<seat>/inbox` | that seat | DM |

`*` matches one level, `**` any depth, exactly as in shells and as NATS `*` / `>` in [[events]]. A sink reached by two paths in one write receives the message once (dedupe by event id). Every delivery records the path it arrived by, so a seat sees "#bboxy/ui" on the message, not just "from Oracle".

## 4. Subscribing, unsubscribing, mute

- Subscribe: `ln -s seats/<me>/inbox channels/bboxy/ui/<me>`. A seat with the right grant may subscribe itself; otherwise the Oracle does it.
- Unsubscribe: `rm channels/bboxy/ui/<me>`.
- Mute: the sink entry stays and is marked paused; deliveries queue as held (the held count in [[ux-prototype]]); unmute releases eligible ones. Mute of a whole channel for a seat is pausing its entry in that folder only.
- Presence: `ls channels/bboxy/**` is the list of who hears it; the entry carries the seat's last-seen state.

## 5. Routing is symlinking

The Oracle glues harnesses together without anyone changing how they speak:

| Link | Effect |
|---|---|
| `channels/bboxy/ui/wren → seats/wren/inbox` | Wren hears `#bboxy/ui` |
| `channels/bboxy/ui/render → channels/bboxy/infrastructure/render/` (folder link) | anything to `#bboxy/ui/**` also reaches the render crew: a bridge |
| `groups/render-crew/nyx → seats/nyx/inbox` | Nyx is in the render crew |
| `ext/github/ci-failed → channels/bboxy/infrastructure/` | CI failures land in infrastructure |
| `channels/bboxy/decisions/oracle → seats/oracle/inbox` + every seat linked in | the decisions channel everyone hears |
| `channels/bboxy/ui/summary → seats/oracle/inbox` with filter `kind=chunk` | the Oracle hears only chunks from #bboxy/ui |

A link may carry a **filter** (kinds, author role class, priority) and a **rate** (coalesce, batch every N minutes): that is how the attention budget is enforced: the Oracle's sinks are batched summaries, a worker's sink receives only its own task and direct addresses. Cycles are allowed in the tree and harmless in delivery: a write visits each sink at most once per event id; the resolver stops on a revisit.

## 6. Who may do what

Folders carry grants like directories: `read` (list, see members), `write` (publish to `*`/`**`), `link` (subscribe others, create bridges), `admin` (mkdir, rm, change grants). Defaults: the Oracle has admin everywhere in its swarm; workers have write on their workstream folder and `link` only for their own sink; Creativity has write on `#…/design`. Grants are the `may-send-to` of the role grants in [[identity]], expressed as paths. A write the grants refuse returns an error naming the folder and the missing right.

## 7. On the bus

The tree is stored in the swarm object ([[storage]]) as rows `(path, kind: folder|sink|link, target, filter, rate, paused, grants)`. Publishing resolves the pattern against the tree at write time, appends one event on the subject `<path>/<kind>` ([[events]] Bus design, `/` separator, kind last) and fans out to resolved sinks; each sink is a named consumer, so `since`, `gap`, ack and evidence states stay exactly as specified. Changes to the tree are themselves events on `bboxy450://swarms/<swarm>/tree/…`, so the GUI and the Oracle see the routing change live and provenance records who linked what.

## 8. Tools and UI

MCP ([[mcp-control-plane]]): `ch_ls(path)`, `ch_mkdir(path)`, `ch_ln(target, link_path, filter?, rate?)`, `ch_rm(path)`, `ch_write(path_pattern, message)`, `ch_mute(path)`, `ch_grant(path, seat, rights)`. Workers mainly use `ch_write` on paths named in their brief and `ch_ls`. The existing `message_send` becomes `ch_write` to the destination's path. The channel graph ([[work-items]]) is this tree drawn: folders, sinks, links, with traffic weights. The stream dock shows folders as tabs, `**` traffic folded under its parent.

## 9. This cohort, expressed as the tree (dogfood)

```
channels/bboxy450-term/            → chunks only: Oracle, all seats linked, filter kind ∈ {chunk, decision, presence}
channels/bboxy450-term/render/     → nyx, oracle(batched)
channels/bboxy450-term/mux/        → rook, oracle(batched)
channels/bboxy450-term/design/     → wren, oracle(batched)
channels/bboxy450-term/decisions/  → everyone
groups/executors/                  → nyx, rook
seats/<each>/inbox                 → DMs
```

ICQ today has flat channels and DMs only, so the cohort approximates this with one channel plus DMs; bboxy450-term makes the tree native.

## Gaps

- **Gap:** cross-account paths (`<account-fpr>/…`) resolve over the Transport; the remote side's grants decide; phase 2.
- **Gap:** pattern resolution cost on very deep trees with many links; cache resolved sink sets per pattern and invalidate on tree events. Measure in W2.4.
