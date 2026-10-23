// Shared demo data for the lab: the showcase and the dedicated pages (graph, stream) read the same fixtures. Data only.
window.FIX = {
  // artifact history: time · seat · event · ref · grade
  history: [
    { time: '13:40', seat: 'Nyx Copperwing', seatColor: '#FF4FA3', event: 'created', kind: 'pr', ref: 'bboxy450-term#14', grade: 'reported' },
    { time: '13:41', seat: 'CI', event: 'created', kind: 'image', ref: 'target/render/smoke/jetbrains-mono-16px-2x-dark.png', grade: 'reported' },
    { time: '13:52', seat: 'Nyx Copperwing', seatColor: '#FF4FA3', event: 'claimed', kind: 'pr', ref: 'bboxy450-term#14', grade: 'reported' },
    { time: '14:03', seat: 'Rook Ashwhisker', seatColor: '#2EE6A8', event: 'modified', kind: 'markdown', ref: '.bboxy/research/gpu-terminals-rust.md', grade: 'inferred' }
  ],
  // what each seat listens to (events.md); auto where delegation subscribed it
  subscriptions: [
    { seat: 'Vesper Greywind', color: '#FFC53D', patterns: [{ pattern: 'task/*', auto: true, why: 'the Oracle follows every task it delegated' }, { pattern: 'seat/*/state' }, { pattern: 'doc/specs/**' }] },
    { seat: 'Nyx Copperwing', color: '#FF4FA3', patterns: [{ pattern: 'task/w2-1', auto: true, why: 'its delegated task' }, { pattern: 'doc/specs/mux-daemon', auto: true, why: 'derived_from of the task' }, { pattern: 'doc/specs/events', auto: true, why: 'derived_from of the task' }, { pattern: 'artifact/*/for:w2-1' }] },
    { seat: 'Rook Ashwhisker', color: '#2EE6A8', patterns: [{ pattern: 'task/w4-1', auto: true, why: 'its delegated task' }, { pattern: 'doc/specs/networking', auto: true, why: 'derived_from of the task' }] },
  ],
  // the external lane: webhooks, CI, cloud, cron
  external: [
    { source: 'github', text: 'PR #14 review requested · bboxy450/bboxy450-term', time: '13:45' },
    { source: 'ci', text: 'macos-15 · render smoke · passed (51 PNGs)', time: '13:58' },
    { source: 'cloud', text: 'eu-2 · seat rig-codex reconnected', time: '14:10' },
    { source: 'cron', text: 'daily checkpoint signed · .bboxy/log/', time: '23:59' },
  ],
  // provenance: items with weighted derived_from, signed digests, a hash-chained log with a signed daily checkpoint
  provenance: {
    items: [
      { type: 'spec', title: 'renderer.md', created_by: 'Vesper Greywind', createdColor: '#FFC53D', updated_by: 'Nyx Copperwing', updatedColor: '#FF4FA3', derived_from: '[[gpu-terminals-rust]]{derives:0.6} [[ux-prototype|ux spec]]{constrains:0.3} [[kittens-events]]{derives:0.1}', digest: { content: 'b3f9c0d2a7e14c88', signed: true, by: 'Boxy (primary)' } },
      { type: 'task', title: 'W3.1 renderer harness', created_by: 'Vesper Greywind', createdColor: '#FFC53D', updated_by: 'Nyx Copperwing', updatedColor: '#FF4FA3', derived_from: '[[renderer]]{derives:1.0}', digest: { content: '7a21ee90cc4f1b03', signed: true, by: 'Boxy (primary)' } },
      { type: 'bug', title: 'glyphs upside down in smoke', created_by: 'Nyx Copperwing', createdColor: '#FF4FA3', derived_from: '[[renderer]]{violates:1.0} [[inv-upright-text|upright text]]{violates:1.0}', digest: { content: '0d4c51f2b9a3e6d7', signed: false } },
    ],
    log: [
      { hash: 'a91f3c2e', time: '13:40', text: 'Nyx Copperwing created pr bboxy450-term#14' },
      { hash: 'c47b08d1', time: '13:52', text: 'Nyx Copperwing claimed bboxy450-term#14' },
      { hash: 'e2d9a570', time: '14:03', text: 'Rook Ashwhisker modified gpu-terminals-rust.md' },
      { hash: 'f00c3b9e', time: '14:30', text: 'Vesper Greywind accepted W3.1' },
      { hash: '5be71a44', time: '23:59', checkpoint: true, by: 'Boxy (primary)' },
    ],
  },
  // seats and epics for the dedicated pages (graph, stream): the swarm "term" as the Oracle's graph.md describes it
  seats: [
    { key: 'v', name: 'Vesper Greywind', harness: 'claude', role: 'oracle', title: 'swarm control', state: 'active', color: '#FFC53D', tasks: [] },
    { key: 'n', name: 'Nyx Copperwing', harness: 'codex', role: 'executor', title: 'W3.2b bboxybook', state: 'active', color: '#FF4FA3', tasks: [{ id: 'w32b', title: 'W3.2b bboxybook', status: 'running' }] },
    { key: 'r', name: 'Rook Ashwhisker', harness: 'codex', role: 'executor', title: 'W2.2 logic', state: 'waiting', color: '#2EE6A8', where: 'cloud', host: 'eu-2', tasks: [{ id: 'w22', title: 'W2.2 logic, bus, index', status: 'running', constrains: 'one sequence per stream' }, { id: 'b2', type: 'bug', title: 'gap row lost on replay', status: 'claimed', violates: 'never invent a count' }] },
    { key: 'w', name: 'Wren Thistledown', harness: 'claude', role: 'creativity', title: 'ux lab', state: 'active', color: '#B07BFF', tasks: [{ id: 'w33', title: 'W3.3 glow and locate', status: 'blocked', waits: 'W3.2b' }] },
  ],
  epics: [
    { id: 'e3', title: 'W3 renderer', open: true, items: [
      { id: 'inv1', type: 'invariant', title: 'upright text', state: 'active', binds: [{ title: 'W3.1 renderer harness', rel: 'constrains' }, { title: 'glyphs upside down in smoke', rel: 'violated-by' }] },
      { id: 'b1', type: 'bug', title: 'glyphs upside down in smoke', status: 'fixed', assignee: 'Nyx Copperwing', violates: 'upright text' },
      { id: 'w31', kind: 'task', title: 'W3.1 renderer harness', status: 'accepted', assignee: 'Nyx Copperwing', constrains: 'upright text', artifacts: [{ kind: 'image', ref: 'target/render/smoke/ · 48 PNG', digest: '7a21ee90', by: 'Nyx Copperwing' }] },
      { id: 'w32a', kind: 'task', title: 'W3.2a ui-model', status: 'accepted', assignee: 'Nyx Copperwing' },
      { id: 'w32b', kind: 'task', title: 'W3.2b bboxybook', status: 'running', assignee: 'Nyx Copperwing' },
      { id: 'w33', kind: 'task', title: 'W3.3 glow and locate', status: 'blocked', waits: 'W3.2b' },
    ] },
    { id: 'e2', title: 'W2 mux', open: false, items: [
      { id: 'w21', kind: 'task', title: 'W2.1 supervisor + PTY + lease', status: 'accepted', assignee: 'Nyx Copperwing' },
      { id: 'w22', kind: 'task', title: 'W2.2 logic, bus, index', status: 'running', assignee: 'Rook Ashwhisker' },
      { id: 'w23', kind: 'task', title: 'W2.3 adapters', status: 'open' },
      { id: 'w12', kind: 'task', title: 'W1.2 byte ring', status: 'open' },
    ] },
  ],
  // the swarm stream as MessageRow projections (decision #38): author, issuer_kind, kind, priority_class + priority, ts; role is the
  // seat registry's (by author.id), evidence the lab's per-message ladder. Nyx's note is a worker at priority 7 (the p1 chip); Wren's
  // is signed under the local-dev issuer; Kestrel is a foreign Oracle, so this swarm receives it as worker class.
  stream: [
    { kind: 'presence', text: 'Rook Ashwhisker joined · cloud eu-2', ts: C.atHM('14:02') },
    { author: { kind: 'seat', id: 'v', name: 'Vesper Greywind' }, role: 'oracle', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'oracle', priority: 0, ts: C.atHM('14:03'), text: 'Rook: W2.2 is yours. Read events.md Bus design first.', evidence: 'application-acknowledged' },
    { author: { kind: 'seat', id: 'r', name: 'Rook Ashwhisker' }, role: 'executor', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'worker', priority: 0, ts: C.atHM('14:05'), text: 'claimed W2.2, store traits first', evidence: 'native-written' },
    { author: { kind: 'seat', id: 'n', name: 'Nyx Copperwing' }, role: 'executor', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'worker', priority: 7, ts: C.atHM('14:09'), text: 'parity triplets: 41 of 52 within threshold, inspecting the rest', evidence: 'recipient-stored', unread: true },
    { kind: 'presence', text: 'Nyx Copperwing muted · 1:1 with the operator', ts: C.atHM('14:11') },
    { author: { kind: 'seat', id: 'w', name: 'Wren Thistledown' }, role: 'creativity', issuer_kind: 'local-dev', kind: 'text', priority_class: 'worker', priority: 0, ts: C.atHM('14:12'), text: 'landed: events.html, the bus at a glance', evidence: 'native-unknown' },
    { author: { kind: 'seat', id: 'k9', name: 'Kestrel Nine' }, role: 'executor', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'worker', priority: 0, foreign: true, ts: C.atHM('14:14'), text: 'relay bench green on eu-2, 41 ms p50', unread: true },
  ],
  // documents the showcase opens in the modal (key V): a .bboxy spec and a plain README
  docs: {
    messaging: { type: 'spec', id: '01JAX4B1C2D3E4F5G6H7J8K9M0', path: '.bboxy/specs/messaging.md', title: 'Messaging on quinn: envelope, grants, evidence', status: 'decided', parent: '[[w2-mux]]', links: ['[[agent-messaging]]{evidence:3}', '[[transport-and-identity]]{evidence:2}'], created: '2026-10-21 10:02', updated: '2026-10-21 16:11',
      prov: { created_by: 'Vesper Greywind', createdColor: '#FFC53D', updated_by: 'Nyx Copperwing', updatedColor: '#FF4FA3', digest: 'b3e1c09a7f2d44e1', signed: 'Boxy · 9B07E849480C68328CA20684844ECC0587DE7AB0' },
      body: '## Decision\nOption C: rebuild the messaging service on quinn; the transport hides behind a trait.\n## Evidence states\n- sender-stored, recipient-stored, native-queued, native-written, native-unknown, application-acknowledged\n- an ambiguous write is never retried; see [[harness-adapters]]\n```mermaid\nsequenceDiagram\n  participant O as Oracle:::oracle\n  participant M as mux\n  participant S as seat:::executor\n  O->>M: message\n  M->>S: wake through the input guard\n  S-->>M: receipt native-written\n  M-->>O: evidence\n```\n## Priority\nOracle-class first; nothing is dropped.',
      near: [{ slug: 'agent-messaging', type: 'note', rel: 'evidence', w: 3 }, { slug: 'transport', type: 'note', rel: 'evidence', w: 2 }, { slug: 'w2-4', type: 'task', rel: 'spec', back: true }, { slug: 'w2-mux', type: 'epic', rel: 'parent' }] },
    readme: { path: 'README.md', title: 'bboxy450-term', body: 'A GPU terminal emulator in Rust built as a cockpit for swarms of coding agents.\n## Start\n```\ncargo run -p bboxy450-term\n```\n- knowledge base in [[README|.bboxy]]\n- vision in [[vision]]' },
  },
  // audit projection for the stream (Oracle effects, swarm-member scope)
  audit: [
    { audit: { actor: 'Vesper Greywind', verb: 'spawned', target: 'Odile Marsh', op: '3fa91c2e', outcome: 'applied' }, ts: C.atHM('14:16') },
    { audit: { actor: 'Vesper Greywind', verb: 'assigned executor to', target: 'Odile Marsh', op: '7d02aa41', outcome: 'applied' }, ts: C.atHM('14:16') },
    { audit: { actor: 'Vesper Greywind', verb: 'interrupted', target: 'Rook Ashwhisker', op: 'c19be004', outcome: 'applied' }, ts: C.atHM('14:18') },
    { audit: { actor: 'Vesper Greywind', verb: 'spawned', target: 'Bram Calder', op: '5e6610fb', outcome: 'rejected', reason: 'spawn cap for this swarm is 8' }, ts: C.atHM('14:19') },
    { audit: { actor: 'Vesper Greywind', verb: 'closed', target: 'Ysolde Pike', op: '90ab3c12', outcome: 'unknown', reason: 'no confirmation from the mux' }, ts: C.atHM('14:21') },
  ],
  // the channel tree (channel-tree.md §9, this cohort): folders, seat sinks, links with filters and rates
  channels: {
    root: 'bboxy450://',
    folders: [
      { path: 'c', label: 'channels/bboxy450-term/', depth: 0, filter: 'kind ∈ chunk · decision · presence' },
      { path: 'c/render', label: 'render/', depth: 1 }, { path: 'c/mux', label: 'mux/', depth: 1 }, { path: 'c/design', label: 'design/', depth: 1 }, { path: 'c/decisions', label: 'decisions/', depth: 1 },
      { path: 'g/render-crew', label: 'groups/render-crew/', depth: 0, kind: 'group' }, { path: 'g/executors', label: 'groups/executors/', depth: 0, kind: 'group' },
      { path: 'x/ci', label: 'ext/github/ci-failed', depth: 0, kind: 'ext' },
    ],
    sinks: [{ id: 'vesper', name: 'Vesper Greywind', color: '#FFC53D' }, { id: 'nyx', name: 'Nyx Copperwing', color: '#FF4FA3' }, { id: 'rook', name: 'Rook Ashwhisker', color: '#2EE6A8' }, { id: 'wren', name: 'Wren Thistledown', color: '#B07BFF' }],
    links: [
      { from: 'c', to: 'vesper', w: .25 }, { from: 'c', to: 'nyx' }, { from: 'c', to: 'rook' }, { from: 'c', to: 'wren' },
      { from: 'c/render', to: 'nyx', w: .9 }, { from: 'c/render', to: 'vesper', rate: 'batched 10m', w: .2 },
      { from: 'c/mux', to: 'rook', w: .6 }, { from: 'c/mux', to: 'vesper', rate: 'batched 10m', w: .1 },
      { from: 'c/design', to: 'wren', w: .7 }, { from: 'c/design', to: 'vesper', filter: 'kind=chunk', w: .15 },
      { from: 'c/decisions', to: 'nyx', paused: true }, { from: 'c/decisions', to: 'rook' }, { from: 'c/decisions', to: 'wren' },
      { from: 'g/render-crew', to: 'nyx' }, { from: 'g/render-crew', to: 'wren' }, { from: 'g/executors', to: 'nyx' }, { from: 'g/executors', to: 'rook' },
    ],
    bridges: [{ from: 'x/ci', to: 'c/mux' }],
  },
  subchannels: [{ name: 'render', unread: 3 }, { name: 'mux', unread: 0 }, { name: 'design', unread: 1 }, { name: 'decisions', unread: 1 }],
  // subchannel traffic, subgroup fan-out and a rejected progress note (messaging.md routing discipline)
  routed: [
    { author: { kind: 'seat', id: 'v', name: 'Vesper Greywind' }, role: 'oracle', issuer_kind: 'gpg-root', priority_class: 'oracle', priority: 0, ts: C.atHM('14:30'), text: 'W3.2b accepted; W3.2c is next', kind: 'chunk', evidence: 'application-acknowledged' },
    { author: { kind: 'seat', id: 'n', name: 'Nyx Copperwing' }, role: 'executor', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'worker', priority: 0, ts: C.atHM('14:31'), ch: 'render', text: 'atlas grows ×2 at 2k glyphs, eviction stays off', evidence: 'native-written' },
    { author: { kind: 'seat', id: 'r', name: 'Rook Ashwhisker' }, role: 'executor', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'worker', priority: 0, ts: C.atHM('14:32'), ch: 'mux', text: 'consumers table lands with ack_wait 30 s', evidence: 'native-written' },
    { author: { kind: 'seat', id: 'w', name: 'Wren Thistledown' }, role: 'creativity', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'worker', priority: 0, ts: C.atHM('14:33'), ch: 'design', text: 'lab.css reads tokens now, 514 literals left', evidence: 'recipient-stored' },
    { author: { kind: 'seat', id: 'v', name: 'Vesper Greywind' }, role: 'oracle', issuer_kind: 'gpg-root', priority_class: 'oracle', priority: 0, ts: C.atHM('14:34'), ch: 'decisions', text: 'pubsub is a filesystem of sinks: channel-tree.md', kind: 'decision', evidence: 'native-written' },
    { author: { kind: 'seat', id: 'v', name: 'Vesper Greywind' }, role: 'oracle', issuer_kind: 'gpg-root', kind: 'text', priority_class: 'oracle', priority: 0, ts: C.atHM('14:35'), group: { name: 'render crew', n: 2 }, text: 'parity triplets: look at the 41 over tolerance together', evidence: 'native-written' },
    { rejected: true, author: { kind: 'seat', id: 'r', name: 'Rook Ashwhisker' }, delegator: 'Vesper Greywind', ts: C.atHM('14:36') },
  ],
};
