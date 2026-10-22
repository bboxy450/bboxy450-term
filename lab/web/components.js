// Component library for the web lab. Every renderer returns a DOM node and reads only CSS variables set by lab.js.
// Mirrors the planned Rust ui-model: seat {key,name,harness,role,title,state}, tab {name,shortcut,tree}, space {name,key,tabs}.
(function () {
  const T = window.TOKENS;
  const el = (tag, attrs, ...kids) => window.Lab.el(tag, attrs, ...kids);

  // ---------- seeded helpers ----------
  function hash32(s) { let h = 2166136261; for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); } return h >>> 0; }
  function rng(seed) { let a = hash32(String(seed)); return () => { a |= 0; a = a + 0x6D2B79F5 | 0; let t = Math.imul(a ^ a >>> 15, 1 | a); t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t; return ((t ^ t >>> 14) >>> 0) / 4294967296; }; }
  const pick = (r, arr) => arr[Math.floor(r() * arr.length)];

  // Seat colour = hash of the seat key, quantised hue, fixed S/L per theme. Operator override = seat.color.
  function seatColor(seatOrKey) {
    const seat = typeof seatOrKey === 'string' ? { key: seatOrKey } : seatOrKey;
    if (seat.color) return seat.color;
    const sc = T.seat_color, h = hash32(seat.key);
    const hue = Math.floor((h % sc.hue_steps) * (360 / sc.hue_steps));
    const bands = window.Lab.prefs.theme === 'light' ? sc.bands_light : sc.bands_dark;
    const light = bands[Math.floor(h / sc.hue_steps) % bands.length];
    return `hsl(${hue} ${sc.saturation}% ${light}%)`;
  }

  function hslToRgb(str) {
    const m = /hsl\((\d+)\s+(\d+)%\s+(\d+)%\)/.exec(str); if (!m) { const h = str.replace('#', ''); return [0, 2, 4].map(i => parseInt(h.slice(i, i + 2), 16) / 255); }
    const h = +m[1] / 360, s = +m[2] / 100, l = +m[3] / 100, q = l < .5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
    const f = t => { t = (t + 1) % 1; return t < 1 / 6 ? p + (q - p) * 6 * t : t < .5 ? q : t < 2 / 3 ? p + (q - p) * (2 / 3 - t) * 6 : p; };
    return [f(h + 1 / 3), f(h), f(h - 1 / 3)];
  }

  // ---------- name pools (fantasy, sci-fi, furry) ----------
  const FIRST = ['Vesper', 'Lumen', 'Nyx', 'Thorn', 'Sable', 'Rook', 'Pip', 'Marrow', 'Ash', 'Bramble', 'Juniper', 'Kestrel', 'Orrin', 'Tamsin', 'Wren', 'Cassia', 'Fenn', 'Ilka', 'Dax', 'Zephyr', 'Ione', 'Corvin', 'Halcyon', 'Mireille', 'Sorrel', 'Tobin', 'Ember', 'Quill', 'Vale', 'Lark'];
  const LAST = ['Greywind', 'Foxglove', 'Copperwing', 'Ashvale', 'Quill', 'Ember', 'Hollow', 'Finch', 'Vex', 'Nightshade', 'Starling', 'Ironpaw', 'Moss', 'Halloran', 'Voss', 'Brightmane', 'Thistle', 'Oakhart', 'Silvertail', 'Dusk', 'Riven', 'Calloway', 'Fernsby', 'Lockridge', 'Bellweather'];
  function rollName(r) { return pick(r, FIRST) + ' ' + pick(r, LAST); }

  const HARNESSES = ['claude', 'codex', 'pi', 'omp', 'zsh'];
  const DEFAULT_ROLE = { claude: 'oracle', codex: 'executor', pi: 'executor', omp: 'executor', zsh: null };
  const STATES = ['active', 'idle', 'waiting', 'errored'];
  const GLYPH = { active: '●', idle: '○', waiting: '◐', errored: '✕' };
  const ROLE_MARK = { oracle: '♔', executor: '♟', creativity: '♝', unsigned: '·' };
  const TITLES = ['gpu research', 'messaging spec', 'ux divergence', 'harness hooks', 'mux daemon', 'cargo test', 'review PR 14', 'ssh rig', 'parse logs', 'idle shell', 'tokens', 'relay bench'];

  // ---------- atoms ----------
  // State glyph family: the glyph is the state; its grade (reported | shim | inferred) is drawn as solid / ringed / dashed;
  // a waiting seat carries its kind (permission | question | auth) as a tiny mark. Grade comes from the harness record.
  const GRADE_TITLE = { reported: 'reported by the harness', shim: 'via hooks (shim)', inferred: 'inferred from output' };
  const KIND_MARK = { permission: '⚿', question: '?', auth: '⚷' };
  function glyph(state, seat) {
    seat = seat || { key: 'x' };
    const color = state === 'active' ? seatColor(seat) : state === 'idle' ? 'var(--c-idle)' : state === 'waiting' ? 'var(--c-waiting)' : 'var(--c-errored)';
    const grade = seat.grade || 'reported', kind = state === 'waiting' ? (seat.waitKind || 'question') : null;
    const g = el('span', { class: 'glyph ' + state + ' g-' + grade + (kind ? ' k-' + kind : ''), style: 'color:' + color, title: state + (kind ? ' · ' + kind : '') + ' · ' + GRADE_TITLE[grade] }, GLYPH[state]);
    if (kind) g.append(el('i', { class: 'kind' }, KIND_MARK[kind]));
    return g;
  }

  const ICON_SVG = {
    // simple monochrome marks; currentColor so they tint with context
    claude: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M12 3v18M3 12h18M5.6 5.6l12.8 12.8M18.4 5.6 5.6 18.4"/></svg>',
    codex: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linejoin="round"><path d="M12 2.5 20.5 7.3v9.4L12 21.5 3.5 16.7V7.3z"/><path d="M12 8v8M8.5 10l3.5 2 3.5-2"/></svg>',
    pi: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round"><path d="M4 7h16M8 7v11M16 7v11"/></svg>',
    omp: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><circle cx="12" cy="12" r="8.5"/><circle cx="12" cy="12" r="2.5" fill="currentColor" stroke="none"/></svg>',
    zsh: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M5 6l7 6-7 6M13 18h6"/></svg>',
    gear: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9l2.1 2.1M17 17l2.1 2.1M4.9 19.1 7 17M17 7l2.1-2.1"/></svg>',
    // sidebar-toggle fox: standard sidebar glyph, left column grows an ear tip (pick §8 shape 3). open = sidebar shown (column filled)
    plug: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3v5M15 3v5M6 8h12v3a6 6 0 0 1-12 0zM12 17v4"/></svg>',
    sidebarfox: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="3" y="8" width="18" height="12" rx="2.5"/><path d="M9.5 8v12"/><path d="M3.2 8.5 3.8 2.2 8.6 7.6Z" fill="currentColor" stroke="none"/></svg>',
    sidebarfoxopen: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="3" y="8" width="18" height="12" rx="2.5"/><path d="M3 8h6.5v12H3z" fill="currentColor" stroke="none"/><path d="M3.2 8.5 3.8 2.2 8.6 7.6Z" fill="currentColor" stroke="none"/></svg>',
    archive: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M3 7h18v3H3zM5 10v9h14v-9M10 14h4"/></svg>',
    sun: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="4"/><path d="M12 2v2M12 20v2M2 12h2M20 12h2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"/></svg>',
    moon: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><path d="M20 14.5A8.5 8.5 0 0 1 9.5 4a7 7 0 1 0 10.5 10.5z"/></svg>',
    muted: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6 9H3v6h3l5 4zM16 9l5 6M21 9l-5 6"/></svg>',
    unmuted: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M11 5 6 9H3v6h3l5 4zM15.5 8.5a5 5 0 0 1 0 7M18.5 5.5a9 9 0 0 1 0 13"/></svg>',
    cloud: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M7 18a4 4 0 0 1-.6-7.95A6 6 0 0 1 18 9a4.5 4.5 0 0 1-.5 9z"/></svg>',
    ssh: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><rect x="3" y="4" width="18" height="6" rx="1.5"/><rect x="3" y="14" width="18" height="6" rx="1.5"/><path d="M7 7h.01M7 17h.01"/></svg>',
    key: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="8" cy="14" r="4"/><path d="M11 11 20 2M16 6l2 2M13 9l2 2"/></svg>',
    google: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M21.6 12.2c0-.7-.1-1.3-.2-1.9H12v3.7h5.4a4.6 4.6 0 0 1-2 3v2.5h3.2c1.9-1.7 3-4.3 3-7.3z"/><path d="M12 22c2.7 0 5-.9 6.6-2.4l-3.2-2.5c-.9.6-2 1-3.4 1-2.6 0-4.8-1.8-5.6-4.1H3.1v2.6A10 10 0 0 0 12 22z"/><path d="M6.4 14a6 6 0 0 1 0-3.9V7.5H3.1a10 10 0 0 0 0 9z"/><path d="M12 6c1.5 0 2.8.5 3.8 1.5l2.8-2.8A10 10 0 0 0 3.1 7.5l3.3 2.6C7.2 7.8 9.4 6 12 6z"/></svg>',
  };
  // Logo = the operator's identity portrait (lab/web/bboxy.png, from ~/.agents/identity/bboxy.png). PNG only: no vector mark exists.
  function logo(px, opts) { opts = opts || {};
    return el('img', { class: 'logo', src: 'bboxy.png', alt: 'bboxy', style: 'width:' + px + 'px;height:' + px + 'px;border-radius:' + (opts.square ? '22%' : '50%') + ';display:inline-block;object-fit:cover' }); }

  function harnessIcon(name, extra) { return el('span', { class: 'hicon ' + (extra || ''), title: name, html: ICON_SVG[name] || ICON_SVG.zsh }); }
  function roleMark(role) { return el('span', { class: 'rmark', style: 'color:var(--role-' + (role || 'unsigned') + ')', title: role || 'unsigned' }, ROLE_MARK[role || 'unsigned']); }
  function kbd(s) { return el('span', { class: 'kbd' }, s); }

  // ---------- model ----------
  function makeSeat(r, i, opts) {
    const harness = opts && opts.harness || pick(r, ['claude', 'codex', 'codex', 'codex', 'pi', 'omp', 'zsh']);
    const key = 'k' + hash32('seat' + i + (opts && opts.seed || '')).toString(16);
    return { key, name: harness === 'zsh' ? 'zsh' : rollName(r), harness, role: DEFAULT_ROLE[harness], title: pick(r, TITLES), state: opts && opts.state || pick(r, ['active', 'active', 'idle', 'idle', 'idle', 'waiting', 'errored']) };
  }
  // split tree: {dir:'v'|'h', a, b} | seat
  function makeTree(r, seats) {
    if (seats.length === 1) return seats[0];
    const cut = 1 + Math.floor(r() * (seats.length - 1));
    return { dir: r() < 0.5 ? 'v' : 'h', a: makeTree(r, seats.slice(0, cut)), b: makeTree(r, seats.slice(cut)) };
  }
  function leaves(tree) { return tree.key ? [tree] : [...leaves(tree.a), ...leaves(tree.b)]; }
  function makeModel(nSeats, seed) {
    const r = rng(seed || nSeats);
    const SPACES = ['term', 'winnow', 'ontology', 'paperhands', 'cloud-eu', 'ssh:rig', 'ssh:alpha', 'icq', 'dotfiles', 'simorgh'];
    const spaces = []; let i = 0, sc = 0, shortcut = 1;
    while (i < nSeats) {
      const spaceSeats = nSeats <= 12 ? Math.min(nSeats - i, 3 + Math.floor(r() * 4)) : Math.min(nSeats - i, 8 + Math.floor(r() * 60));
      const name = SPACES[sc % SPACES.length] + (sc >= SPACES.length ? '-' + Math.floor(sc / SPACES.length) : '');
      const space = { name, key: 'space' + sc, tabs: [] };
      let j = 0;
      while (j < spaceSeats) {
        const n = Math.min(spaceSeats - j, pick(r, [1, 1, 1, 2, 2, 3, 4]));
        const seats = Array.from({ length: n }, (_, k) => makeSeat(r, i + j + k, { seed }));
        const tree = makeTree(r, seats);
        space.tabs.push({ name: seats[0].title, shortcut: shortcut <= 9 ? '⌘' + shortcut : '', tree, seats });
        shortcut++; j += n;
      }
      spaces.push(space); i += spaceSeats; sc++;
    }
    return { spaces };
  }
  function rollup(seats) {
    const order = { errored: 0, waiting: 1, active: 2, idle: 3 };
    return seats.slice().sort((a, b) => order[a.state] - order[b.state]);
  }

  // Pulse phase lock: CSS animations start when applied, so rows and panes created at different times would drift.
  // A negative delay aligned to performance.now() puts every pulsing element on one global clock (glow.js uses the same clock).
  function syncPulse(node, state) {
    const ms = state === 'waiting' ? T.motion.pulse_ms : state === 'active' ? T.glow.active_pulse_ms : 0;
    if (ms) node.style.animationDelay = (-(performance.now() % ms)) + 'ms';
    return node;
  }

  // ---------- molecules ----------
  function seatRow(seat, opts) {
    opts = opts || {};
    const row = el('div', { class: 'seat-row ' + seat.state + (opts.focused ? ' focused' : ''), style: '--seat:' + seatColor(seat) });
    const tileCol = (LAUNCH.find(l => l.harness === seat.harness) || {}).tile || 'var(--surface-2)';
    if (seat.where && seat.where !== 'local') row.append(el('span', { class: 'where ' + seat.where, title: seat.where === 'cloud' ? 'runs in the cloud' + (seat.host ? ' · ' + seat.host : '') : 'over ssh' + (seat.host ? ' · ' + seat.host : '') }, harnessIcon(seat.where)));
    row.append(el('span', { class: 'tile', style: 'background:' + tileCol }, harnessIcon(seat.harness)), el('span', { class: 'name', style: 'color:' + seatColor(seat) }, seat.name));
    if (seat.role) row.append(roleMark(seat.role));
    if (seat.muted) { row.classList.add('muted'); row.append(el('span', { class: 'mutemark', title: 'muted: 1:1 with the operator' }, harnessIcon('muted'))); }
    row.append(el('span', { class: 'title' }, seat.title || ''));
    const right = el('span', { class: 'right' });
    if (seat.interruptSent) right.append(el('span', { class: 'isent-chip', style: 'animation-delay:' + since(seat.interruptSent), title: 'interrupt sent · not a promise that it stopped' }, 'interrupt sent'));
    // process generation changed (mux state streams): one-shot chip, the stream line follows after 4 s
    if (seat.restarted) right.append(el('span', { class: 'restarted', style: 'animation-delay:' + since(seat.restarted), title: 'process generation changed' + (seat.generation ? ' · now ' + seat.generation : '') }, 'restarted'));
    // held work (messaging.md swarm mute): muted → everything waits; after unmute only items still blocked by current authority stay, with reason and original id
    const held = seat.held || [];
    if (held.length) {
      const nIn = held.filter(h => h.dir === 'in').length, blocked = held.filter(h => h.blocked);
      const tip = seat.muted ? held.length + ' held while muted · ' + nIn + ' in, ' + (held.length - nIn) + ' out · eligible items release on unmute'
        : blocked.map(h => h.id + ' · ' + h.blocked).join('\n');
      right.append(el('span', { class: 'held' + (seat.muted ? '' : ' blocked'), title: tip }, 'held ' + held.length));
    }
    // seats are sockets: broker telemetry, counts only, never who
    if (seat.listeners > 0) right.append(el('span', { class: 'listeners', title: seat.listeners + ' client' + (seat.listeners === 1 ? '' : 's') + " on this seat's socket · counts only" }, harnessIcon('plug'), String(seat.listeners)));
    const touched = seat.touched || 0;
    if (touched) right.append(el('span', { class: 'touched', title: touched + ' artifact' + (touched === 1 ? '' : 's') + ' touched by this seat' }, harnessIcon('archive'), String(touched)));
    const unread = opts.unread != null ? opts.unread : seat.unread, unreadRole = opts.unreadRole || seat.unreadRole || 'executor';
    if (unread) right.append(el('span', { class: 'badge', style: 'background:var(--role-' + unreadRole + ')' }, unread));
    if (opts.shortcut) right.append(kbd(opts.shortcut));
    right.append(glyph(seat.state, seat));
    row.append(right);
    if (opts.subtitle) row.append(el('span', { class: 'sub' }, opts.subtitle));
    row.dataset.key = seat.key;
    return syncPulse(row, seat.state);
  }

  // Task row: a delegated task under its seat. State is a glyph + chip; reported shows an accept pill (Oracle/operator).
  // statuses are the work-item frontmatter statuses (work-items.md): the file is the contract
  const TASK_STATES = ['open', 'claimed', 'running', 'reported', 'accepted', 'rejected', 'blocked', 'done'];
  const TASK_GLYPH = { open: '○', claimed: '◔', running: '◑', reported: '◕', accepted: '●', done: '✓', rejected: '✕', blocked: '⊘' };
  // nested subagent rows (subagent-detection): running only (active or waiting), in the parent seat's colour; grade from the source
  // (7501 → reported for Claude, hooks → shim for Codex); absent when the harness reports nothing; past 8 collapse into +N running
  function subagentRows(seat) {
    const run = (seat.subagents || []).filter(a => a.state === 'active' || a.state === 'waiting'); if (!run.length) return [];
    const grade = seat.harness === 'codex' ? 'shim' : 'reported', shown = run.length > 8 ? run.slice(0, 7) : run;
    const rows = shown.map(a => el('div', { class: 'sa-row ' + a.state, 'data-seat': seat.key, 'data-agent': a.id, title: a.description + ' · ' + a.state + (a.detail ? ' · ' + a.detail : '') + ' · ' + (grade === 'shim' ? 'from hooks' : 'from OSC 7501') },
      el('span', { class: 'branch' }), glyph(a.state, { key: seat.key, color: seat.color, grade, waitKind: a.kind }), el('span', { class: 'sdesc' }, a.description), a.detail ? el('span', { class: 'sdetail' }, a.detail) : null));
    if (run.length > 8) rows.push(el('div', { class: 'sa-row more', title: run.slice(7).map(a => a.description).join('\n') }, el('span', { class: 'branch' }), el('span', { class: 'sdesc' }, '+' + (run.length - 7) + ' running')));
    return rows;
  }
  // row menu: right-click, ⌘-click, long-press or ⇧F10 on a seat row; never hover-only. Role, permissions and interrupt only for
  // an oracle-class or operator caller (opts.steer); interrupt permission belongs to the caller, not the target.
  const SEND_SCOPES = ['all channels · all DMs', 'own swarm · operator DM', 'operator DM only'];
  const socketPath = seat => '$TMPDIR/bboxy/seats/' + seat.key + '.sock';
  function rowMenu(seat, opts) {
    opts = opts || {};
    const act = (a, v) => { opts.onAction && opts.onAction(a, seat, v); if (!/^copy/.test(a)) opts.onClose && opts.onClose(); };
    const roles = opts.roles || ['oracle', 'executor', 'creativity'];
    const ag = opts.agent;
    const items = ag ? [
      { label: 'Message this subagent', a: 'message-agent', k: '↩', cap: (opts.addressable ? 'direct' : 'relayed via main: ' + (seat.harness || 'the harness') + ' cannot address subagents') },
      { label: 'Message the main agent', a: 'message-main' },
      '-',
      { label: 'Focus ' + seat.name, a: 'focus' },
      { label: 'Copy agent id', a: 'copy-agent', cap: ag.id },
    ] : [
      { label: 'Focus', a: 'focus', k: '↩' },
      { label: 'Split right', a: 'split-right', k: '⌘D' },
      { label: 'Split down', a: 'split-down', k: '⌘⇧D' },
      '-',
      ...(opts.steer ? [
        { label: 'Assign role', sub: roles.map(r => ({ label: r, a: 'role', v: r, on: seat.role === r, mark: ROLE_MARK[r] })) },
        { label: 'Permissions', sub: SEND_SCOPES.map(x => ({ label: x, a: 'sends', v: x, on: (seat.sends || SEND_SCOPES[1]) === x })) },
        { label: 'Interrupt', a: 'interrupt', k: '⌘.' },
      ] : []),
      { label: seat.muted ? 'Unmute' : 'Mute', a: 'mute', k: '⌘M' },
      '-',
      { label: 'Copy socket path', a: 'copy-socket', cap: socketPath(seat) },
      '-',
      { label: 'Close', a: 'close', k: '⌘W', danger: true },
      ...(opts.steer ? [{ label: 'Purge…', a: 'purge', danger: true }] : []),
    ];
    const menu = el('div', { class: 'row-menu', role: 'menu', tabindex: '-1' }, el('div', { class: 'rm-head' }, harnessIcon(seat.harness), el('b', { style: 'color:' + seatColor(seat) }, seat.name), ag ? el('span', { class: 'rm-ag' }, '↳ ' + ag.description) : null));
    const rows = [];
    let sel = 0, flyout = null;
    const closeFly = () => { if (flyout) { flyout.remove(); flyout = null; } };
    const openFly = (it, row) => {
      closeFly();
      flyout = el('div', { class: 'row-menu fly', role: 'menu' }, ...it.sub.map(x => el('div', { class: 'rm-item' + (x.on ? ' on' : ''), role: 'menuitemradio', 'aria-checked': String(!!x.on), onclick: e => { e.stopPropagation(); act(x.a, x.v); } },
        el('span', { class: 'chk' }, x.on ? '✓' : ''), x.mark ? el('span', { class: 'rmk' }, x.mark) : null, el('span', { class: 'lbl' }, x.label))));
      flyout.style.top = row.offsetTop - 5 + 'px'; menu.append(flyout);
    };
    const paint = () => rows.forEach((r, i) => r.classList.toggle('sel', i === sel));
    for (const it of items) {
      if (it === '-') { menu.append(el('div', { class: 'rm-sep' })); continue; }
      const row = el('div', { class: 'rm-item' + (it.danger ? ' danger' : ''), role: 'menuitem' },
        el('span', { class: 'chk' }), el('span', { class: 'lbl' }, it.label, it.cap ? el('small', {}, it.cap) : null), it.sub ? el('span', { class: 'k' }, '▸') : it.k ? el('span', { class: 'k' }, it.k) : null);
      const i = rows.length; rows.push(row);
      row.addEventListener('pointerenter', () => { sel = i; paint(); it.sub ? openFly(it, row) : closeFly(); });
      row.addEventListener('click', e => { e.stopPropagation(); if (it.sub) { openFly(it, row); return; }
        if (it.a === 'copy-agent') { try { navigator.clipboard && navigator.clipboard.writeText(it.cap).catch(() => {}); } catch {} row.classList.add('copied'); }
        if (it.a === 'copy-socket') { try { navigator.clipboard && navigator.clipboard.writeText(it.cap).catch(() => {}); } catch {} row.classList.add('copied'); row.querySelector('.lbl').firstChild.textContent = 'Copied socket path'; }
        if (it.a !== 'copy-agent') act(it.a); });
      row._it = it; menu.append(row);
    }
    menu.addEventListener('keydown', e => {
      const it = rows[sel] && rows[sel]._it;
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); sel = (sel + (e.key === 'ArrowDown' ? 1 : rows.length - 1)) % rows.length; paint(); closeFly(); }
      else if (e.key === 'ArrowRight' && it && it.sub) { e.preventDefault(); openFly(it, rows[sel]); }
      else if (e.key === 'ArrowLeft') { e.preventDefault(); closeFly(); }
      else if (e.key === 'Enter') { e.preventDefault(); rows[sel].click(); }
      else if (e.key === 'Escape') { e.preventDefault(); opts.onClose && opts.onClose(); }
    });
    paint(); if (opts.openSub != null) { const r = rows.find(x => x._it.label === opts.openSub); if (r) requestAnimationFrame(() => openFly(r._it, r)); }
    return menu;
  }
  // purge (GUI only, never MCP): the one confirm in the product, because it skips grace and ends the process now. The seat name
  // must be typed. History moves to cold storage; the terminal never removes it. The d-word never appears.
  function purgeModal(seat, opts) {
    opts = opts || {};
    const go = el('button', { class: 'pg-go', disabled: '' }, 'Purge');
    const input = el('input', { placeholder: seat.name, spellcheck: 'false', autocomplete: 'off', value: opts.typed || '' });
    const ok = () => input.value.trim().toLowerCase() === seat.name.toLowerCase();
    const sync = () => { if (ok()) go.removeAttribute('disabled'); else go.setAttribute('disabled', ''); input.classList.toggle('ok', ok()); };
    const fire = () => { if (ok()) opts.onPurge && opts.onPurge(seat); };
    go.addEventListener('click', fire);
    input.addEventListener('input', sync);
    input.addEventListener('keydown', e => { e.stopPropagation(); if (e.key === 'Enter') fire(); if (e.key === 'Escape') opts.onCancel && opts.onCancel(); });
    const panel = el('div', { class: 'purge', role: 'alertdialog', 'aria-label': 'Purge seat' },
      el('div', { class: 'pg-title' }, 'Purge seat'),
      el('div', { class: 'pg-seat' }, el('span', { class: 'tile', style: 'background:' + ((LAUNCH.find(l => l.harness === seat.harness) || {}).tile || 'var(--surface-2)') }, harnessIcon(seat.harness)), el('b', { style: 'color:' + seatColor(seat) }, seat.name), seat.title ? el('span', { class: 'pg-t' }, seat.title) : null),
      el('p', { class: 'pg-sub' }, 'Terminates now and moves its history to cold storage · nothing is lost'),
      el('ul', { class: 'pg-list' }, el('li', {}, 'skips the ' + (opts.graceS || 60) + ' s grace, no undo'), el('li', {}, 'ring, checkpoints and events move to cold storage'), el('li', {}, 'a later resume can replay them by session id')),
      el('label', { class: 'pg-label' }, 'Type ', el('b', {}, seat.name), ' to confirm'), input,
      el('div', { class: 'pg-acts' }, el('button', { class: 'pg-cancel', onclick: () => opts.onCancel && opts.onCancel() }, 'Cancel'), go));
    sync(); panel.focus = () => input.focus();
    return panel;
  }
  // pending row (MCP mutation status): a seat the Oracle is spawning. accepted → dashed with a spinner; partial and unknown stay
  // dashed with the outcome spelled; rejected looks like the closed strip with its reason. Never a failure, never an automatic retry.
  const PENDING_SAY = { accepted: 'spawning', partial: 'partial', unknown: 'unknown', rejected: 'rejected' };
  function pendingRow(p, opts) {
    opts = opts || {};
    const st = p.status || 'accepted';
    const mark = st === 'accepted' ? el('i', { class: 'spin' }) : st === 'rejected' ? el('span', { class: 'x' }, '×') : el('i', { class: 'ring' });
    const say = PENDING_SAY[st] + ' · ' + (st === 'accepted' ? 'by ' + (p.by || 'the Oracle') : st === 'rejected' ? (p.reason || 'no reason given') : (p.outcome || (st === 'unknown' ? 'no confirmation from the mux · never retried' : 'some effects applied')));
    const top = el('span', { class: 'ptop' }, el('span', { class: 'tname' }, p.name));
    if (st !== 'accepted' && opts.onDismiss) top.append(el('button', { class: 'undo', onclick: e => { e.stopPropagation(); opts.onDismiss(p); } }, 'dismiss'));
    return el('div', { class: 'tab-row pending ' + st, title: (p.op ? 'op ' + p.op + ' · ' : '') + say }, mark, harnessIcon(p.harness), el('span', { class: 'pcol' }, top, el('span', { class: 'pcap' }, say)));
  }
  function taskRow(task, seat, opts) {
    opts = opts || {};
    const row = el('div', { class: 'task-row ' + task.state, 'data-task': task.id, title: task.title + ' · ' + task.state + (task.from ? ' · from ' + task.from : '') });
    row.append(el('span', { class: 'tglyph' }, TASK_GLYPH[task.state] || '○'), el('span', { class: 'ttitle' }, task.title));
    if (task.state === 'blocked' && task.waits) row.append(el('span', { class: 'twaits' }, 'waits on ' + task.waits));
    const right = el('span', { class: 'right' });
    if (task.state === 'reported' && opts.onAccept) right.append(el('button', { class: 'accept', onclick: e => { e.stopPropagation(); opts.onAccept(task, seat); } }, 'accept'));
    right.append(el('span', { class: 'tstate' }, task.state));
    row.append(right);
    return row;
  }

  // Graph page: who is on what (agents) and the work-item tree (epics → specs/tasks with dependencies), generated from data
  function graphPage(opts) {
    opts = opts || {};
    const seats = opts.seats || [], epics = opts.epics || [], swarmName = opts.swarm || '';
    const root = el('div', { class: 'graph' });
    const agents = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Agents · ' + swarmName)));
    const hdr = el('div', { class: 'g-row g-hdr' }, el('span', {}, 'seat'), el('span', {}, 'role'), el('span', {}, 'now'), el('span', {}, 'blocked on'));
    agents.append(hdr);
    for (const s of seats) {
      const live = (s.tasks || []).find(t => ['claimed', 'running', 'reported'].includes(t.state));
      const blocked = (s.tasks || []).find(t => t.state === 'blocked');
      const tileCol = (LAUNCH.find(l => l.harness === s.harness) || {}).tile || 'var(--surface-2)';
      agents.append(el('div', { class: 'g-row', 'data-key': s.key }, el('span', { class: 'g-seat' }, el('span', { class: 'tile', style: 'background:' + tileCol }, harnessIcon(s.harness)), el('b', { style: 'color:' + seatColor(s) }, s.name), glyph(s.state, s)), el('span', {}, s.role ? roleMark(s.role) : '', ' ', s.role || 'shell'), el('span', { class: 'g-now' }, live ? [el('span', { class: 'tglyph ' + live.state }, TASK_GLYPH[live.state]), ' ', live.title] : (s.state === 'waiting' ? 'waiting · ' + (s.waitKind || 'answer') : s.state)), el('span', { class: 'g-block' }, blocked ? 'waits on ' + blocked.waits : (s.state === 'waiting' ? 'the operator' : '—'))));
    }
    const tree = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Graph · epics → work')));
    const pre = el('div', { class: 'g-tree' });
    for (const ep of epics) {
      const done = (ep.items || []).filter(i => ['accepted', 'done'].includes(i.state)).length;
      pre.append(el('div', { class: 'g-epic' }, el('span', { class: 'ekind' }, 'epic'), el('b', {}, ep.title), el('span', { class: 'ecount' }, done + '/' + (ep.items || []).length)));
      (ep.items || []).forEach((it, i, arr) => {
        const last = i === arr.length - 1;
        const line = el('div', { class: 'g-item ' + it.state }, el('span', { class: 'g-branch' }, last ? '└─' : '├─'), el('span', { class: 'tglyph' }, TASK_GLYPH[it.state] || '○'), el('span', { class: 'ekind' }, it.type || 'task'), el('span', { class: 'g-title' }, it.title));
        if (it.assignee) line.append(el('span', { class: 'eass' }, '· ' + it.assignee));
        if (it.waits) line.append(el('span', { class: 'g-dep' }, '◀ waits on ' + it.waits));
        pre.append(line);
        for (const a of (it.artifacts || [])) pre.append(el('div', { class: 'g-art' }, el('span', { class: 'g-branch' }, (last ? '   ' : '│  ') + '└─'), el('span', { class: 'aglyph' }, ART_GLYPH[a.kind] || '▢'), el('span', { class: 'ekind' }, a.kind || 'file'), el('span', { class: 'g-title' }, a.ref), a.responsible ? el('span', { class: 'achip', style: 'color:' + (a.responsibleColor || 'var(--fg)') + ';border-color:' + (a.responsibleColor || 'var(--line)') }, a.responsible.split(' ')[0]) : null));
      });
    }
    tree.append(pre);
    // opts.only: render a subset of cards (graph.html deep links); default all
    const want = k => !opts.only || opts.only.includes(k);
    if (want('agents')) root.append(agents);
    if (want('work')) root.append(tree);
    if (opts.subscriptions && want('subscriptions')) root.append(subscriptionsCard(opts.subscriptions));
    if (opts.provenance && want('provenance')) root.append(provenanceCard(opts.provenance));
    const hist = opts.history || [];
    if (hist.length) {
      const h = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Artifact history · ' + hist.length)));
      for (const ev of hist.slice(-12).reverse()) h.append(el('div', { class: 'g-hist' }, el('span', { class: 'htime' }, ev.time), el('span', { class: 'hseat', style: 'color:' + (ev.seatColor || 'var(--fg)') }, ev.seat), el('span', { class: 'hev ' + ev.event }, ev.event), el('span', { class: 'aglyph' }, ART_GLYPH[ev.kind] || '▢'), el('span', { class: 'href' }, ev.ref), el('span', { class: 'hgrade ' + (ev.grade || 'inferred') }, ev.grade || 'inferred')));
      if (want('history')) root.append(h);
    }
    return root;
  }

  // Epic group: a collapsible header in the stream dock with the epic's specs and tasks as lines (work-items.md)
  function epicGroup(epic, opts) {
    opts = opts || {};
    const items = epic.items || [], done = items.filter(i => ['accepted', 'done'].includes(i.state)).length;
    const g = el('div', { class: 'epic' + (epic.open === false ? ' shut' : '') });
    const head = el('div', { class: 'ehead', onclick: () => { epic.open = epic.open === false; if (opts.onToggle) opts.onToggle(epic); } }, el('span', { class: 'ecaret' }, epic.open === false ? '▸' : '▾'), el('span', { class: 'ekind' }, 'epic'), el('span', { class: 'etitle' }, epic.title), el('span', { class: 'ecount' }, done + '/' + items.length));
    g.append(head);
    if (epic.open !== false) for (const it of items) {
      const line = el('div', { class: 'eitem ' + it.state }, el('span', { class: 'tglyph' }, TASK_GLYPH[it.state] || '○'), el('span', { class: 'ekind' }, it.type || 'task'), el('span', { class: 'etitle' }, it.title));
      if (it.assignee) line.append(el('span', { class: 'eass' }, it.assignee));
      if (it.state === 'blocked' && it.waits) line.append(el('span', { class: 'twaits' }, 'waits on ' + it.waits));
      g.append(line);
    }
    return g;
  }

  // Artifact row: a leaf of the provenance graph under its task. Kind glyph · ref · short digest · responsible-seat chip (artifact_claim).
  const ART_GLYPH = { markdown: '▤', pr: '⎇', binary: '▣', file: '▢', image: '▧', release: '◆' };
  function artifactRow(art, opts) {
    opts = opts || {};
    const row = el('div', { class: 'art-row', title: (art.kind || 'file') + ' · ' + art.ref + (art.digest ? ' · ' + art.digest : '') + (art.produced_by ? ' · produced by ' + art.produced_by : '') });
    row.append(el('span', { class: 'aglyph' }, ART_GLYPH[art.kind] || '▢'), el('span', { class: 'akind' }, art.kind || 'file'), el('span', { class: 'aref' }, art.ref));
    const right = el('span', { class: 'right' });
    if (art.digest) right.append(el('span', { class: 'adigest' }, art.digest.slice(0, 7)));
    if (art.responsible) right.append(el('span', { class: 'achip', style: 'color:' + (art.responsibleColor || 'var(--fg)') + ';border-color:' + (art.responsibleColor || 'var(--line)'), title: 'responsible: ' + art.responsible + ' (artifact_claim)' }, art.responsible.split(' ')[0]));
    row.append(right);
    return row;
  }

  // Wikilinks with the annotation form: [[slug]] · [[slug|display]] · [[slug]]{rel:w}. Rendered as a link chip plus a relation badge.
  const LINK_RE = /\[\[([^\]|]+)(?:\|([^\]]+))?\]\](?:\{([a-z_]+)(?::([0-9.]+))?\})?/g;
  function renderLinks(text) {
    const frag = el('span', { class: 'links' }); let last = 0, m;
    while ((m = LINK_RE.exec(text))) {
      if (m.index > last) frag.append(text.slice(last, m.index));
      const chip = el('span', { class: 'wl', title: m[1] + (m[3] ? ' · ' + m[3] + (m[4] ? ' · weight ' + m[4] : '') : '') }, el('span', { class: 'wl-slug' }, m[2] || m[1]));
      if (m[3]) chip.append(el('span', { class: 'wl-rel ' + m[3] }, m[3] + (m[4] ? ' ' + m[4] : '')));
      frag.append(chip); last = m.index + m[0].length;
    }
    if (last < text.length) frag.append(text.slice(last));
    return frag;
  }

  // Event bus (events.md, Bus design): subject tokens coloured by root, the kind (last token) emphasised, seq per stream,
  // tier mark (stored ● appended before delivery, fan-out ○ live only), request-reply pairs linked, gaps spelled.
  const EV_ROOT = { seat: '#2EE6A8', doc: '#58A6FF', task: '#B07BFF', swarm: '#FFC53D', ext: '#FF7A45', mux: '#9AA3C7', inbox: '#E96BFF', artifact: '#4FD1C5', dm: '#FF4FA3', channel: '#F2D16B', cloud: '#58D7FF' };
  const evTier = subj => /^inbox\//.test(subj) || /^seat\/[^/]+\/output$/.test(subj) ? 'fanout' : 'stored';
  function subjectChip(subj) {
    const t = subj.split('/'), root = t[0], c = EV_ROOT[root] || 'var(--fg-dim)';
    const out = el('span', { class: 'subj', style: '--root:' + c });
    t.forEach((tok, i) => { if (i) out.append(el('i', { class: 'sl' }, '/')); out.append(el('span', { class: i === 0 ? 'root' : i === t.length - 1 ? 'kind' : 'tok' }, tok)); });
    return out;
  }
  function eventRow(ev) {
    if (ev.gap) return el('div', { class: 'ev-gap' }, el('i'), el('span', {}, 'gap · seq ' + ev.gap[0] + '–' + ev.gap[1] + ' not delivered to ' + ev.consumer + ' · ' + (ev.why || 'queue_max reached') + ' · replay from ' + (ev.gap[1] + 1)), el('i'));
    const tier = evTier(ev.subject);
    const row = el('div', { class: 'ev-row ' + tier + (ev.pair ? ' pair' : '') + (ev.fresh ? ' fresh' : ''), 'data-pair': ev.pair || '' },
      el('span', { class: 'ev-seq' }, tier === 'fanout' ? '—' : String(ev.seq)), el('span', { class: 'ev-ts' }, ev.ts),
      el('span', { class: 'ev-tier', title: tier === 'fanout' ? 'fan-out: live consumers only, never appended' : 'stored: appended to events.sqlite before delivery' }, tier === 'fanout' ? '○' : '●'),
      subjectChip(ev.subject),
      el('span', { class: 'ev-src', style: ev.srcColor ? 'color:' + ev.srcColor : '' }, ev.source || ''),
      el('span', { class: 'ev-pay' }, ev.payload || ''),
      ev.reply_to ? el('span', { class: 'ev-rr', title: 'request: reply_to ' + ev.reply_to }, '⇢ ' + ev.reply_to.split('/').slice(-1)[0]) : ev.reply_of ? el('span', { class: 'ev-rr', title: 'reply to seq ' + ev.reply_of }, '⇠ #' + ev.reply_of) : null,
      ev.coalesced ? el('span', { class: 'ev-co', title: 'latest-wins coalescing per subject' }, '×' + ev.coalesced) : null,
      el('span', { class: 'ev-sig' + (ev.signer ? ' ok' : ''), title: ev.signer ? 'signed by ' + ev.signer : 'unsigned' }, ev.signer ? '✓' : '·'));
    if (ev.pair) { row.addEventListener('pointerenter', () => row.parentElement && row.parentElement.querySelectorAll('[data-pair="' + ev.pair + '"]').forEach(r => r.classList.add('lit'))); row.addEventListener('pointerleave', () => row.parentElement && row.parentElement.querySelectorAll('.lit').forEach(r => r.classList.remove('lit'))); }
    return row;
  }
  function consumerCard(c) {
    const fill = Math.min(1, (c.pending || 0) / (c.queue_max || 1));
    const kv = (k, v) => [el('span', { class: 'k' }, k), el('span', { class: 'v' }, v)];
    return el('div', { class: 's-card cons ' + (c.state || 'live') },
      el('div', { class: 's-card-h' }, el('span', {}, c.name ? c.name : 'ephemeral'), el('span', { class: 'cons-dur' }, c.name ? 'durable' : 'dies with its connection'), el('span', { class: 'cons-st ' + (c.state || 'live') }, c.state || 'live')),
      el('div', { class: 'cons-grid' }, ...kv('filter', el('span', { class: 'pats' }, ...c.filter.map(f => el('code', {}, f)))), ...kv('start', c.start), ...kv('ack', c.ack + (c.ack === 'explicit' ? ' · ack_wait ' + (c.ack_wait || '30s') + ' · max_deliver ' + (c.max_deliver || 5) : '')),
        ...kv('deliver', c.deliver), ...kv('cursor', el('code', {}, 'seq ' + c.cursor)), ...kv('pending', el('span', { class: 'qbar' }, el('span', { class: 'track' }, el('i', { style: 'width:' + Math.round(fill * 100) + '%' + (fill > .85 ? ';background:var(--c-waiting)' : '') })), el('b', {}, (c.pending || 0) + ' / ' + c.queue_max)))));
  }
  // Diagrams (document-viewer.md, Mermaid subset built ourselves): flowchart TD/LR, sequence, state. Parsed here, laid out in
  // layers, drawn with the chrome look: rounded nodes tinted in role/seat colours, hairline edges with rounded bends, arrowheads,
  // labels on the type scale. Unsupported syntax renders the raw block with a note, never a broken diagram.
  const DG_ROLE = { oracle: 'var(--role-oracle)', executor: 'var(--role-executor)', creativity: 'var(--role-creativity)', idle: 'var(--c-idle)', waiting: 'var(--c-waiting)', errored: 'var(--c-errored)', accent: 'var(--accent)' };
  function parseMermaid(src) {
    const lines = src.split('\n').map(l => l.replace(/%%.*$/, '').trim()).filter(Boolean);
    const head = (lines.shift() || '').toLowerCase();
    if (/^(flowchart|graph)\b/.test(head)) {
      const dir = /\b(lr|rl)\b/.test(head) ? 'LR' : 'TD', nodes = new Map(), edges = [];
      const node = tok => { const m = tok.trim().match(/^([A-Za-z0-9_]+)(?:(\[\[?|\(\(?|\{)(.*?)(\]\]?|\)\)?|\}))?(?::::(\w+))?$/); if (!m) throw new Error('node: ' + tok);
        const id = m[1], n = nodes.get(id) || { id, label: id, shape: 'rect' }; if (m[3] != null) { n.label = m[3]; n.shape = m[2] === '(' ? 'stadium' : m[2] === '((' ? 'circle' : m[2] === '{' ? 'diamond' : 'rect'; if (m[2] === '[' && /^\(.*\)$/.test(m[3])) { n.label = m[3].slice(1, -1); n.shape = 'store'; } } if (m[5]) n.cls = m[5]; nodes.set(id, n); return id; };
      for (const l of lines) {
        const cm = l.match(/^class\s+([\w,]+)\s+(\w+)$/); if (cm) { cm[1].split(',').forEach(id => { node(id); nodes.get(id).cls = cm[2]; }); continue; }
        const parts = l.split(/\s*(-->|-\.->|==>)\s*(?:\|([^|]*)\|\s*)?/);
        if (parts.length === 1) { node(l); continue; }
        let prev = node(parts[0]);
        for (let i = 1; i < parts.length; i += 3) { const to = node(parts[i + 2]); edges.push({ from: prev, to, label: parts[i + 1] || '', dashed: parts[i] === '-.->', thick: parts[i] === '==>' }); prev = to; }
      }
      return { type: 'flow', dir, nodes: [...nodes.values()], edges };
    }
    if (/^statediagram/.test(head)) {
      const nodes = new Map(), edges = []; let sN = 0;
      const st = (tok, side) => { tok = tok.trim(); if (tok === '[*]') { const id = side === 'from' ? '__start' : '__end' + (sN++); nodes.set(id, { id, label: '', shape: side === 'from' ? 'start' : 'end' }); return id; } const m = tok.match(/^(\w+)(?::::(\w+))?$/); if (!m) throw new Error('state: ' + tok); if (!nodes.has(m[1])) nodes.set(m[1], { id: m[1], label: m[1], shape: 'stadium' }); if (m[2]) nodes.get(m[1]).cls = m[2]; return m[1]; };
      for (const l of lines) { const m = l.match(/^(.+?)\s*-->\s*([^:]+?)(?:\s*:\s*(.*))?$/); if (!m) throw new Error('state line: ' + l); edges.push({ from: st(m[1], 'from'), to: st(m[2], 'to'), label: m[3] || '' }); }
      return { type: 'flow', dir: 'TD', nodes: [...nodes.values()], edges };
    }
    if (/^sequencediagram/.test(head)) {
      const parts = [], msgs = [], cls = {};
      const P = n => { n = n.trim(); if (!parts.includes(n)) parts.push(n); return n; };
      for (const l of lines) {
        let m;
        if ((m = l.match(/^(participant|actor)\s+(\w+)(?:\s+as\s+(.+?))?(?::::(\w+))?$/))) { P(m[2]); if (m[4]) cls[m[2]] = m[4]; if (m[3]) cls['label:' + m[2]] = m[3]; continue; }
        if ((m = l.match(/^note\s+over\s+([\w,\s]+):\s*(.*)$/i))) { msgs.push({ note: m[2], over: m[1].split(',').map(x => P(x)) }); continue; }
        if ((m = l.match(/^(\w+)\s*(-->>|->>|-->|->|-x)\s*(\w+)\s*:\s*(.*)$/))) { msgs.push({ from: P(m[1]), to: P(m[3]), text: m[4], dashed: m[2].startsWith('--'), lost: m[2] === '-x' }); continue; }
        throw new Error('sequence line: ' + l);
      }
      return { type: 'seq', parts, msgs, cls };
    }
    throw new Error('unsupported diagram: ' + head);
  }
  function diagram(src) {
    let g; try { g = parseMermaid(src); } catch (e) { return el('div', { class: 'dg-raw' }, el('div', { class: 'dg-note' }, 'not drawn · ' + e.message + ' · shown as source'), el('pre', {}, src)); }
    const NS = 'http://www.w3.org/2000/svg', svg = document.createElementNS(NS, 'svg');
    const mk = (tag, a, parent) => { const e = document.createElementNS(NS, tag); for (const k in a) e.setAttribute(k, a[k]); (parent || svg).append(e); return e; };
    const txt = (x, y, t, cls, parent) => { const e = mk('text', { x, y, 'text-anchor': 'middle', 'dominant-baseline': 'central', class: cls || 'dg-t' }, parent); e.textContent = t; return e; };
    const tw = t => Math.max(24, t.length * 6.6);
    const defs = mk('defs', {}); const mkr = mk('marker', { id: 'dg-arrow', viewBox: '0 0 10 10', refX: 9, refY: 5, markerWidth: 7, markerHeight: 7, orient: 'auto-start-reverse' }, defs); mk('path', { d: 'M1,1.5 L9,5 L1,8.5 Z', class: 'dg-head' }, mkr);
    svg.setAttribute('class', 'dg'); svg.style.setProperty('--n', 'var(--fg-dim)');
    if (g.type === 'seq') {
      const colW = 150, top = 14, rowH = 40, W = g.parts.length * colW, xs = {}; g.parts.forEach((p, i) => xs[p] = colW / 2 + i * colW);
      const H = top + 40 + g.msgs.length * rowH + 50;
      const box = (p, y) => { const lab = g.cls['label:' + p] || p, w = tw(lab) + 18, c = DG_ROLE[g.cls[p]] || 'var(--fg-dim)'; const gr = mk('g', { class: 'dg-node', style: '--n:' + c }); mk('rect', { x: xs[p] - w / 2, y, width: w, height: 28, rx: 8 }, gr); txt(xs[p], y + 14, lab, 'dg-t', gr); };
      g.parts.forEach(p => { mk('line', { x1: xs[p], y1: top + 28, x2: xs[p], y2: H - 42, class: 'dg-life' }); box(p, top); box(p, H - 42); });
      g.msgs.forEach((m, i) => { const y = top + 56 + i * rowH;
        if (m.note) { const xa = Math.min(...m.over.map(p => xs[p])), xb = Math.max(...m.over.map(p => xs[p])), w = Math.max(xb - xa + 40, tw(m.note) + 20); mk('rect', { x: (xa + xb) / 2 - w / 2, y: y - 13, width: w, height: 24, rx: 6, class: 'dg-notebox' }); txt((xa + xb) / 2, y - 1, m.note, 'dg-t small'); return; }
        const x1 = xs[m.from], x2 = xs[m.to];
        if (m.from === m.to) { mk('path', { d: `M${x1},${y} h34 q6,0 6,6 v8 q0,6 -6,6 h-34`, class: 'dg-edge' + (m.dashed ? ' dashed' : ''), 'marker-end': 'url(#dg-arrow)' }); txt(x1 + 44 + tw(m.text) / 2, y + 10, m.text, 'dg-t small'); return; }
        mk('line', { x1, y1: y, x2: x2 + (x2 > x1 ? -2 : 2), y2: y, class: 'dg-edge' + (m.dashed ? ' dashed' : ''), 'marker-end': m.lost ? '' : 'url(#dg-arrow)' });
        if (m.lost) txt(x2 + (x2 > x1 ? -6 : 6), y, '×', 'dg-t lost');
        txt((x1 + x2) / 2, y - 9, m.text, 'dg-t small'); });
      svg.setAttribute('viewBox', `0 0 ${W} ${H}`); svg.setAttribute('width', W); return el('div', { class: 'dg-wrap' }, svg);
    }
    // layered layout (Sugiyama-lite): DFS marks back edges; rank = longest path over forward edges; long edges get virtual
    // waypoints so they route around nodes; layers ordered by barycentre sweeps; each back edge gets its own lane outside.
    const rank = {}, out = {}, seen = {};
    g.nodes.forEach(n => { rank[n.id] = 0; out[n.id] = []; }); g.edges.forEach(e => out[e.from].push(e));
    const visit = id => { seen[id] = 1; for (const e of out[id]) { if (seen[e.to] === 1) e.back = true; else if (!seen[e.to]) visit(e.to); } seen[id] = 2; };
    const indeg = {}; g.nodes.forEach(n => indeg[n.id] = 0); g.edges.forEach(e => indeg[e.to]++);
    g.nodes.filter(n => !indeg[n.id]).concat(g.nodes).forEach(n => { if (!seen[n.id]) visit(n.id); });
    for (let pass = 0; pass < g.nodes.length; pass++) for (const e of g.edges) if (!e.back && rank[e.to] < rank[e.from] + 1) rank[e.to] = rank[e.from] + 1;
    const LR = g.dir === 'LR', size = n => n.shape === 'start' || n.shape === 'end' ? [18, 18] : n.shape === 'circle' ? [Math.max(tw(n.label) + 24, 40), Math.max(tw(n.label) + 24, 40)] : n.shape === 'diamond' ? [tw(n.label) + 38, 48] : [tw(n.label) + 22, 32];
    const V = {}, layers = [];
    const put = v => (layers[v.r] = layers[v.r] || []).push(v);
    g.nodes.forEach(n => { const z = size(n); V[n.id] = { id: n.id, n, w: z[0], h: z[1], r: rank[n.id] }; put(V[n.id]); });
    const succ = {}, pred = {}, link = (x, y) => { (succ[x] = succ[x] || []).push(y); (pred[y] = pred[y] || []).push(x); };
    g.edges.forEach((e, i) => { if (e.back) return; e.chain = [e.from]; for (let r = rank[e.from] + 1; r < rank[e.to]; r++) { const id = '~' + i + '_' + r; V[id] = { id, virtual: true, w: LR ? 2 : 10, h: LR ? 10 : 2, r }; put(V[id]); e.chain.push(id); } e.chain.push(e.to); for (let k = 1; k < e.chain.length; k++) link(e.chain[k - 1], e.chain[k]); });
    const bary = (L, nb, ref) => { const idx = Object.fromEntries((ref || []).map((v, i) => [v.id, i])); L.forEach((v, i) => { const ps = (nb[v.id] || []).filter(x => x in idx).map(x => idx[x]); v.b = ps.length ? ps.reduce((a, b) => a + b, 0) / ps.length : i; }); L.sort((x, y) => x.b - y.b); };
    for (let sweep = 0; sweep < 3; sweep++) { for (let r = 1; r < layers.length; r++) if (layers[r]) bary(layers[r], pred, layers[r - 1]); for (let r = layers.length - 2; r >= 0; r--) if (layers[r]) bary(layers[r], succ, layers[r + 1]); }
    const gapMain = LR ? 74 : 58, gapCross = LR ? 22 : 28; let main = 14, crossMax = 0;
    layers.forEach(L => { if (!L) return; const thick = Math.max(...L.map(v => LR ? v.w : v.h)), tot = L.reduce((a, v) => a + (LR ? v.h : v.w), 0) + gapCross * (L.length - 1); crossMax = Math.max(crossMax, tot); let c = 0; L.forEach(v => { v.m = main + thick / 2; v.c = c + (LR ? v.h : v.w) / 2; v.tot = tot; c += (LR ? v.h : v.w) + gapCross; }); main += thick + gapMain; });
    Object.values(V).forEach(v => { v.c += (crossMax - v.tot) / 2 + 14; v.x = LR ? v.m : v.c; v.y = LR ? v.c : v.m; });
    const pos = V, R = 8, backs = g.edges.filter(e => e.back), mid = 14 + crossMax / 2;
    backs.forEach(e => { e.side = LR ? 'bottom' : V[e.to].x < mid - 1 ? 'left' : 'right'; });
    const nLeft = backs.filter(e => e.side === 'left').length, nRight = backs.filter(e => e.side === 'right').length, padL = nLeft ? 20 + nLeft * 14 : 0;
    if (padL) Object.values(V).forEach(v => { v.x += padL; });
    const W = (LR ? main - gapMain : crossMax) + 28 + padL + (!LR && nRight ? 24 + nRight * 14 : 0), H = (LR ? crossMax : main - gapMain) + 28 + (LR && backs.length ? 20 + backs.length * 14 : 0);
    // orthogonal step between two points along the main axis, rounded corners
    const step = (ax, ay, bx, by) => { if (LR) { if (Math.abs(ay - by) < 1) return ` H${bx}`; const mx = (ax + bx) / 2, r = Math.min(R, Math.abs(by - ay) / 2, Math.abs(mx - ax)), sy = Math.sign(by - ay); return ` H${mx - r} Q${mx},${ay} ${mx},${ay + sy * r} V${by - sy * r} Q${mx},${by} ${mx + r},${by} H${bx}`; }
      if (Math.abs(ax - bx) < 1) return ` V${by}`; const my = (ay + by) / 2, r = Math.min(R, Math.abs(bx - ax) / 2, Math.abs(my - ay)), sx = Math.sign(bx - ax); return ` V${my - r} Q${ax},${my} ${ax + sx * r},${my} H${bx - sx * r} Q${bx},${my} ${bx},${my + r} V${by}`; };
    for (const e of g.edges) {
      if (e.back) continue;
      const pts = e.chain.map(id => V[id]), a = pts[0], b = pts[pts.length - 1];
      let x = LR ? a.x + a.w / 2 : a.x, y = LR ? a.y : a.y + a.h / 2, d = `M${x},${y}`;
      for (let k = 1; k < pts.length; k++) { const q = pts[k], last = k === pts.length - 1; const tx = LR ? (last ? q.x - q.w / 2 - 2 : q.x) : q.x, ty = LR ? q.y : (last ? q.y - q.h / 2 - 2 : q.y); d += step(x, y, tx, ty); x = tx; y = ty; }
      mk('path', { d, class: 'dg-edge' + (e.dashed ? ' dashed' : '') + (e.thick ? ' thick' : ''), 'marker-end': 'url(#dg-arrow)' });
      if (e.label) { const q = pts[1]; const lx = LR ? (a.x + a.w / 2 + (q.x - (q.virtual ? 0 : q.w / 2))) / 2 : q.x, ly = LR ? q.y : (a.y + a.h / 2 + q.y - (q.virtual ? 0 : q.h / 2)) / 2 + (q.virtual ? 0 : 6); txt(lx, ly, e.label, 'dg-t small pill'); }
    }
    backs.forEach((e, i) => { const a = V[e.from], b = V[e.to];
      if (LR) { const yb = 14 + crossMax + 14 + i * 14, ya = a.y + a.h / 2, yt = b.y + b.h / 2 + 2; mk('path', { d: `M${a.x},${ya} V${yb - R} Q${a.x},${yb} ${a.x - R},${yb} H${b.x + R} Q${b.x},${yb} ${b.x},${yb - R} V${yt}`, class: 'dg-edge back' + (e.dashed ? ' dashed' : ''), 'marker-end': 'url(#dg-arrow)' }); if (e.label) txt((a.x + b.x) / 2, yb, e.label, 'dg-t small pill'); }
      else {
        const right = e.side === 'right', k = backs.filter(x => x.side === e.side).indexOf(e), lane = right ? padL + 14 + crossMax + 16 + k * 14 : padL - 6 - k * 14, dir = right ? 1 : -1;
        // exit sideways only when no node of the source's layer sits between it and the lane; otherwise leave from the top into the inter-layer gap
        const blocked = (layers[a.r] || []).some(v => !v.virtual && v !== a && (right ? v.x > a.x : v.x < a.x));
        const xt = b.x + dir * (b.w / 2 + 2);
        let d;
        if (!blocked) { const xa = a.x + dir * a.w / 2; d = `M${xa},${a.y} H${lane - dir * R} Q${lane},${a.y} ${lane},${a.y - R} V${b.y + R} Q${lane},${b.y} ${lane - dir * R},${b.y} H${xt}`; }
        else { const ya = a.y - a.h / 2, yg = ya - 12 - k * 4; d = `M${a.x},${ya} V${yg + R} Q${a.x},${yg} ${a.x + dir * R},${yg} H${lane - dir * R} Q${lane},${yg} ${lane},${yg - R} V${b.y + R} Q${lane},${b.y} ${lane - dir * R},${b.y} H${xt}`; }
        mk('path', { d, class: 'dg-edge back' + (e.dashed ? ' dashed' : ''), 'marker-end': 'url(#dg-arrow)' }); if (e.label) txt(lane, (a.y + b.y) / 2 - (blocked ? 6 : 0), e.label, 'dg-t small pill');
      }
    });
    // labels on edges sit above lines: move pill texts to the end, nodes last
    for (const n of g.nodes) { const p = pos[n.id], c = DG_ROLE[n.cls] || (n.cls ? 'var(--accent)' : 'var(--fg-dim)'); const gr = mk('g', { class: 'dg-node' + (n.cls ? ' tinted' : ''), style: '--n:' + c });
      if (n.shape === 'start') mk('circle', { cx: p.x, cy: p.y, r: 7, class: 'dg-dot' }, gr);
      else if (n.shape === 'end') { mk('circle', { cx: p.x, cy: p.y, r: 8, class: 'dg-ring' }, gr); mk('circle', { cx: p.x, cy: p.y, r: 4.5, class: 'dg-dot' }, gr); }
      else if (n.shape === 'diamond') mk('path', { d: `M${p.x},${p.y - p.h / 2} L${p.x + p.w / 2},${p.y} L${p.x},${p.y + p.h / 2} L${p.x - p.w / 2},${p.y} Z`, 'stroke-linejoin': 'round' }, gr);
      else if (n.shape === 'circle') mk('circle', { cx: p.x, cy: p.y, r: p.w / 2 }, gr);
      else if (n.shape === 'store') { mk('rect', { x: p.x - p.w / 2, y: p.y - p.h / 2, width: p.w, height: p.h, rx: 12 }, gr); mk('path', { d: `M${p.x - p.w / 2 + 4},${p.y - p.h / 2 + 7} Q${p.x},${p.y - p.h / 2 + 13} ${p.x + p.w / 2 - 4},${p.y - p.h / 2 + 7}`, style: 'fill:none' }, gr); }
      else mk('rect', { x: p.x - p.w / 2, y: p.y - p.h / 2, width: p.w, height: p.h, rx: n.shape === 'stadium' ? p.h / 2 : 8 }, gr);
      if (n.label) txt(p.x, p.y, n.label, 'dg-t', gr); }
    svg.querySelectorAll('text.pill').forEach(t => { const w = t.textContent.length * 5.8 + 10; const r = mk('rect', { x: +t.getAttribute('x') - w / 2, y: +t.getAttribute('y') - 8, width: w, height: 16, rx: 8, class: 'dg-pill' }); svg.append(r, t); });
    svg.setAttribute('viewBox', `0 0 ${W} ${H}`); svg.setAttribute('width', W);
    return el('div', { class: 'dg-wrap' }, svg);
  }
  // Document view (work-items.md): frontmatter as a header, Markdown body with annotated wikilinks, a provenance strip, and a right
  // rail with the item's neighbourhood in the markdown graph (weights as stroke). Broken frontmatter shows as broken, never skipped.
  const DOC_HUE = { epic: '#B07BFF', spec: '#58A6FF', task: '#2EE6A8', bug: '#FF6B6B', note: '#9AA3C7', invariant: '#F2C14E', artifact: '#4FD1C5' };
  function mdLite(src) {
    const out = el('div', { class: 'md' }); let list = null, code = null;
    for (const line of src.split('\n')) {
      if (line.startsWith('```')) { if (list && !code) { out.append(list); list = null; } if (code) { out.append(code.mermaid ? diagram(code.textContent) : code); code = null; } else { code = el('pre', {}, ''); code.mermaid = /mermaid/.test(line); } continue; }
      if (code) { code.textContent += (code.textContent ? '\n' : '') + line; continue; }
      const li = line.match(/^\s*[-*] (.*)/), h = line.match(/^(#{1,3}) (.*)/);
      if (!li && list) { out.append(list); list = null; }
      if (h) out.append(el('h' + (h[1].length + 1), {}, renderLinks(h[2])));
      else if (li) { list = list || el('ul'); list.append(el('li', {}, renderLinks(li[1]))); }
      else if (line.trim()) out.append(el('p', {}, renderLinks(line)));
    }
    if (list) out.append(list); if (code) out.append(code);
    return out;
  }
  function neighbourhood(doc) {
    const W = 260, H = 230, cx = W / 2, cy = H / 2 - 6, n = (doc.near || []).length, NS = 'http://www.w3.org/2000/svg';
    const svg = document.createElementNS(NS, 'svg'); svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H); svg.setAttribute('class', 'nbhd');
    const mk = (tag, a) => { const e = document.createElementNS(NS, tag); for (const k in a) e.setAttribute(k, a[k]); svg.append(e); return e; };
    (doc.near || []).forEach((x, i) => {
      const t = -Math.PI / 2 + i * 2 * Math.PI / n, r = 78 - (x.w || 1) * 6, X = cx + Math.cos(t) * r, Y = cy + Math.sin(t) * r;
      mk('line', { x1: cx, y1: cy, x2: X, y2: Y, style: 'stroke:var(--fg-dim);stroke-opacity:.45;stroke-width:' + (0.8 + (x.w || 1) * 0.7) + (x.back ? ';stroke-dasharray:3 3' : '') });
      mk('circle', { cx: X, cy: Y, r: 5, style: 'fill:' + (DOC_HUE[x.type] || 'var(--fg-dim)') });
      const tx = mk('text', { x: X, y: Y + (Y < cy ? -9 : 15), 'text-anchor': 'middle', style: 'fill:var(--fg);font-size:9.5px;font-family:var(--font-mono)' }); tx.textContent = x.slug;
      if (x.rel) { const tr = mk('text', { x: (cx + X) / 2, y: (cy + Y) / 2 - 3, 'text-anchor': 'middle', style: 'fill:var(--fg-dim);font-size:8px' }); tr.textContent = x.rel + (x.w > 1 ? ' ' + x.w : ''); }
    });
    mk('circle', { cx, cy, r: 9, style: 'fill:' + (DOC_HUE[doc.type] || 'var(--accent)') + ';stroke:var(--bg);stroke-width:2' });
    return svg;
  }
  function docView(doc, opts) {
    opts = opts || {};
    const hue = DOC_HUE[doc.type] || 'var(--accent)';
    const chipFor = v => v instanceof Node ? v : typeof v === 'string' && v.startsWith('[[') ? renderLinks(v) : el('span', {}, String(v));
    const fm = el('div', { class: 'doc-fm' });
    const row = (k, v) => { if (v == null || (Array.isArray(v) && !v.length)) return; fm.append(el('span', { class: 'k' }, k), el('span', { class: 'v' }, ...(Array.isArray(v) ? v.map(chipFor) : [chipFor(v)]))); };
    row('id', el('code', {}, doc.id)); row('assignee', doc.assignee ? el('span', { class: 'seatchip', style: 'color:' + (doc.assigneeColor || 'var(--fg)') }, doc.assignee) : null);
    row('role', doc.role); row('parent', doc.parent); row('children', doc.children); row('depends_on', doc.depends_on);
    row('links', (doc.links || []).map(l => renderLinks(l))); row('allowed_paths', (doc.allowed_paths || []).map(x => el('code', {}, x))); row('verify', doc.verify ? el('code', {}, doc.verify) : null);
    row('created', doc.created); row('updated', doc.updated);
    const glyphFor = doc.type === 'task' ? (TASK_GLYPH[doc.status] || '○') + ' ' : '';
    const head = el('div', { class: 'doc-head', style: '--doc:' + hue },
      el('div', { class: 'doc-top' }, el('span', { class: 'doc-type' }, doc.type), el('span', { class: 'doc-status ' + (doc.status || 'none') }, glyphFor + (doc.status || 'current')), el('code', { class: 'doc-path' }, doc.path || '')),
      el('h1', {}, doc.title));
    if (doc.superseded_by) head.append(el('div', { class: 'doc-super' }, 'superseded by ', renderLinks(doc.superseded_by), ' · kept for provenance'));
    if (doc.broken) head.append(el('div', { class: 'doc-broken' }, 'frontmatter does not parse · ' + doc.broken + ' · shown, never skipped'));
    const pv = doc.prov || {};
    const prov = el('div', { class: 'doc-prov' + (pv.signed ? ' signed' : ' unsigned') },
      el('span', {}, 'created by ', el('b', { style: 'color:' + (pv.createdColor || 'var(--fg)') }, pv.created_by || 'unknown')),
      pv.updated_by ? el('span', {}, 'updated by ', el('b', { style: 'color:' + (pv.updatedColor || 'var(--fg)') }, pv.updated_by)) : null,
      pv.derived_from ? el('span', {}, 'derived from ', renderLinks(pv.derived_from)) : null,
      el('span', { class: 'dig' }, el('code', {}, (pv.digest || '').slice(0, 12)), pv.signed ? el('i', { class: 'ok' }, '✓ signed · ' + pv.signed) : el('i', { class: 'no' }, 'unsigned · no write is anonymous: updated_by stamped')));
    const main = el('div', { class: 'doc-main' }, head, fm, mdLite(doc.body || ''), prov);
    const backs = (doc.near || []).filter(x => x.back);
    const rail = el('div', { class: 'doc-rail' }, el('div', { class: 'rail-h' }, 'markdown graph · neighbourhood'), neighbourhood(doc),
      el('div', { class: 'rail-legend' }, ...Object.keys(DOC_HUE).map(t => el('span', {}, el('i', { style: 'background:' + DOC_HUE[t] }), t))),
      backs.length ? el('div', { class: 'rail-h' }, 'backlinks') : null, ...backs.map(x => el('div', { class: 'rail-back' }, el('i', { style: 'background:' + (DOC_HUE[x.type] || 'var(--fg-dim)') }), el('code', {}, x.slug), el('span', {}, x.rel || 'ref'))));
    return el('div', { class: 'doc-view', style: '--doc:' + hue }, main, rail);
  }
  // Document modal (document-viewer.md "What it is"): open, scroll, copy maybe, close, feed into the harness. Centred over the
  // blurred window in the launcher material. Esc closes; ⌘C copies the selection as Markdown source; ⏎ / ⌘⏎ feeds the document or
  // the selection into the focused seat as a message and closes; focus returns where it was; pin as tab exists but is rare.
  function docModal(doc, opts) {
    opts = opts || {};
    const seat = opts.seat, plain = !doc.type;
    const view = plain ? el('div', { class: 'doc-view plain' }, el('div', { class: 'doc-main' }, el('div', { class: 'doc-head' }, el('div', { class: 'doc-top' }, el('code', { class: 'doc-path', style: 'margin-left:0' }, doc.path || '')), el('h1', {}, doc.title || doc.path)), mdLite(doc.body || ''))) : docView(doc);
    const scroller = el('div', { class: 'dm-body' }, view);
    const act = (label, keys, fn, cls) => el('button', { class: 'dm-act' + (cls ? ' ' + cls : ''), onclick: e => { e.stopPropagation(); fn && fn(e.currentTarget); } }, el('span', {}, label), el('kbd', {}, keys));
    const feed = sel => opts.onFeed && opts.onFeed(doc, sel || null);
    const foot = el('div', { class: 'dm-foot' },
      seat ? act('Feed into ' + seat.name, '⏎', () => feed(), 'primary') : act('Feed into the focused seat', '⏎', () => feed(), 'primary'),
      act('Feed selection', '⌘⏎', () => feed(String(window.getSelection() || ''))),
      el('span', { class: 'dm-sep' }),
      act('Copy as Markdown', '⌘C', b => { try { navigator.clipboard && navigator.clipboard.writeText(doc.source || doc.body || '').catch(() => {}); } catch {} b.classList.add('done'); }),
      act('Copy path', '⌘⇧C', b => { try { navigator.clipboard && navigator.clipboard.writeText(doc.path || '').catch(() => {}); } catch {} b.classList.add('done'); }),
      act('Find', '⌘F', () => {}),
      el('span', { class: 'dm-sep' }),
      act('Pin as tab', '', () => opts.onPin && opts.onPin(doc), 'quiet'),
      act('Close', 'esc', () => opts.onClose && opts.onClose(), 'quiet'));
    const panel = el('div', { class: 'doc-modal', role: 'dialog', 'aria-label': doc.title || doc.path, tabindex: '-1' }, scroller, foot);
    panel.addEventListener('keydown', e => {
      e.stopPropagation();
      if (e.key === 'Escape') { e.preventDefault(); opts.onClose && opts.onClose(); }
      else if (e.key === 'Enter') { e.preventDefault(); feed(e.metaKey ? String(window.getSelection() || '') : null); }
      else if (e.key === ' ' || e.key === 'PageDown') { e.preventDefault(); scroller.scrollBy({ top: scroller.clientHeight * .85, behavior: 'smooth' }); }
      else if (e.key === 'PageUp') { e.preventDefault(); scroller.scrollBy({ top: -scroller.clientHeight * .85, behavior: 'smooth' }); }
      else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); scroller.scrollBy({ top: e.key === 'ArrowDown' ? 40 : -40 }); }
    });
    return panel;
  }
  // Provenance card: created_by / updated_by, weighted derived_from, the signed digest pair, and the hash-chained activity log
  function provenanceCard(prov) {
    const card = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Provenance · ' + (prov.items || []).length + ' items · log ' + (prov.log || []).length)));
    for (const it of (prov.items || []).slice(0, 6)) {
      const row = el('div', { class: 'p-item' });
      row.append(el('div', { class: 'p-head' }, el('span', { class: 'ekind' }, it.type || 'note'), el('b', {}, it.title), el('span', { class: 'p-by' }, 'created by ', el('span', { style: 'color:' + (it.createdColor || 'var(--fg)') }, it.created_by), it.updated_by && it.updated_by !== it.created_by ? [' · updated by ', el('span', { style: 'color:' + (it.updatedColor || 'var(--fg)') }, it.updated_by)] : null)));
      if (it.derived_from) row.append(el('div', { class: 'p-derived' }, el('span', { class: 'p-lbl' }, 'derived from'), renderLinks(it.derived_from)));
      if (it.digest) row.append(el('div', { class: 'p-digest' }, el('span', { class: 'p-lbl' }, 'digest'), el('code', {}, it.digest.content.slice(0, 12)), el('span', { class: 'p-sig' + (it.digest.signed ? ' ok' : '') }, it.digest.signed ? '✓ signed · ' + it.digest.by : 'unsigned')));
      card.append(row);
    }
    if ((prov.log || []).length) {
      card.append(el('div', { class: 'p-loghead' }, 'activity log · .bboxy/log/ · hash-chained'));
      for (const ev of prov.log.slice(-6).reverse()) card.append(el('div', { class: 'p-log' + (ev.checkpoint ? ' cp' : '') }, el('code', { class: 'p-hash' }, ev.hash.slice(0, 7)), el('span', { class: 'p-time' }, ev.time), el('span', { class: 'p-text' }, ev.checkpoint ? '✓ daily checkpoint · signed by ' + ev.by : ev.text)));
    }
    return card;
  }

  // Subscriptions card: each seat and the topic patterns it follows (events.md); auto-subscriptions are badged
  function subscriptionsCard(subs) {
    const card = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Subscriptions · ' + subs.length + ' seats')));
    for (const s of subs) {
      const row = el('div', { class: 'sub-row' }, el('span', { class: 'sub-seat', style: 'color:' + (s.color || 'var(--fg)') }, s.seat));
      const pats = el('span', { class: 'sub-pats' });
      for (const pt of s.patterns) pats.append(el('span', { class: 'sub-pat' + (pt.auto ? ' auto' : ''), title: pt.auto ? 'auto-subscribed: ' + pt.why : 'subscribed' }, el('code', {}, pt.pattern), pt.auto ? el('i', {}, 'auto') : null));
      row.append(pats);
      card.append(row);
    }
    return card;
  }
  // External-events lane: GitHub, CI, cloud, cron, operator webhooks as compact lines with a source chip
  const EXT_TILE = { github: '#24292F', ci: '#2E6BFF', cloud: '#0EA5E9', cron: '#7C3AED', webhook: '#FF7A45' };
  function externalLane(events, opts) {
    opts = opts || {};
    const lane = el('div', { class: 'ext-lane' + (opts.open === false ? ' shut' : '') });
    lane.append(el('div', { class: 'ext-head', onclick: () => { if (opts.onToggle) opts.onToggle(); } }, el('span', { class: 'ecaret' }, opts.open === false ? '▸' : '▾'), el('span', { class: 'ekind' }, 'external'), el('span', { class: 'ext-count' }, events.length + ' events')));
    if (opts.open !== false) for (const ev of events.slice(-5).reverse()) lane.append(el('div', { class: 'ext-ev' }, el('span', { class: 'ext-src', style: 'background:' + (EXT_TILE[ev.source] || '#374151') }, ev.source), el('span', { class: 'ext-text' }, ev.text), el('span', { class: 'ext-time' }, ev.time)));
    return lane;
  }

  function miniGrid(tree, focusedKey, big) {
    function walk(t) {
      if (t.key) {
        const c = el('div', { class: 'cell' + (t.key === focusedKey ? ' focus' : '') + (t.state === 'waiting' || t.state === 'errored' ? ' ' + t.state : ''), title: t.name });
        if (big) c.textContent = t.name.split(' ')[0];
        return c;
      }
      const A = walk(t.a), B = walk(t.b);
      if (t.ratio) { A.style.flex = t.ratio; B.style.flex = 1 - t.ratio; }
      return el('div', { class: 'mg ' + t.dir }, A, B);
    }
    const root = tree.key ? el('div', { class: 'mg' }, walk(tree)) : walk(tree);
    root.classList.add('mg'); if (big) root.classList.add('big');
    return root;
  }

  function tabRow(tab, opts) {
    opts = opts || {};
    const seats = tab.seats || leaves(tab.tree);
    if (seats.length === 1) {
      const s = seats[0];
      const w = el('div', {}, seatRow(s, { focused: s.key === opts.focusedKey, shortcut: tab.shortcut }), ...subagentRows(s));
      for (const t of (s.tasks || [])) { w.append(taskRow(t, s, { onAccept: opts.onAccept })); for (const a of (t.artifacts || [])) w.append(artifactRow(a)); }
      return w;
    }
    const row = el('div', { class: 'tab-row' }, el('span', { class: 'caret' }, opts.expanded ? '▾' : '▸'), el('span', { class: 'tname' }, tab.name));
    if (!opts.expanded) {
      const roll = el('span', { class: 'roll' });
      const rolled = rollup(seats).slice(0, T.sidebar.collapse_glyphs);
      for (const s of rolled) roll.append(glyph(s.state, s));
      if (seats.length > T.sidebar.collapse_glyphs) roll.append(el('span', { class: 'ellip' }, '…'));
      row.append(roll);
    }
    const right = el('span', { class: 'right' });
    if (tab.shortcut) right.append(kbd(tab.shortcut));
    right.append(miniGrid(tab.tree, opts.focusedKey));
    row.append(right);
    if (opts.onToggle) row.addEventListener('click', opts.onToggle);
    const wrap = el('div', {}, row);
    if (opts.expanded) {
      const body = el('div', { class: 'tab-body' });
      for (const s of seats) { body.append(seatRow(s, { focused: s.key === opts.focusedKey }), ...subagentRows(s)); for (const t of (s.tasks || [])) { body.append(taskRow(t, s, { onAccept: opts.onAccept })); for (const a of (t.artifacts || [])) body.append(artifactRow(a)); } }
      wrap.append(body);
    }
    return wrap;
  }

  function spaceHeader(space, opts) {
    opts = opts || {};
    const seats = space.tabs.flatMap(t => t.seats || leaves(t.tree));
    const waiting = seats.filter(s => s.state === 'waiting' || s.state === 'errored').length;
    const nameEl = el('span', { class: 'sname' }, space.name);
    const head = el('div', { class: 'space-head' }, el('span', { class: 'caret' }, opts.expanded ? '▾' : '▸'), nameEl, el('span', { class: 'count' }, seats.length));
    // rename in place: double-click the band (or opts.editName on birth); Enter / blur commit, Escape cancels
    const edit = () => {
      if (!opts.onRename) return;
      const inp = el('input', { class: 'sname-edit', value: space.name, onclick: e => e.stopPropagation() });
      const done = ok => { const v = inp.value.trim(); inp.replaceWith(nameEl); if (ok && v) { space.name = v; nameEl.textContent = v; opts.onRename(space, v); } };
      inp.addEventListener('keydown', e => { if (e.key === 'Enter') { e.preventDefault(); done(true); } if (e.key === 'Escape') done(false); e.stopPropagation(); });
      inp.addEventListener('blur', () => done(true));
      nameEl.replaceWith(inp); inp.focus(); inp.select();
    };
    head.addEventListener('dblclick', e => { e.stopPropagation(); edit(); });
    if (opts.editName) requestAnimationFrame(edit);
    if (!opts.expanded) {
      const roll = el('span', { class: 'roll' });
      for (const s of rollup(seats).slice(0, T.sidebar.collapse_glyphs)) roll.append(glyph(s.state, s));
      if (seats.length > T.sidebar.collapse_glyphs) roll.append(el('span', { class: 'ellip' }, '…'));
      head.append(roll);
    }
    const right = el('span', { class: 'right' });
    if (waiting && !opts.expanded) right.append(el('span', { class: 'badge', style: 'background:var(--c-waiting);color:#000' }, waiting));
    if (opts.shortcut) right.append(kbd(opts.shortcut));
    right.append(el('span', { class: 'swatch', title: 'change colour', onclick: e => { e.stopPropagation(); if (opts.onColor) opts.onColor(space); } }));
    head.append(right);
    if (opts.onToggle) head.addEventListener('click', opts.onToggle);
    return head;
  }

  function spacePill(space, st) {
    const expanded = st.openSpaces.has(space.key);
    const pill = el('div', { class: 'space' + (expanded ? ' open' : ' closed') + (st.currentSpace && st.currentSpace !== space.key ? ' unfocused' : ''), style: '--space:' + seatColor(space), 'data-space': space.key });
    pill.append(spaceHeader(space, { expanded, onColor: st.onColor, onRename: st.onRename, editName: st.editName === space.key, onToggle: () => { if (st.onToggleSpace) { st.onToggleSpace(space); return; } expanded ? st.openSpaces.delete(space.key) : st.openSpaces.add(space.key); st.rerender(); } }));
    if (expanded) {
      const body = el('div', { class: 'space-body' });
      for (const tab of space.tabs) {
        const tkey = space.key + '/' + tab.name;
        const open = st.openTabs.has(tkey) || (tab.seats || leaves(tab.tree)).length === 1 && st.autoOpenSingles;
        body.append(tabRow(tab, { expanded: open, focusedKey: st.focusedKey, onAccept: st.onAccept, onToggle: () => { st.openTabs.has(tkey) ? st.openTabs.delete(tkey) : st.openTabs.add(tkey); st.rerender(); } }));
      }
      for (const c of (space.closed || [])) body.append(el('div', { class: 'tab-row closed-strip', 'data-closed': c.seat.key }, el('span', { class: 'x' }, '×'), harnessIcon(c.seat.harness), el('span', { class: 'tname' }, c.seat.name), el('span', { class: 'right' }, el('span', { class: 'kbd left' }, ''), el('button', { class: 'undo', onclick: e => { e.stopPropagation(); st.onUndo && st.onUndo(c); } }, 'undo ⌘Z'))));
      for (const p of (space.pending || [])) body.append(pendingRow(p, { onDismiss: st.onDismissPending }));
      body.append(el('div', { class: 'tab-row newseat', onclick: () => st.onNewSeat && st.onNewSeat(space) }, el('span', { class: 'plus' }, '+'), el('span', { class: 'tname' }, 'new seat'), el('span', { class: 'right' }, kbd('⌘T'))));
      if (space.participants && space.participants.length) {
        body.append(el('div', { class: 'tab-row participants' }, el('span', { class: 'caret' }, '·'), el('span', { class: 'tname' }, 'Shadows'), el('span', { class: 'right' }, el('span', { class: 'kbd' }, space.participants.length))));
        for (const ph of space.participants) { const row = seatRow(ph, {}); row.classList.add('phantom'); body.append(row); }
      }
      pill.append(body);
    }
    return pill;
  }

  function sidebar(model, opts) {
    opts = opts || {};
    const seats = model.spaces.flatMap(s => s.tabs.flatMap(t => t.seats || leaves(t.tree)));
    const waiting = seats.filter(s => s.state === 'waiting').length;
    const st = { openSpaces: new Set(opts.openSpaces || []), openTabs: new Set(opts.openTabs || []), focusedKey: opts.focusedKey || seats[0].key, autoOpenSingles: seats.length <= 40, onNewSeat: opts.onNewSeat, onColor: opts.onColor, onUndo: opts.onUndo, onDismissPending: opts.onDismissPending, onRename: opts.onRename, editName: opts.editName, onAccept: opts.onAccept };
    const owner = model.spaces.find(sp => sp.tabs.some(t => (t.seats || leaves(t.tree)).some(s => s.key === st.focusedKey)));
    st.currentSpace = opts.currentSpace || (owner ? owner.key : (model.spaces[0] && model.spaces[0].key));
    // cohorts read as tabs: every band shows; open/shut is the caller's set (band click toggles it) and is separate from focus
    const shown = model.spaces;
    st.onToggleSpace = opts.onToggleSpace;
    const sb = el('div', { class: 'sidebar' });
    const scroll = el('div', { class: 'sb-scroll' });
    st.rerender = () => { scroll.innerHTML = ''; for (const s of shown) scroll.append(spacePill(s, st)); if (opts.onNewSpace) scroll.append(el('div', { class: 'new-cohort', onclick: () => opts.onNewSpace() }, el('span', { class: 'plus' }, '+'), 'new swarm')); };
    st.rerender();
    // no footer: the big '+ new swarm' button under the blocks is the one way to create a swarm
    sb.append(
      el('div', { class: 'sb-top' }, harnessIcon('sidebarfox'), el('span', {}, 'bboxy450-term'), el('span', { style: 'margin-left:auto' }, harnessIcon('gear'))),
      scroll,
    );
    sb.seatCount = seats.length; sb.waitingCount = waiting;
    return sb;
  }

  function fakeTerm(seat) {
    const lines = {
      active: '<b>$ cargo build --release</b>\n   Compiling wgpu v0.20\n   Compiling glyphon v0.5\n   Compiling bboxy450-term v0.1.0\n<span class="cur"></span>',
      idle: '<b>$ git status</b>\nOn branch main\nnothing to commit, working tree clean\n<b>$</b> <span class="cur"></span>',
      waiting: 'Allow Bash(git push)?\n  <b>› Yes</b>\n    No\n<span class="cur"></span>',
      errored: 'error[E0599]: no method named `glow`\n  --> src/sidebar.rs:88:14\n<b>$</b> <span class="cur"></span>',
    };
    const t = el('div', { class: 'term', html: seat.term || lines[seat.state] });
    if (seat.gap) t.prepend(gapMarker(seat.gap));
    return t;
  }
  // scrollback gap (mux ReadRecords → Gap): the caption comes only from the supplied retained range; never an invented line count
  function gapMarker(range) {
    const cap = range && range.from ? 'earlier output evicted · retained from ' + range.from : 'earlier output unavailable';
    return el('div', { class: 'gap-mark', title: cap + (range && range.of ? ' · ' + range.of : '') }, el('i'), el('span', {}, cap), el('i'));
  }
  // one-shot notices survive re-renders: a timestamp value resumes the animation at its elapsed point (true = start now)
  const since = v => typeof v === 'number' ? '-' + Math.max(0, Date.now() - v) + 'ms' : '0ms';
  function titleStrip(seat, opts) {
    opts = opts || {};
    const s = el('div', { class: 'strip', style: 'background:' + seatColor(seat) }, el('span', { class: 'close', title: 'close seat', onclick: e => { e.stopPropagation(); if (opts.onClose) opts.onClose(seat); } }));
    if (seat.where && seat.where !== 'local') s.append(el('span', { class: 'where', title: seat.where + (seat.host ? ' · ' + seat.host : '') }, harnessIcon(seat.where)));
    s.append(harnessIcon(seat.harness), el('span', { class: 'name' }, seat.name));
    if (seat.role) s.append(roleMark(seat.role));
    s.append(el('span', { class: 'title' }, (seat.title || '') + (seat.state === 'waiting' ? ' · waiting for ' + (seat.waitKind || 'an answer') : '')));
    if (opts.shortcut) s.append(kbd(opts.shortcut));
    // stop (seat_interrupt, Oracle/operator only: the caller passes onInterrupt only when allowed). A request, not a promise.
    if (seat.interruptSent) { s.classList.add('isent'); s.style.setProperty('--isd', since(seat.interruptSent)); s.append(el('span', { class: 'isent-chip', style: 'animation-delay:' + since(seat.interruptSent), title: 'interrupt sent · the seat decides when it stops' }, 'interrupt sent')); }
    if (opts.onInterrupt) s.append(el('span', { class: 'stop', title: 'interrupt (⌘.) · a request, the seat decides', onclick: e => { e.stopPropagation(); opts.onInterrupt(seat); } }, el('i')));
    if (opts.onMute) s.append(el('span', { class: 'mute' + (seat.muted ? ' on' : ''), title: seat.muted ? 'unmute (⌘M)' : 'mute: 1:1 with the operator (⌘M)', onclick: e => { e.stopPropagation(); opts.onMute(seat); } }, harnessIcon(seat.muted ? 'muted' : 'unmuted')));
    s.append(glyph(seat.state, seat));
    return s;
  }
  function pane(seat, opts) {
    opts = opts || {};
    const p = el('div', { class: 'pane ' + seat.state + (opts.unfocused ? ' unfocused' : '') + (opts.focused ? ' focused' : ''), style: '--seat:' + seatColor(seat) + (opts.height ? ';min-height:' + opts.height + 'px' : '') });
    p.append(titleStrip(seat, opts), fakeTerm(seat));
    // the GUI lost the daemon: keep the stale frame, dim it, freeze the glow, centred pill; never blank, never modal
    if (opts.reconnecting || opts.reconnected) { if (opts.reconnecting) p.classList.add('stale'); p.append(el('div', { class: 'reconnect' + (opts.reconnected ? ' out' : '') }, el('i', { class: 'spin' }), opts.reconnected ? 'reconnected' : 'reconnecting…')); }
    p.dataset.key = seat.key;
    return syncPulse(p, seat.state);
  }

  // ⌘T picker: vertical, keyboard-first, typed prefix, name reveal with reroll
  // Launchables: harnesses and zsh today; browser tab, vnc and ssh later. Rows are data, the picker does not care.
  const LAUNCH = [
    { id: 'claude', label: 'Claude', sub: 'oracle by default', harness: 'claude', kind: 'Harness', tile: '#D97757', section: 'Suggestions' },
    { id: 'codex', label: 'Codex', sub: 'executor by default', harness: 'codex', kind: 'Harness', tile: '#1F2937', section: 'Suggestions' },
    { id: 'pi', label: 'pi', sub: 'executor by default', harness: 'pi', kind: 'Harness', tile: '#7C3AED', section: 'Harnesses' },
    { id: 'omp', label: 'omp', sub: 'executor by default', harness: 'omp', kind: 'Harness', tile: '#0EA5E9', section: 'Harnesses' },
    { id: 'zsh', label: 'zsh', sub: '~', harness: 'zsh', kind: 'Shell', tile: '#111827', section: 'Shells' },
  ];
  // App title bar (design system › elements.title_bar): window dots · sidebar toggle (the fox ear) · the identity PNG · app name ·
  // crumb (swarm · seat) · spacer · theme icon. Height from style.json spacing.title_bar.
  function titleBar(opts) {
    opts = opts || {};
    const toggle = el('span', { class: 'tbtn' + (opts.sidebarOpen !== false ? ' on' : ''), title: 'toggle sidebar (⌘S)', onclick: () => opts.onToggleSidebar && opts.onToggleSidebar() }, harnessIcon(opts.sidebarOpen !== false ? 'sidebarfoxopen' : 'sidebarfox'));
    const crumb = el('span', { class: 'crumb', id: opts.crumbId || null }, opts.crumb || '');
    const theme = opts.themeBtn || el('span', { class: 'tbtn theme', title: 'toggle light / dark', onclick: () => opts.onTheme && opts.onTheme() }, harnessIcon(opts.theme === 'light' ? 'moon' : 'sun'));
    return el('div', { class: 'titlebar app-bar' + (opts.blurred ? ' blurred' : '') }, el('span', { class: 'win' }, el('i'), el('i'), el('i')), toggle, logo(18), el('span', { class: 'ttl' }, opts.title || 'bboxy450-term'), crumb, el('span', { class: 'spacer' }), theme);
  }
  // ⌘T launcher items (design system › modal): launchables ▸ placements; SSH ▸ machine ▸ harness ▸ placement; Cloud ▸ region ▸
  // harness ▸ placement; Go to ▸ every seat and swarm; Graph; Settings ▸ section. Shared by the showcase and launcher.html.
  function launcherItems(ctx) {
    const { seats, spaces, sp, anchorSeat, tabOf } = ctx;
    // rows with sublists: each launchable → placements; 'Go to' → every seat and swarm. 'goto <name>' types straight in.
    const tileOf = h => (LAUNCH.find(l => l.harness === h) || {}).tile || '#374151';
    const goSeats = () => seats.filter(x => !x.settings && !x.graph).map(x => { const t = tabOf(x.key); return { id: 'seat:' + x.key, label: x.name, sub: (t ? t.sp.name + ' · ' : '') + (x.title || '') + (x.where && x.where !== 'local' ? ' · ' + x.where + (x.host ? ' ' + x.host : '') : ''), harness: x.harness, kind: 'Seat', tile: tileOf(x.harness), section: 'Seats', seat: x, tags: [x.harness, x.where || 'local', x.host || '', x.role || '', x.state, x.grade || '', x.state === 'waiting' ? (x.waitKind || 'question') : '', t ? t.sp.name : '', x.muted ? 'muted' : '', ...((x.tasks || []).map(t => t.title))].filter(Boolean) }; });
    const goSwarms = () => spaces.map(x => ({ id: 'swarm:' + x.key, label: x.name, sub: x.tabs.reduce((n, t) => n + t.seats.length, 0) + ' seats', harness: 'sidebarfoxopen', kind: 'Swarm', tile: x.color, section: 'Swarms', swarm: x, tags: ['swarm'] }));
    const placements = l => [
      { id: l.id + ':tab', label: 'New tab', sub: 'in ' + sp.name, harness: l.harness, tile: l.tile, kind: 'Placement', place: 'tab', launch: l },
      ...(anchorSeat ? [{ id: l.id + ':right', label: 'Split right', sub: 'of ' + anchorSeat.name, harness: l.harness, tile: l.tile, kind: 'Placement', place: 'right', launch: l },
                        { id: l.id + ':bottom', label: 'Split down', sub: 'of ' + anchorSeat.name, harness: l.harness, tile: l.tile, kind: 'Placement', place: 'bottom', launch: l }] : []),
    ];
    const launchRows = LAUNCH.map(l => ({ ...l, enterPicks: true, children: () => placements(l), aliases: [l.harness] }));
    // remote: SSH ▸ harness ▸ placement, Cloud ▸ harness ▸ placement. 'ssh codex' and 'cloud claude' type straight through.
    // SSH ▸ machine ▸ harness ▸ placement; Cloud ▸ region ▸ harness ▸ placement. Machines come from ~/.ssh/config in the
    // real app; here a demo list. 'ssh rig codex' types straight through all three levels.
    const HOSTS = { ssh: [['rig', 'rig.local · M2 Ultra'], ['alpha', 'alpha.tail · Linux x86'], ['studio', 'studio.local · M1 Max']], cloud: [['eu-2', 'Hetzner · Falkenstein'], ['us-1', 'Fly · Ashburn'], ['gpu-a', 'Lambda · A100']] };
    const harnessesAt = (where, host) => LAUNCH.map(l => ({ ...l, id: where + ':' + host + ':' + l.id, sub: l.sub + ' · ' + host, enterPicks: true, aliases: [l.harness], where, host, children: () => placements(l).map(pl => ({ ...pl, where, host })) }));
    const remoteRow = (id, label, where, tile) => ({ id, label, aliases: [where], sub: where === 'cloud' ? 'pick a region, then a harness' : 'pick a machine, then a harness', harness: where, kind: 'Remote', tile, section: 'Remote',
      children: () => HOSTS[where].map(([host, desc]) => ({ id: where + ':' + host, label: host, sub: desc, harness: where, kind: where === 'cloud' ? 'Region' : 'Machine', tile, where, host, children: () => harnessesAt(where, host) })) });
    const remoteRows = [remoteRow('ssh', 'SSH', 'ssh', '#374151'), remoteRow('cloud', 'Cloud', 'cloud', '#2E6BFF')];
    const goRow = { id: 'goto', label: 'Go to', aliases: ['goto', 'go', 'jump'], sub: 'a seat or a swarm', harness: 'sidebarfoxopen', kind: 'Navigate', tile: '#6B7190', section: 'Navigate', children: () => [...goSeats(), ...goSwarms()] };
    const settingsRow = { id: 'settings', label: 'Settings', aliases: ['prefs', 'preferences', 'config'], sub: 'account · cloud · machines · roles · harnesses · appearance', harness: 'gear', kind: 'Open', tile: '#6B7190', section: 'Navigate', settingsOpen: true, enterPicks: true, children: () => ['Account', 'Cloud', 'Machines', 'Roles', 'Harnesses', 'Appearance', 'Shortcuts'].map(sct => ({ id: 'settings:' + sct.toLowerCase(), label: sct, harness: 'gear', kind: 'Section', tile: '#6B7190', settingsOpen: true, section: sct })) };
    const graphRow = { id: 'graph', label: 'Graph', aliases: ['who', 'status', 'board'], sub: 'who is on what · epics → work', harness: 'sidebarfoxopen', kind: 'Open', tile: '#6B7190', section: 'Navigate', graphOpen: true, enterPicks: true };
    return [...launchRows, ...remoteRows, goRow, graphRow, settingsRow];
  }
  function picker(opts) {
    opts = opts || {};
    // Nested choices: an item may carry `children` (array or function). Right / Enter dives in, Left / Backspace on an
    // empty field climbs out, and typing flattens: "goto nyx" or "codex split" filters straight into the sublist.
    let q = opts.query || '', sel = 0, seed = opts.seed || 1, stack = [];
    const items0 = opts.items || LAUNCH;
    const box = el('div', { class: 'picker' });
    const input = el('input', { placeholder: opts.placeholder || 'new seat · type to filter', value: q });
    const crumbs = el('span', { class: 'crumbs' });
    const list = el('div');
    const foot = el('div', { class: 'foot' });
    function nameFor(seed) { return rollName(rng('roll' + seed)); }
    const kids = it => typeof it.children === 'function' ? it.children() : (it.children || []);
    const norm = s => (s || '').toLowerCase().replace(/\s+/g, ' ').trim();
    const aliasOf = it => [it.label, ...(it.aliases || [])].map(norm);
    // where are we: the explicit stack, plus a virtual descent typed as "<parent> <rest>"
    function scope() {
      // descend as many levels as the typed words name parents: "ssh alpha cl" → SSH ▸ alpha, query "cl"
      let level = stack.length ? kids(stack[stack.length - 1]) : items0, qq = norm(q); const virt = [];
      for (let guard = 0; guard < 8; guard++) {
        let found = null;
        for (const it of level) { if (!it.children) continue; for (const a of aliasOf(it)) if (qq.startsWith(a + ' ')) { found = it; qq = qq.slice(a.length + 1); break; } if (found) break; }
        if (!found) break;
        virt.push(found); level = kids(found);
      }
      return { level, qq, virt };
    }
    // matching tiers (design system › modal › matching): 1 label / alias prefix, 2 facet tags (harness, where, host,
    // role, state, swarm), 3 subtitle substring, 4 fuzzy subsequence. Tier 5, semantic via the Jev encoder, lives in the
    // real app, not the web demo. Every typed word must hit on some tier; words are ANDed.
    const subseq = (hay, needle) => { let i = 0; for (const ch of hay) if (ch === needle[i]) i++; return i === needle.length; };
    const wordHit = (it, w) => aliasOf(it).some(a => a.startsWith(w)) || norm(it.label).split(' ').some(x => x.startsWith(w)) || (it.tags || []).some(t => norm(t).startsWith(w)) || norm(it.sub).includes(w) || (it.id || '').startsWith(w) || (w.length >= 3 && subseq(norm(it.label).replace(/\s/g, ''), w));
    const hit = (it, qq) => !qq || qq.split(' ').filter(Boolean).every(w => wordHit(it, w));
    const tier = (it, qq) => { if (!qq) return 0; const w = qq.split(' ')[0]; if (aliasOf(it).some(a => a.startsWith(w)) || norm(it.label).split(' ').some(x => x.startsWith(w))) return 1; if ((it.tags || []).some(t => norm(t).startsWith(w))) return 2; if (norm(it.sub).includes(w)) return 3; return 4; };
    const matches = () => { const s = scope(); return s.level.filter(it => hit(it, s.qq)).map((it, i) => [tier(it, s.qq), i, it]).sort((a, b) => a[0] - b[0] || a[1] - b[1]).map(x => x[2]); };
    const path = () => [...stack, ...scope().virt];
    const dive = it => { stack = [...path(), it]; q = ''; input.value = ''; sel = 0; draw(); };
    // opts.path: ids to start inside (deep links: launcher.html?state=ssh-rig-harnesses)
    if (opts.path) { let level = items0; for (const id of opts.path) { const hit = level.find(x => x.id === id); if (!hit) break; stack.push(hit); level = kids(hit); } }
    const climb = () => { if (!stack.length) return; stack = stack.slice(0, -1); q = ''; input.value = ''; sel = 0; draw(); };
    // spawn cap (launcher_cap_notice): launch rows dim and refuse; Go to, Graph and Settings keep working
    const isNav = it => !!(it.seat || it.swarm || it.graphOpen || it.settingsOpen || it.section === 'Navigate' || it.section === 'Swarms' || it.kind === 'Navigate' || it.kind === 'Open');
    const capped = it => !!opts.cap && !isNav(it);
    const flashCap = () => { const b = list.querySelector('.capnote'); if (b) { b.classList.remove('flash'); void b.offsetWidth; b.classList.add('flash'); } };
    const pick = (it, ev) => { if (!it) return; if (capped(it) && !it.children) return flashCap(); if (it.children && !it.enterPicks) return dive(it); if (opts.onPick) opts.onPick(it, nameFor(seed), { meta: !!(ev && (ev.metaKey || ev.ctrlKey)), shift: !!(ev && ev.shiftKey), alt: !!(ev && ev.altKey) }, path()); };
    function draw() {
      list.innerHTML = ''; crumbs.innerHTML = '';
      for (const c of path()) crumbs.append(el('span', { class: 'crumb' }, c.label));
      const items = matches();
      if (opts.cap && !draw.once) { draw.once = true; const k = items.findIndex(x => !capped(x)); if (k >= 0 && capped(items[sel] || {})) sel = k; }
      if (opts.cap) { const c = opts.cap; list.append(el('div', { class: 'capnote', role: 'status' }, el('b', {}, 'Spawn cap reached · ' + c.scope + (c.name ? ' ' + c.name : '') + ' · ' + c.used + ' of ' + c.limit), el('small', {}, (c.counts || 'running, starting and grace processes count') + ' · close a seat or raise the cap in Settings · Go to still works'))); }
      if (!items.length) list.append(el('div', { class: 'empty' }, 'nothing matches "' + q + '"'));
      let lastSec = null;
      items.forEach((it, i) => {
        if (it.section && it.section !== lastSec) { list.append(el('div', { class: 'sec' }, it.section)); lastSec = it.section; }
        const o = el('div', { class: 'opt' + (i === sel ? ' sel' : '') + (it.children ? ' parent' : '') + (capped(it) ? ' capped' : ''), onclick: e => pick(it, e), onpointerenter: () => { if (sel !== i) { sel = i; draw(); } } }, el('span', { class: 'tile', style: 'background:' + (it.tile || 'var(--surface-2)') }, harnessIcon(it.harness)));
        // plain text in every row; the only emphasis is the selection band (operator, Raycast reference)
        const label = el('span', { class: 'lbl' }, it.label);
        o.append(label, el('span', { class: 'sub' }, it.sub || ''), el('span', { class: 'kind' }, it.kind || it.hint || ''));
        o.append(it.children ? el('span', { class: 'chev', title: '→ opens' }, '›') : el('span', { class: 'num' }, String(i + 1)));
        list.append(o);
      });
      const nm = nameFor(seed);
      foot.innerHTML = '';
      foot.append(el('span', { class: 'sw', style: 'background:' + seatColor({ key: nm }), title: 'seat colour' }), el('span', { class: 'nm' }, nm), el('button', { onclick: () => { seed++; draw(); }, title: 'reroll name' }, '↻'), (() => { const pill = el('span', { class: 'pill' }); const acts = (typeof opts.actions === 'function' ? opts.actions(path()) : opts.actions) || [['Open', '↵'], ['Into', '→'], ['Back', '←'], ['Reroll', '⌘R']]; acts.forEach(([l, kb], i) => { if (i) pill.append(el('span', { class: 'sep' })); pill.append(el('span', {}, l), el('kbd', {}, kb)); }); return pill; })());
    }
    input.addEventListener('input', () => { q = input.value; sel = 0; draw(); });
    input.addEventListener('keydown', e => {
      const n = matches().length, cur = matches()[sel];
      if (e.key === 'ArrowDown') { e.preventDefault(); sel = (sel + 1) % Math.max(n, 1); draw(); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); sel = (sel + n - 1) % Math.max(n, 1); draw(); }
      else if (e.key === 'ArrowRight') { if (cur && cur.children) { e.preventDefault(); dive(cur); } }
      else if (e.key === 'ArrowLeft') { if (!q && stack.length) { e.preventDefault(); climb(); } }
      else if (e.key === 'Backspace' && !q && stack.length) { e.preventDefault(); climb(); }
      else if (e.key === 'Enter') { e.preventDefault(); pick(cur, e); }
      else if (e.key === 'Escape') { if (stack.length) climb(); else if (opts.onClose) opts.onClose(); }
      else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') { e.preventDefault(); seed++; draw(); }
      else if (/^[1-9]$/.test(e.key) && !q) { e.preventDefault(); pick(matches()[+e.key - 1], e); }
      e.stopPropagation();
    });
    draw();
    const inrow = el('div', { class: 'inrow' }, crumbs, input);
    box.append(inrow, list, foot);
    box.focus = () => input.focus();
    return box;
  }

  // message row: proposal for role/priority surfacing (see page note)
  // delivery evidence (messaging.md): six states, one ladder; unknown is its own hollow mark, never red, never a failure
  const EVIDENCE = {
    'sender-stored': { ticks: '·', title: 'in your outbox · not at the recipient yet' },
    'recipient-stored': { ticks: '✓', title: "in the recipient's inbox" },
    'native-queued': { ticks: '✓✓', dim: 1, title: "queued in the recipient's harness" },
    'native-written': { ticks: '✓✓', title: "written into the recipient's harness" },
    'application-acknowledged': { ticks: '✓✓✓', title: 'acknowledged by the harness' },
    'native-unknown': { ticks: '✓◌', title: 'in the inbox · write outcome unknown · never resent on its own' },
  };
  function tickLadder(state) {
    const e = EVIDENCE[state]; if (!e) return null;
    const marks = [...e.ticks].map((t, i) => el('i', { class: (t === '◌' ? 'hollow' : '') + (e.dim && i === 1 ? ' q' : '') }, t));
    return el('span', { class: 'ticks ' + state, title: state + ' · ' + e.title }, ...marks);
  }
  function messageRow(m, opts) {
    opts = opts || {};
    if (m.presence) return el('div', { class: 'msg presence' }, el('span', { class: 'ptext' }, m.text), el('span', { class: 'time' }, m.time || ''));
    // audit line (a projection of the Oracle audit log, swarm-member scope): the Oracle's effects in the open, centred like presence
    if (m.audit) { const a = m.audit; return el('div', { class: 'msg presence audit ' + (a.outcome || 'applied'), title: 'audit · op ' + (a.op || '') + ' · ' + (a.outcome || 'applied') + (a.reason ? ' · ' + a.reason : '') },
      el('div', { class: 'aline' }, el('span', { class: 'amark' }, ROLE_MARK.oracle + ' '), el('b', {}, a.actor || 'Oracle'), ' ' + a.verb + ' ', el('b', {}, a.target || '')),
      el('div', { class: 'ameta' }, a.op ? el('code', { class: 'aop' }, a.op.slice(0, 6)) : null, a.outcome && a.outcome !== 'applied' ? el('span', { class: 'aout' }, a.outcome) : null, el('span', { class: 'time' }, m.time || ''))); }
    const role = m.role || 'unsigned';
    const row = el('div', { class: 'msg ' + role + (m.unread ? ' unread' : '') + (m.replyTo ? ' nested' : ''), style: '--role:var(--role-' + role + ')' });
    const head = el('div', { class: 'head' }, el('span', { class: 'who' }, m.from), el('span', { class: 'role' }, '<' + role.toUpperCase() + '>'));
    if (role === 'oracle' && !m.foreign) head.append(el('span', { class: 'p0' }, 'p0'));  // a foreign Oracle ranks as a worker here
    // subagent addressing (messaging.md): to main or to a subagent; from a subagent, or relayed via main; held when the agent ended
    if (m.fromAgent) head.append(el('span', { class: 'agchip from', title: 'from subagent ' + m.fromAgent }, '↳ ' + m.fromAgent));
    if (m.via === 'main') head.append(el('span', { class: 'agchip via', title: 'the harness relayed it; the subagent is not addressable directly' }, 'via main'));
    if (m.to) head.append(el('span', { class: 'agchip to' + (m.to === 'main' ? ' main' : ''), title: m.to === 'main' ? 'to the main agent' : 'to subagent ' + m.to }, '→ ' + (m.to === 'main' ? 'main' : '↳ ' + m.to)));
    head.append(el('span', { class: 'time' }, m.time || ''));
    if (m.held === 'agent_ended') head.append(el('span', { class: 'agchip held', title: 'held: the subagent ended before delivery; never redirected to main on its own' }, 'held · agent ended'));
    else if (m.evidence) head.append(tickLadder(m.evidence));
    const body = el('div', { class: 'body' }, head);
    if (m.replyTo) body.append(el('div', { class: 'reply' }, '↩ ' + m.replyTo));
    body.append(el('div', { class: 'text' }, /\[\[/.test(m.text || '') ? renderLinks(m.text) : m.text));
    row.append(el('div', { class: 'rail' }), body);
    if (opts.ripple) row.append(el('div', { class: 'ripple' }));
    return row;
  }

  // ---------- settings: data-driven page, left nav + cards, read-mostly (config.toml), diff-from-default marks ----------
  const LT = h => (LAUNCH.find(l => l.harness === h) || {}).tile || '#374151';
  const SET_DEF = {
    account: {
      name: 'Boxy', handle: 'bboxy450', email: 'bboxy450@gmail.com',
      fingerprint: '9B07E849480C68328CA20684844ECC0587DE7AB0', algo: 'ed25519', keyState: 'unlocked · gpg-agent cache',
      trust: ['GPG primary', 'certified device key', 'certified seat key'],
      devices: [['this Mac', 'root custodian', '2026-10-12'], ['rig', 'enrolled', '2026-10-19']],
      sign: { commits: true, messages: true, seatCommits: false },
    },
    cloud: [['Hetzner', 'eu-2 · Falkenstein', true, '#D50C2D'], ['Fly', 'us-1 · Ashburn', true, '#7B3FE4'], ['Lambda', 'gpu-a · A100', false, '#2563EB']],
    machines: [['rig', 'feral@rig.local', 'M2 Ultra · 64 GB', '~/.ssh/id_ed25519', true], ['alpha', 'feral@alpha.tail', 'Linux x86 · 128 GB', '~/.ssh/id_ed25519', true], ['studio', 'feral@studio.local', 'M1 Max · 32 GB', '~/.ssh/id_ed25519', false]],
    // roles: built-ins are editable, not deletable; custom roles carry custom:true. cls = priority class (delivery order and badging)
    roles: {
      oracle: { prompt: 'You are the Oracle. Supervise, verify, never implement. Sign every message Name <ORACLE>.', send: 'all channels · all DMs', color: 'var(--role-oracle)', mark: '♔', cls: 'oracle' },
      executor: { prompt: 'You are an Executor. Do the assigned work, report evidence, never expand scope.', send: 'own swarm · operator DM', color: 'var(--role-executor)', mark: '♟', cls: 'worker' },
      creativity: { prompt: 'You are Creativity. Diverge before converging; return a field, not a pick.', send: 'own swarm · operator DM', color: 'var(--role-creativity)', mark: '♝', cls: 'worker' },
    },
    // harnesses: built-ins carry command/tile/icon; custom ones run through the PTY adapter
    harness: {
      claude: { role: 'oracle', wake: 'socket', hooks: true, command: 'claude', tile: LT('claude'), icon: 'claude', stateFrom: 'osc7501' },
      codex: { role: 'executor', wake: 'hook', hooks: true, command: 'codex', tile: LT('codex'), icon: 'codex', stateFrom: 'hooks' },
      pi: { role: 'executor', wake: 'pty', hooks: false, command: 'pi', tile: LT('pi'), icon: 'pi', stateFrom: 'output cadence' },
      omp: { role: 'executor', wake: 'pty', hooks: false, command: 'omp', tile: LT('omp'), icon: 'omp', stateFrom: 'output cadence' },
    },
    appearance: { theme: 'auto', density: 'cozy', gutter: 2, glow: true, subtitle: false },
    shortcuts: [['New tab', '⌘T'], ['Split right', '⌘D'], ['Split down', '⌘⇧D'], ['Mute seat', '⌘M'], ['Undo close', '⌘Z'], ['Settings', '⌘,'], ['Focus next pane', 'Tab'], ['Focus previous pane', '⇧Tab']],
  };
  const SET = JSON.parse(JSON.stringify(SET_DEF));
  SET.roles.creativity.prompt += ' Name the Censor when it fires.'; SET.harness.pi.role = 'creativity';
  // settingsPage({ section, onChange }) — onChange(section, key, value): 'appearance' per value; 'roles' / 'harness' with the record, null on delete
  function settingsPage(opts) {
    opts = opts || {};
    const emit = (s, k, v) => { if (opts.onChange) opts.onChange(s, k, v); };
    const sections = ['Account', 'Cloud', 'Machines', 'Roles', 'Harnesses', 'Appearance', 'Shortcuts'];
    const NAV_ICON = { Account: 'key', Cloud: 'cloud', Machines: 'ssh', Roles: 'roles', Harnesses: 'gear', Appearance: 'sun', Shortcuts: 'keys' };
    const SENDS = ['all channels · all DMs', 'own swarm · operator DM', 'operator DM only'], WAKES = ['socket', 'hook', 'mcp', 'pty'], STATE_FROM = ['osc7501', 'hooks', 'output cadence'];
    const MARKS = ['♔', '♟', '♝', '♜', '♞', '♚'], ICONS = ['claude', 'codex', 'pi', 'omp', 'zsh'];
    const HUES = [0, 30, 55, 95, 160, 190, 215, 250, 285, 325], hue = i => 'hsl(' + HUES[((i % HUES.length) + HUES.length) % HUES.length] + ' 85% 64%)';
    // marks the settings page alone needs; everything else comes from ICON_SVG
    const SVG = {
      roles: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M5 19h14M5.5 16 3.5 7l5 3.5L12 5l3.5 5.5 5-3.5-2 9z"/></svg>',
      keys: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 10h.01M11 10h.01M15 10h.01M7.5 14h9"/></svg>',
      search: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><circle cx="10.5" cy="10.5" r="6"/><path d="M15.5 15.5 20 20"/></svg>',
      copy: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V6a1 1 0 0 1 1-1h9"/></svg>',
      link: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1.5 1.5M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1.5-1.5"/></svg>',
      open: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 4h6v6M20 4l-9 9M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5"/></svg>',
    };
    const icon = name => ICON_SVG[name] ? harnessIcon(name) : el('span', { class: 'hicon', html: SVG[name] || '' });
    let cur = opts.section || 'Account', draftRole = null, draftHarness = opts.draftHarness || null;
    const root = el('div', { class: 'settings' });
    const nav = el('div', { class: 's-nav' });
    const q = el('input', { class: 's-q', placeholder: 'Search settings', type: 'search' });
    const main = el('div', { class: 's-main' });
    // the one rule: a value that differs from config.toml gets the accent dot and a reset
    const diff = (changed, onReset) => changed ? el('span', { class: 'diff' }, el('i', { title: 'differs from config.toml default' }), el('button', { onclick: onReset }, 'reset')) : null;
    const row = (label, hint, control, changed, onReset) => el('div', { class: 's-row' + (changed ? ' changed' : '') }, el('div', { class: 'lbl' }, label, hint ? el('small', {}, hint) : null), el('div', { class: 'ctl' }, control), diff(changed, onReset));
    const select = (optsArr, val, on) => { const s = el('select', { onchange: e => on && on(e.target.value) }); for (const o of optsArr) s.append(el('option', { value: o, ...(o === val ? { selected: '' } : {}) }, o)); return s; };
    const toggle = (on, cb) => el('span', { class: 'toggle' + (on ? ' on' : ''), role: 'switch', 'aria-checked': String(on), onclick: () => cb && cb(!on) });
    const card = (title, kids, extra, cls) => el('div', { class: 's-card' + (cls ? ' ' + cls : '') }, title ? el('div', { class: 's-card-h' }, el('span', { class: 't' }, title), extra || null) : null, ...kids);
    const btn = (label, iconName, kind) => el('button', { class: 's-btn' + (kind ? ' ' + kind : '') }, iconName ? icon(iconName) : null, label);
    const dot = (on, title) => el('i', { class: 'sdot' + (on ? ' on' : ''), title: title || (on ? 'connected' : 'not connected') });
    const group = (label, hint) => el('div', { class: 's-group' }, label, hint ? el('small', {}, hint) : null);
    const note = text => el('div', { class: 's-note' }, text);
    const chip = (text, cls) => el('span', { class: 'chip ' + (cls || '') }, text);
    const field = (label, control, hint, extra) => el('div', { class: 'fl' }, el('label', {}, el('span', {}, label, hint ? el('small', {}, hint) : null), extra || null), control);
    const text = (val, ph, on, mono) => el('input', { type: 'text', class: mono ? 'mono' : '', value: val || '', placeholder: ph || '', spellcheck: 'false', oninput: e => on(e.target.value) });
    const swatch = (color, on) => el('button', { class: 'swatch', style: 'background:' + color, title: 'click cycles the colour', onclick: on });
    const seg = (optsArr, val, on) => el('div', { class: 'segc' }, ...optsArr.map(o => el('button', { class: o === val ? 'on' : '', onclick: () => on(o) }, o)));
    const roleNames = () => Object.keys(SET.roles);
    const draw = () => {
      nav.innerHTML = '';
      for (const s of sections) nav.append(el('div', { class: 's-navi' + (s === cur ? ' on' : ''), onclick: () => { cur = s; draw(); } }, icon(NAV_ICON[s]), el('span', {}, s)));
      main.innerHTML = '';
      const needle = q.value.toLowerCase();
      const add = (title, node) => { if (!needle || node.textContent.toLowerCase().includes(needle)) main.append(node); };

      if (cur === 'Account' || needle) {
        const a = SET.account, d = SET_DEF.account;
        const trust = el('div', { class: 'trust' }, el('span', { class: 'k' }, 'trust path'));
        a.trust.forEach((t, i) => { if (i) trust.append(el('span', { class: 'arrow' }, '→')); trust.append(chip(t)); });
        add('Account', card(null, [
          el('div', { class: 'hero' },
            logo(72, { square: true }),
            el('div', { class: 'hero-b' },
              el('div', { class: 'hname' }, el('span', { class: 'n' }, a.name), el('span', { class: 'pill-ok' }, el('i'), 'signed in')),
              el('div', { class: 'fp', title: 'primary key fingerprint' }, ...a.fingerprint.match(/.{4}/g).map(g => el('span', {}, g))),
              el('div', { class: 'chips' },
                el('span', { class: 'chip mono' }, icon('key'), a.algo + ' · signing'),
                el('span', { class: 'chip' }, dot(true, 'unlocked by pinentry'), a.keyState),
                el('button', { class: 'chip cact', title: 'copy the full 40-hex fingerprint' }, icon('copy'), 'copy fingerprint')),
              el('div', { class: 'uid' }, a.name + ' (' + a.handle + ') <' + a.email + '>'),
              trust,
              el('div', { class: 'held' }, el('span', {}, 'primary held by gpg-agent · pinned from your keyring · never exported'), el('button', { class: 'chip cact' }, icon('open'), 'open in gpg')),
              el('div', { class: 'acts' },
                el('div', { class: 'act' }, btn('Show paper recovery', 'key', 'primary'), el('small', {}, 'restores identity only · needs your GPG backup or paperkey plus the public certificate · seats, history and grants are not recovered')),
                el('div', { class: 'act' }, btn('Link Google for cloud', 'link', 'link'), el('small', {}, 'binds a provider for cloud seats · the key stays the identity'))))),
          group('Devices · certified leaf keys', 'revoke is local · peers learn on next contact'),
          el('div', { class: 'subkeys' },
            ...a.devices.map(([n, s, when]) => el('div', { class: 'sk' },
              el('div', { class: 'skh' }, el('b', {}, n), dot(true, s)),
              el('div', { class: 'skm' }, 'device cert · ed25519'),
              el('div', { class: 'sks' }, s + ' · ' + when),
              el('button', { class: 'revoke', title: 'revoked here · peers learn on next contact' }, 'revoke'))),
            el('button', { class: 'sk add' }, el('b', {}, '+ enroll a device'), el('span', {}, 'request if primary is elsewhere'), el('small', {}, 'primary not available here → produces a signed request'))),
        ], null, 'account'));
        add('Account', note('Devices and seats are leaf keys certified by your GPG primary; enrollment is a one-use ticket signed where the primary lives. Expired, revoked or forked authority blocks controls on its own, apart from seat state. Google is a later link for cloud; the primary stays the identity.'));
        add('Account', card('Signatures', [
          row('Sign commits as this key', 'git author and committer · Boxy <' + a.email + '>', toggle(a.sign.commits, x => { a.sign.commits = x; draw(); }), a.sign.commits !== d.sign.commits, () => { a.sign.commits = d.sign.commits; draw(); }),
          row('Sign messages and presence with the seat\'s leaf key', 'chain verifies to this key', toggle(a.sign.messages, x => { a.sign.messages = x; draw(); }), a.sign.messages !== d.sign.messages, () => { a.sign.messages = d.sign.messages; draw(); }),
          row('Seats sign commits with their own keys', 'assumed, not decided', toggle(a.sign.seatCommits, x => { a.sign.seatCommits = x; draw(); }), a.sign.seatCommits !== d.sign.seatCommits, () => { a.sign.seatCommits = d.sign.seatCommits; draw(); }),
        ]));
      }

      if (cur === 'Cloud' || needle) add('Cloud', card('Cloud providers', [
        el('div', { class: 'tiles' },
          ...SET.cloud.map(([p, r, on, col], i) => { const ch = on !== SET_DEF.cloud[i][2]; return el('div', { class: 'ptile' + (ch ? ' changed' : '') },
            el('div', { class: 'ph' }, el('span', { class: 'tile', style: 'background:' + col }, p[0]), el('div', { class: 'pt' }, el('b', {}, p), el('small', {}, r)), dot(on)),
            el('div', { class: 'pf' }, el('span', {}, on ? 'connected' : 'not connected'), diff(ch, () => { SET.cloud[i][2] = SET_DEF.cloud[i][2]; draw(); }), toggle(on, v => { SET.cloud[i][2] = v; draw(); }))); }),
          el('button', { class: 'ptile add' }, '+ connect a provider')),
        note('Regions appear under Cloud in the launcher. Credentials live in the keychain, never in config.toml.'),
      ]));

      if (cur === 'Machines' || needle) add('Machines', card('SSH machines', [
        ...SET.machines.map(([h, u, hw, k, up]) => el('div', { class: 'mrow' }, dot(up, up ? 'reachable' : 'unreachable'), el('div', { class: 'mt' }, el('b', {}, h), el('code', {}, u)), el('span', { class: 'hw' }, hw), el('span', { class: 'kbd' }, k))),
        el('button', { class: 'mrow add' }, '+ add a machine'),
        note('Read from ~/.ssh/config. Machines appear under SSH in the launcher.'),
      ]));

      if (cur === 'Roles' || needle) {
        const roleCard = r => {
          const v = SET.roles[r], d = SET_DEF.roles[r] || null, chP = !!d && v.prompt !== d.prompt, chS = !!d && v.send !== d.send, chC = !!d && v.cls !== d.cls;
          const head = el('div', { class: 'band' }, el('span', { class: 'rmark' }, v.mark), el('span', { class: 'rn' }, r), v.custom ? el('button', { class: 'del', onclick: () => { delete SET.roles[r]; emit('roles', r, null); draw(); } }, 'delete') : chip('built-in', 'bi'));
          return el('div', { class: 's-card role', style: '--role:' + v.color }, head, el('div', { class: 'rb' },
            el('div', { class: 'fl' + (chP ? ' changed' : '') }, el('label', {}, el('span', {}, 'Default system prompt'), diff(chP, () => { v.prompt = d.prompt; emit('roles', r, v); draw(); })), el('textarea', { html: v.prompt, spellcheck: 'false', oninput: e => { v.prompt = e.target.value; emit('roles', r, v); } })),
            el('div', { class: 'fl' + (chS ? ' changed' : '') }, el('label', {}, el('span', {}, 'May send to', el('small', {}, 'terminal-enforced')), diff(chS, () => { v.send = d.send; emit('roles', r, v); draw(); })), select(SENDS, v.send, x => { v.send = x; emit('roles', r, v); draw(); })),
            el('div', { class: 'fl' + (chC ? ' changed' : '') }, el('label', {}, el('span', {}, 'Priority class', el('small', {}, 'delivery order and badging follow the class')), diff(chC, () => { v.cls = d.cls; emit('roles', r, v); draw(); })), seg(['oracle class', 'worker class'], v.cls + ' class', x => { v.cls = x.split(' ')[0]; emit('roles', r, v); draw(); }), el('small', { class: 'cap' }, 'changing class re-issues this role\'s seat grants'))));
        };
        const newRole = () => {
          if (!draftRole) return el('button', { class: 's-card role add', onclick: () => { draftRole = { name: '', ci: 4, mark: '♜', prompt: '', send: SENDS[1], cls: 'worker' }; draw(); } }, el('b', {}, '+ new role'), el('span', {}, 'your own mark, colour and prompt'));
          const dr = draftRole, ok = () => dr.name.trim() && !SET.roles[dr.name.trim()];
          return el('div', { class: 's-card role editing', style: '--role:' + hue(dr.ci) },
            el('div', { class: 'band' }, swatch(hue(dr.ci), () => { dr.ci++; draw(); }), el('input', { type: 'text', class: 'rn', placeholder: 'role name', value: dr.name, spellcheck: 'false', oninput: e => { dr.name = e.target.value; } }), chip('new', 'bi')),
            el('div', { class: 'rb' },
              field('Mark', el('div', { class: 'marks' }, ...MARKS.map(m => el('button', { class: m === dr.mark ? 'on' : '', onclick: () => { dr.mark = m; draw(); } }, m)))),
              field('Default system prompt', el('textarea', { placeholder: 'You are …', spellcheck: 'false', oninput: e => { dr.prompt = e.target.value; } }, dr.prompt)),
              field('May send to', select(SENDS, dr.send, x => { dr.send = x; }), 'terminal-enforced'),
              field('Priority class', seg(['oracle class', 'worker class'], dr.cls + ' class', x => { dr.cls = x.split(' ')[0]; draw(); }), 'delivery order and badging follow the class'),
              el('div', { class: 'facts' }, btn('Add role', null, 'primary'), btn('Cancel', null, 'ghost'))));
        };
        const grid = el('div', { class: 'roles' }, ...roleNames().map(roleCard), newRole());
        const acts = grid.querySelector('.facts'); if (acts) { const dr = draftRole; acts.children[0].addEventListener('click', () => { const name = dr.name.trim(); if (!name || SET.roles[name]) { grid.querySelector('input.rn').focus(); return; } const rec = { prompt: dr.prompt, send: dr.send, color: hue(dr.ci), mark: dr.mark, cls: dr.cls, custom: true }; SET.roles[name] = rec; draftRole = null; emit('roles', name, rec); draw(); }); acts.children[1].addEventListener('click', () => { draftRole = null; draw(); }); }
        add('Roles', grid);
      }

      if (cur === 'Harnesses' || needle) {
        // generic PTY adapter (harness-adapters R7): no hooks → operator handoff, never an inferred Enter; hooks → typed only after an idle hook
        const submitHint = (wake, hooks) => wake !== 'pty' ? null : hooks ? 'types after an idle hook' : 'never submits on its own';
        const hTile = h => {
          const v = SET.harness[h], d = SET_DEF.harness[h] || null, ch = k => !!d && v[k] !== d[k], rs = k => () => { v[k] = d[k]; emit('harness', h, v); draw(); }, set = k => x => { v[k] = x; emit('harness', h, v); draw(); };
          return el('div', { class: 's-card htile' },
            el('div', { class: 'hh' }, el('span', { class: 'tile', style: 'background:' + v.tile }, harnessIcon(v.icon || h)), el('div', { class: 'ht' }, el('b', {}, (LAUNCH.find(l => l.harness === h) || {}).label || h), el('code', {}, v.command || h)),
              v.custom ? el('button', { class: 'del', onclick: () => { delete SET.harness[h]; emit('harness', h, null); draw(); } }, 'delete') : chip('built-in', 'bi')),
            row('Default role', null, select(roleNames(), v.role, set('role')), ch('role'), rs('role')),
            row('Wake', v.custom ? submitHint(v.wake, v.hooks) : null, select(WAKES, v.wake, set('wake')), ch('wake'), rs('wake')),
            row('State from', null, select(STATE_FROM, v.stateFrom, set('stateFrom')), ch('stateFrom'), rs('stateFrom')),
            row('Hooks', 'state from harness hooks', toggle(v.hooks, set('hooks')), ch('hooks'), rs('hooks')));
        };
        const newHarness = () => {
          if (!draftHarness) return el('button', { class: 's-card htile add', onclick: () => { draftHarness = { name: '', command: '', ci: 7, icon: 'zsh', role: 'executor', wake: 'pty', stateFrom: 'output cadence', hooks: false }; draw(); } }, el('b', {}, '+ new harness'), el('span', {}, 'any command that talks in a terminal'));
          const dh = draftHarness;
          return el('div', { class: 's-card htile editing' },
            el('div', { class: 'hh' }, swatch(hue(dh.ci), () => { dh.ci++; draw(); }), el('div', { class: 'ht' }, text(dh.name, 'harness name', x => { dh.name = x; }), text(dh.command, 'aider --model …', x => { dh.command = x; }, true)), chip('new', 'bi')),
            field('Icon', el('div', { class: 'marks icons' }, ...ICONS.map(i => el('button', { class: i === dh.icon ? 'on' : '', style: i === dh.icon ? 'background:' + hue(dh.ci) : '', onclick: () => { dh.icon = i; draw(); } }, harnessIcon(i))))),
            row('Default role', null, select(roleNames(), dh.role, x => { dh.role = x; })),
            row('Wake', submitHint(dh.wake, dh.hooks), select(WAKES, dh.wake, x => { dh.wake = x; draw(); })),
            row('State from', null, select(STATE_FROM, dh.stateFrom, x => { dh.stateFrom = x; })),
            row('Hooks', null, toggle(dh.hooks, x => { dh.hooks = x; draw(); })),
            el('div', { class: 's-note' }, 'unknown harnesses run through the PTY adapter: state is inferred from output until hooks or OSC 7501 upgrade it'),
            el('div', { class: 'facts' }, btn('Add harness', null, 'primary'), btn('Cancel', null, 'ghost')));
        };
        const grid = el('div', { class: 'htiles' }, ...Object.keys(SET.harness).map(hTile), newHarness());
        const acts = grid.querySelector('.facts'); if (acts) { const dh = draftHarness; acts.children[0].addEventListener('click', () => { const name = dh.name.trim(); if (!name || SET.harness[name]) { grid.querySelector('.editing input').focus(); return; } const rec = { role: dh.role, wake: dh.wake, hooks: dh.hooks, command: dh.command, tile: hue(dh.ci), icon: dh.icon, custom: true, stateFrom: dh.stateFrom }; SET.harness[name] = rec; draftHarness = null; emit('harness', name, rec); draw(); }); acts.children[1].addEventListener('click', () => { draftHarness = null; draw(); }); }
        add('Harnesses', grid);
      }

      if (cur === 'Appearance' || needle) {
        const v = SET.appearance, d = SET_DEF.appearance, C = T.color, setA = k => x => { v[k] = x; emit('appearance', k, x); draw(); }, rsA = k => () => { v[k] = d[k]; emit('appearance', k, d[k]); draw(); };
        // theme swatches are little windows drawn in the real theme colours (tokens, not guesses)
        const win = t => { const c = C[t === 'light' ? 'light' : 'dark'], l = C.light; return el('span', { class: 'win', style: 'background:' + c.bg },
          el('i', { class: 'tb', style: 'background:' + c.surface }), el('i', { class: 'sb', style: 'background:' + c.sidebar }), el('i', { class: 'pn', style: 'background:' + c.surface_2 }),
          t === 'auto' ? el('i', { class: 'half', style: 'background:' + l.bg }, el('i', { class: 'tb', style: 'background:' + l.surface }), el('i', { class: 'pn', style: 'background:' + l.surface_2 })) : null); };
        const themes = el('div', { class: 'thumbs' }, ...['auto', 'dark', 'light'].map(t => el('button', { class: 'thumb' + (v.theme === t ? ' on' : ''), onclick: () => setA('theme')(t) }, win(t), el('span', { class: 'cap' }, t))));
        // density previews at the real row heights from tokens, so the choice is visible before it applies
        const dens = el('div', { class: 'thumbs' }, ...['touch', 'comfortable', 'cozy', 'compact'].map(k => { const dd = T.density[k], n = Math.floor(100 / dd.row_h);
          return el('button', { class: 'thumb' + (v.density === k ? ' on' : ''), onclick: () => setA('density')(k) },
            el('span', { class: 'win rows' }, ...Array.from({ length: n }, (_, i) => el('i', { class: 'r' + (i === 1 ? ' on' : ''), style: 'height:' + dd.row_h + 'px;padding:0 ' + Math.round(dd.pad_x / 2) + 'px;gap:' + dd.gap + 'px' }, el('i', { class: 't', style: 'width:' + dd.icon + 'px;height:' + dd.icon + 'px' }), el('i', { class: 'b' }), el('i', { class: 'g', style: 'width:' + dd.glyph / 2 + 'px;height:' + dd.glyph / 2 + 'px' })))),
            el('span', { class: 'cap' }, k, el('small', {}, dd.row_h + ' px'))); }));
        add('Appearance', card('Appearance', [
          row('Theme', 'starts from the OS', themes, v.theme !== d.theme, rsA('theme')),
          row('Density', 'row height on the map', dens, v.density !== d.density, rsA('density')),
          row('Gutter', 'between panes', seg(['2 px', '4 px'], v.gutter + ' px', x => setA('gutter')(parseInt(x, 10))), v.gutter !== d.gutter, rsA('gutter')),
          row('Border glow', 'particles in the seat colour', toggle(v.glow, setA('glow')), v.glow !== d.glow, rsA('glow')),
          row('Last-output subtitle', 'second line on seat rows', toggle(v.subtitle, setA('subtitle')), v.subtitle !== d.subtitle, rsA('subtitle')),
        ]));
      }

      if (cur === 'Shortcuts' || needle) add('Shortcuts', card('Shortcuts', [el('div', { class: 'keys' }, ...SET.shortcuts.map(([l, k]) => el('div', { class: 'kr' }, el('span', {}, l), el('span', { class: 'kbd' }, k))))]));
      if (!main.children.length) main.append(note('nothing matches "' + q.value + '"'));
    };
    q.addEventListener('input', draw); draw();
    root.append(el('div', { class: 's-side' }, el('div', { class: 's-qwrap' }, icon('search'), q), nav, el('div', { class: 's-src' }, '~/.config/bboxy/config.toml')), main);
    return root;
  }

  window.C = { docModal, diagram, parseMermaid, titleBar, launcherItems, eventRow, consumerCard, subjectChip, EV_ROOT, evTier, docView, DOC_HUE, mdLite, purgeModal, rowMenu, SEND_SCOPES, pendingRow, gapMarker, subagentRows, EVIDENCE, tickLadder, subscriptionsCard, externalLane, renderLinks, provenanceCard, artifactRow, ART_GLYPH, graphPage, epicGroup, taskRow, TASK_STATES, TASK_GLYPH, GRADE_TITLE, KIND_MARK, settingsPage, SET, SET_DEF, LAUNCH, hslToRgb, logo, syncPulse, hash32, rng, seatColor, rollName, HARNESSES, DEFAULT_ROLE, STATES, GLYPH, ROLE_MARK, ICON_SVG, glyph, harnessIcon, roleMark, kbd, makeSeat, makeTree, leaves, makeModel, rollup, seatRow, miniGrid, tabRow, spaceHeader, sidebar, titleStrip, pane, picker, messageRow };
})();
