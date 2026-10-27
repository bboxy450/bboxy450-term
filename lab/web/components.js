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
  function rollName(r) { return pick(r, FIRST) + ' ' + pick(r, LAST); }   // demo fixtures only; a new seat takes poolName
  // the product's seat-name pool (crates/bbt-proto/src/seat_names.json, generated into window.SEAT_NAMES; T1): a new seat takes the
  // first pool name not used in its swarm, then '<name> 2', '<name> 3', ... as the backend allocates; k is the k-th such name (reroll)
  function poolName(used, k) {
    const P = window.SEAT_NAMES || [], u = new Set(used || []), want = k || 0;
    if (!P.length) throw new Error('the seat-name pool is empty');
    // at most u.size names are taken, so ceil((u.size + want + 1) / |P|) tiers, plus one, always hold the want-th free name
    const tiers = Math.ceil((u.size + want + 1) / P.length) + 1; let i = 0;
    for (let s = 1; s <= tiers; s++) for (const n of P) { const c = s === 1 ? n : n + ' ' + s; if (!u.has(c) && i++ === want) return c; }
    throw new Error('unreachable: no free pool name within ' + tiers + ' tiers');
  }

  const HARNESSES = ['claude', 'codex', 'pi', 'omp', 'zsh'];
  // What the app launches (decision #262, AGENTS.md): Claude supervises, so oracle; Codex implements, so executor and never an oracle
  // class; a shell has no harness to prompt, so shell (worker class, no chip). A launcher preset or the row's role chip overrides.
  const DEFAULT_ROLE = { claude: 'oracle', codex: 'executor', pi: 'executor', omp: 'executor', zsh: 'shell' };
  // Demo cohorts only: a swarm with its oracle, so the lab shows every role mark.
  const DEMO_ROLE = { claude: 'oracle', codex: 'executor', pi: 'executor', omp: 'executor', zsh: 'shell' };   // decision #262: a shell's role is shell
  const STATES = ['active', 'idle', 'waiting', 'errored'];
  const GLYPH = { active: '●', idle: '○', waiting: '◐', errored: '✕' };
  const ROLE_MARK = { oracle: '♔', architect: '♖', executor: '♟', creativity: '♝', unsigned: '·' };
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
    const g = el('span', { class: 'glyph ' + state + ' g-' + grade + (kind ? ' k-' + kind : ''), style: 'color:' + color, title: state + (kind ? ' · ' + kind : '') + ' · ' + GRADE_TITLE[grade] }, (SEAT_MARK[state] || SEAT_MARK.idle)());
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
    bug: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="7" y="7" width="10" height="13" rx="5"/><path d="M9.5 7.5 8 4.5M14.5 7.5 16 4.5M7 11H3.5M17 11h3.5M7 15.5H4M17 15.5h3M12 7v13"/></svg>',
    keyx: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="6.5" cy="12" r="3.5"/><path d="M10 12h11M17 12v3M20 12v2.5M3 20L21 4"/></svg>',
    plug: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M9 3v5M15 3v5M6 8h12v3a6 6 0 0 1-12 0zM12 17v4"/></svg>',
    sidebarfox: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="3" y="8" width="18" height="12" rx="2.5"/><path d="M9.5 8v12"/><path d="M3.2 8.5 3.8 2.2 8.6 7.6Z" fill="currentColor" stroke="none"/></svg>',
    sidebarfoxopen: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="3" y="8" width="18" height="12" rx="2.5"/><path d="M3 8h6.5v12H3z" fill="currentColor" stroke="none"/><path d="M3.2 8.5 3.8 2.2 8.6 7.6Z" fill="currentColor" stroke="none"/></svg>',
    archive: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M3 7h18v3H3zM5 10v9h14v-9M10 14h4"/></svg>',
    chevdown: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"><path d="M6.5 9.5 12 15l5.5-5.5"/></svg>',
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

  // select: the native control keeps keyboard and the OS menu, but draws none of its own chrome; the caret is our chevdown icon
  function selectBox(optsArr, val, on) { const s = el('select', { onchange: e => on && on(e.target.value) }); for (const o of optsArr) s.append(el('option', { value: o, ...(o === val ? { selected: '' } : {}) }, o)); return el('span', { class: 'sel' }, s, harnessIcon('chevdown', 'caret')); }
  function harnessIcon(name, extra) { return el('span', { class: 'hicon ' + (extra || ''), title: name, html: ICON_SVG[name] || ICON_SVG.zsh }); }
  function roleMark(role) { return el('span', { class: 'rmark', style: 'color:var(--role-' + (role || 'unsigned') + ')', title: role || 'unsigned' }, ROLE_MARK[role || 'unsigned']); }
  function kbd(s) { return el('span', { class: 'kbd' }, s); }

  // ---------- model ----------
  function makeSeat(r, i, opts) {
    const harness = opts && opts.harness || pick(r, ['claude', 'codex', 'codex', 'codex', 'pi', 'omp', 'zsh']);
    const key = 'k' + hash32('seat' + i + (opts && opts.seed || '')).toString(16);
    return { key, name: harness === 'zsh' ? 'zsh' : rollName(r), harness, role: DEMO_ROLE[harness], title: pick(r, TITLES), state: opts && opts.state || pick(r, ['active', 'active', 'idle', 'idle', 'idle', 'waiting', 'errored']) };
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
    const row = el('div', { class: 'seat-row ' + seat.state + (seat.life ? ' life-' + seat.life : '') + (opts.focused ? ' focused' : ''), style: '--seat:' + seatColor(seat) });
    const tileCol = (LAUNCH.find(l => l.harness === seat.harness) || {}).tile || 'var(--surface-2)';
    if (seat.where && seat.where !== 'local') row.append(el('span', { class: 'where ' + seat.where + (seat.link ? ' ' + linkOf(seat.link).kind : ''), title: (seat.where === 'cloud' ? 'runs in the cloud' : 'over ssh') + (seat.host ? ' · ' + seat.host : '') + (seat.link ? ' · ' + linkSay(seat.link) : '') }, harnessIcon(seat.where)));
    row.append(el('span', { class: 'tile', style: 'background:' + tileCol }, harnessIcon(seat.harness)), el('span', { class: 'name', style: 'color:' + seatColor(seat) }, seat.name));
    if (seat.role && seat.role !== 'shell') row.append(roleMark(seat.role));   // a shell carries no role chip (decision #262)
    if (seat.authority) row.append(authMark(seat));
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
    // deferred wakes (messaging-attention §3, MA-8): messages admitted to a digest wait for its window; the row says how many and when it
    // delivers. A digest delays a wake and never holds a message (messaging-never-held), so this is a plain count, not the held colour.
    if (seat.deferred) right.append(el('span', { class: 'deferred', title: seat.deferred.n + ' deferred to a digest · delivers at ' + seat.deferred.at + ' or at 50, whichever first · stored, never held' }, seat.deferred.n + ' at ' + seat.deferred.at));
    // noise hint (messaging.md routing discipline 4): the seat's share of stream messages is over the threshold; shown to the Oracle
    if (seat.noise) right.append(el('span', { class: 'noise', title: 'stream share ' + Math.round(seat.noise * 100) + '% over the ' + Math.round((seat.noiseMax || .25) * 100) + '% threshold · route progress by DM' }, 'noisy ' + Math.round(seat.noise * 100) + '%'));
    // seats are sockets: broker telemetry, counts only, never who
    if (seat.listeners > 0) right.append(el('span', { class: 'listeners', title: seat.listeners + ' client' + (seat.listeners === 1 ? '' : 's') + " on this seat's socket · counts only" }, harnessIcon('plug'), String(seat.listeners)));
    const touched = seat.touched || 0;
    if (touched) right.append(el('span', { class: 'touched', title: touched + ' artifact' + (touched === 1 ? '' : 's') + ' touched by this seat' }, harnessIcon('archive'), String(touched)));
    const unread = opts.unread != null ? opts.unread : seat.unread, unreadRole = opts.unreadRole || seat.unreadRole || 'executor';
    // unread is a count only when the retained pages prove it; otherwise the literal 'more', a named gap, never a guessed number
    if (unread) right.append(el('span', { class: 'badge' + (unread === 'more' ? ' more' : ''), style: 'background:var(--role-' + unreadRole + ')', title: unread === 'more' ? 'unread beyond the retained pages · count not proven' : unread + ' unread' }, unread));
    if (opts.shortcut) right.append(kbd(opts.shortcut));
    // lifecycle (lifecycle.html): starting → a spinner stands in for the glyph; exited → the ✕ with an 'exited' chip carrying the reason
    if (seat.life === 'exited') right.append(el('span', { class: 'exited', title: exitSay(seat.exit) }, 'exited'));
    right.append(seat.life === 'starting' ? el('i', { class: 'spin', title: 'starting · ' + seat.harness }) : glyph(seat.state, seat));
    row.append(right);
    if (opts.subtitle) row.append(el('span', { class: 'sub' }, opts.subtitle));
    row.dataset.key = seat.key;
    return syncPulse(row, seat.state);
  }

  // Task row: a delegated task under its seat. State is a glyph + chip; reported shows an accept pill (Oracle/operator).
  // statuses are the work-item frontmatter statuses (work-items.md): the file is the contract
  const TASK_STATES = ['open', 'claimed', 'running', 'reported', 'accepted', 'rejected', 'blocked', 'done'];
  const TASK_GLYPH = { open: '○', claimed: '◔', running: '◑', reported: '◕', accepted: '●', done: '✓', rejected: '✕', blocked: '⊘' };
  // State marks drawn as geometry, centred by construction (operator 2026-10-23: the font circles sat off centre when large, on web
  // and native). A ring of radius 6 in a 16-unit box centred on the origin, a pie wedge from `from` turns past 12 o'clock sweeping
  // `frac` clockwise: ◔ ◑ ◕ are .25/.5/.75, the seat's waiting ◐ is the left half (.5 from .5). Cross, check and slash share the
  // ring's stroke. Each mark fills its box (.glyph, .tglyph), so the box decides the size and nothing moves.
  const svgMark = (inner, cls) => { const m = el('span', { class: 'mark' + (cls ? ' ' + cls : '') }); m.innerHTML = '<svg viewBox="-8 -8 16 16" aria-hidden="true">' + inner + '</svg>'; return m; };
  const RING = '<circle r="6" fill="none" stroke="currentColor" stroke-width="1.5"/>';
  function circleMark(frac, from) {
    const r = 6, a0 = (from || 0) * 2 * Math.PI, a1 = a0 + frac * 2 * Math.PI, P = a => (r * Math.sin(a)).toFixed(3) + ',' + (-r * Math.cos(a)).toFixed(3);
    const fill = frac >= 1 ? '<circle r="6" fill="currentColor"/>' : frac > 0 ? '<path d="M0,0L' + P(a0) + 'A6,6 0 ' + (frac > .5 ? 1 : 0) + ' 1 ' + P(a1) + 'Z" fill="currentColor"/>' : '';
    return svgMark(RING + fill, 'f' + Math.round(frac * 100));
  }
  const crossMark = () => svgMark('<path d="M-4.2,-4.2L4.2,4.2M4.2,-4.2L-4.2,4.2" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>', 'cross');
  // check points centred on the origin: ink y spans ±(3.55 + stroke/2)
  const checkMark = () => svgMark('<path d="M-4.8,0.25L-1.5,3.55L4.8,-3.55" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>', 'check');
  const slashMark = () => svgMark(RING + '<path d="M-4.24,4.24L4.24,-4.24" stroke="currentColor" stroke-width="1.5"/>', 'slash');
  const TASK_MARK = { open: () => circleMark(0), claimed: () => circleMark(.25), running: () => circleMark(.5), reported: () => circleMark(.75), accepted: () => circleMark(1), done: checkMark, rejected: crossMark, blocked: slashMark };
  const taskMark = st => (TASK_MARK[st] || TASK_MARK.open)();
  const SEAT_MARK = { active: () => circleMark(1), idle: () => circleMark(0), waiting: () => circleMark(.5, .5), errored: crossMark };
  // L9 projections (decision #38; bbt-proto messaging.rs MessageRow, TaskCard, TaskReport): the lab's message and task inputs carry
  // the projection's own fields and the work-items status names, so native parses them as they are, with no label mapping.
  // Priority chip: p0 = oracle class, p1 = worker class at priority >= 7 on the 0-9 scale, p2 otherwise; display only, never order.
  const prioChip = m => m.priority_class === 'oracle' ? 'p0' : (m.priority || 0) >= 7 ? 'p1' : 'p2';
  // ts is epoch ms, shown as HH:MM in UTC so a pure view formats it the same on any machine with no time-zone IO. atHM builds a
  // fixture ts at that UTC wall-clock time.
  const atHM = hm => { const [h, mi] = hm.split(':').map(Number); return Date.UTC(2026, 9, 23, h, mi); };
  const hhmm = ts => { if (ts == null) return ''; const d = new Date(ts); return String(d.getUTCHours()).padStart(2, '0') + ':' + String(d.getUTCMinutes()).padStart(2, '0'); };
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
      { label: seat.muted ? 'Unmute' : 'Mute', a: 'mute', k: '⌘M', disabled: seat.authority ? 'needs the seat signature · ' + AUTH_SAY[seat.authority] : null },
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
      const row = el('div', { class: 'rm-item' + (it.danger ? ' danger' : '') + (it.disabled ? ' disabled' : ''), role: 'menuitem', 'aria-disabled': it.disabled ? 'true' : 'false' },
        el('span', { class: 'chk' }), el('span', { class: 'lbl' }, it.label, it.cap || it.disabled ? el('small', {}, it.disabled || it.cap) : null), it.sub ? el('span', { class: 'k' }, '▸') : it.k ? el('span', { class: 'k' }, it.k) : null);
      const i = rows.length; rows.push(row);
      row.addEventListener('pointerenter', () => { sel = i; paint(); it.sub ? openFly(it, row) : closeFly(); });
      row.addEventListener('click', e => { e.stopPropagation(); if (it.disabled) return; if (it.sub) { openFly(it, row); return; }
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
    return el('div', { class: 'tab-row pending ' + st, 'data-rim-key': p.op || p.name, title: (p.op ? 'op ' + p.op + ' · ' : '') + say }, mark, harnessIcon(p.harness), el('span', { class: 'pcol' }, top, el('span', { class: 'pcap' }, say)));
  }
  // invariant chip (work-items.md): outlined "inv · <title>" where an item is constrained, red "breaks inv · <title>" where a bug violates
  const invChip = (title, rel) => el('span', { class: 'invchip ' + rel, title: (rel === 'violates' ? 'violates invariant ' : 'constrained by invariant ') + title }, rel === 'violates' ? 'breaks ' : 'inv ', el('b', {}, title));
  function taskRow(task, seat, opts) {
    opts = opts || {};
    const bug = task.type === 'bug';
    const row = el('div', { class: 'task-row ' + task.status + (bug ? ' bug' : ''), 'data-task': task.id, title: (bug ? 'bug · ' : '') + task.title + ' · ' + task.status + (task.delegated_by ? ' · delegated by ' + task.delegated_by : '') });
    row.append(el('span', { class: 'tglyph' }, bug ? harnessIcon('bug') : taskMark(task.status)), el('span', { class: 'ttitle' }, task.title));
    if (task.violates) row.append(invChip(task.violates, 'violates'));
    if (task.constrains) row.append(invChip(task.constrains, 'constrains'));
    if (task.status === 'blocked' && task.waits) row.append(el('span', { class: 'twaits' }, 'waits on ' + task.waits));
    const right = el('span', { class: 'right' });
    if (task.verify) right.append(el('span', { class: 'tverify' + (task.verify.exit_code === 0 ? ' ok' : ' bad'), title: 'verify exit ' + task.verify.exit_code }, 'exit ' + task.verify.exit_code));
    if (task.status === 'reported' && (opts.review || opts.onAccept)) right.append(el('button', { class: 'accept', onclick: e => { e.stopPropagation(); if (opts.onAccept) opts.onAccept(task, seat); } }, 'accept'));
    right.append(el('span', { class: 'tstate' }, task.status));
    row.append(right);
    return row;
  }

  // Graph page: who is on what (agents) and the work-item tree (epics → specs/tasks with dependencies), generated from data
  // E2E checklist (spec #258 §1, §4, A-3): the promises parsed from §1 joined to their e2e records by function. A promise with no
  // record or two reads untested and names the fault. A fail (blocked counts as one) is tracked when a link { to, rel: 'tracked-by' }
  // names a task whose status (tasks [{ ref, status }], ref '#N' or a slug, matched to the link's [[target]]) is still open.
  // Three summary states and no fourth: green (every promise one usable record, none untested, zero fail), amber (the only non-pass
  // rows are tracked fails, plus manual rows), red (anything missing, doubled, untested, or a fail untracked).
  const E2E_OPEN = ['open', 'claimed', 'running', 'reported', 'blocked'], E2E_ST = ['pass', 'fail', 'blocked', 'untested', 'manual'];
  // #316: a pass is a claim about the whole promise. coverage: full | partial (missing: the promise's assertions the script does not
  // make); a pass with no coverage reads partial, missing 'unstated'. Display statuses add 'partial' (amber, never green).
  const E2E_SHOW = ['pass', 'partial', 'fail', 'blocked', 'untested', 'manual'];
  const e2eRef = to => String(to).replace(/^\[\[|\]\]$/g, '');
  const e2eTasks = (r, tasks) => (r.links || []).filter(l => l.rel === 'tracked-by').map(l => { const ref = e2eRef(l.to), t = (tasks || []).find(x => x.ref === ref); return { ref, status: t ? t.status : 'unknown' }; });
  // fail closed: an empty catalogue is refused (throws), and a record whose status is not one of the five is not usable: the promise
  // reads untested with the fault 'invalid status · <s>'
  // faults: host diagnostics (the provider is offline, a read failed), never fake records; any one makes the summary red, so a stale
  // set of records can never read green
  function e2eSummary(promises, records, tasks, faults) {
    if (!promises || !promises.length) throw new Error('the e2e catalogue is empty: no promises parsed from spec #258 §1');
    const by = new Map(); for (const r of records) { if (!by.has(r.function)) by.set(r.function, []); by.get(r.function).push(r); }
    const rows = promises.map(p => { const rs = by.get(p.id) || [];
      if (rs.length !== 1) return { ...p, status: 'untested', fault: rs.length ? 'doubled · ' + rs.length + ' records' : 'no record' };
      const r = rs[0], failing = r.status === 'fail' || r.status === 'blocked';
      if (!E2E_ST.includes(r.status)) return { ...p, ...r, status: 'untested', fault: 'invalid status · ' + r.status };
      const tracked_by = e2eTasks(r, tasks), open = tracked_by.some(t => E2E_OPEN.includes(t.status));
      if (r.status === 'pass' && r.coverage !== 'full') return { ...p, ...r, status: 'partial', partial: true, missing: r.coverage === 'partial' && (r.missing || []).length ? r.missing : ['unstated'], tracked_by, tracked: open };
      return { ...p, ...r, failing, tracked_by, tracked: failing && open }; });
    const counts = Object.fromEntries(E2E_SHOW.map(k => [k, rows.filter(r => r.status === k).length]));
    const missing = rows.filter(r => r.fault === 'no record').map(r => r.id), doubled = rows.filter(r => r.fault && r.fault.startsWith('doubled')).map(r => r.id), invalid = rows.filter(r => r.fault && r.fault.startsWith('invalid')).map(r => r.id);
    const untracked = rows.filter(r => r.failing && !r.tracked).map(r => r.id), partialUntracked = rows.filter(r => r.partial && !r.tracked).map(r => r.id);
    faults = faults || [];
    const state = faults.length || missing.length || doubled.length || invalid.length || counts.untested || untracked.length || partialUntracked.length ? 'red' : counts.fail + counts.blocked + counts.partial ? 'amber' : 'green';
    return { state, rows, counts, missing, doubled, invalid, untracked, partialUntracked, faults };
  }
  const E2E_MARK = { pass: checkMark, partial: () => circleMark(.5, .5), fail: crossMark, blocked: slashMark, untested: () => circleMark(0), manual: () => circleMark(.5) };   // partial: the left half, amber; manual the right half
  // e2eChecklist({ promises, records, tasks, faults }): the summary band reads from across the room (state word, the owed count, one cell per
  // promise in group order; manual rows are listed in the headline, never folded into a pass, §4); the groups below carry each promise's status, last run (UTC) and the seat that ran it
  function e2eChecklist(opts) {
    let S; try { S = e2eSummary(opts.promises, opts.records, opts.tasks, opts.faults); } catch (e) { return el('div', { class: 'e2e-err' }, el('b', {}, 'No checklist: '), e.message); }
    const c = S.counts, n = S.rows.length, faults = S.missing.length + S.doubled.length + S.invalid.length;
    const plural = (k, w) => k + ' ' + w + (k === 1 ? '' : 's');
    const say = S.state === 'green' ? (c.manual ? [c.pass + ' pass', c.manual + ' manual'] : ['all ' + n + ' pass']) : S.state === 'amber' ? [[c.fail + c.blocked ? plural(c.fail + c.blocked, 'failure') + ' tracked' : null, c.partial ? c.partial + ' partial tracked' : null].filter(Boolean).join(', '), c.manual ? c.manual + ' manual' : null].filter(Boolean)
      : [S.faults.length ? plural(S.faults.length, 'fault') : null, c.untested - faults ? (c.untested - faults) + ' untested' : null, S.untracked.length ? plural(S.untracked.length, 'fail') + ' untracked' : null, S.partialUntracked.length ? S.partialUntracked.length + ' partial untracked' : null,
        S.missing.length ? S.missing.length + ' missing' : null, S.doubled.length ? S.doubled.length + ' doubled' : null, S.invalid.length ? S.invalid.length + ' invalid' : null].filter(Boolean);
    const runs = opts.records.filter(r => r.last_run), last = runs.length ? Math.max(...runs.map(r => Date.parse(r.last_run))) : null, seats = new Set(runs.map(r => r.run_by)).size;
    const groups = [...new Set(S.rows.map(r => r.group))];
    const cell = r => el('i', { class: 'e2c ' + r.status + ((r.failing || r.partial) && !r.tracked ? ' untracked' : '') + (r.fault ? ' fault' : ''), title: r.id + ' · ' + (r.fault || r.status) });
    const head = el('div', { class: 'e2e-head ' + S.state },
      el('div', { class: 'e2e-top' }, el('span', { class: 'e2e-word' }, S.state), el('span', { class: 'e2e-say' }, el('b', {}, say[0]), ...say.slice(1).map(x => el('span', {}, ' · ' + x))),
        el('span', { class: 'e2e-meta' }, n + ' promises' + (last ? ' · last run ' + hhmm(last) + ' UTC · ' + plural(seats, 'seat') : ' · never run'))),
      el('div', { class: 'e2e-strip' }, ...groups.map(g => el('span', { class: 'e2s', title: g }, ...S.rows.filter(r => r.group === g).map(cell)))),
      el('div', { class: 'e2e-counts' }, ...E2E_SHOW.filter(k => k !== 'partial' || c.partial).map(k => el('span', { class: 'e2k ' + k + (c[k] ? '' : ' zero') }, el('i', { class: 'e2c ' + k }), c[k] + ' ' + k))),
      ...S.faults.map(f => el('div', { class: 'e2e-fault', role: 'status' }, el('b', {}, 'fault'), ' · ' + f)));
    const row = r => el('div', { class: 'e2r ' + r.status + (r.failing || r.partial ? (r.tracked ? ' tracked' : ' untracked') : '') + (r.fault ? ' fault' : ''), title: r.text },
      el('code', { class: 'e2id' }, r.id), el('span', { class: 'e2m' }, E2E_MARK[r.status]()), el('span', { class: 'e2t' }, r.text),
      el('span', { class: 'e2s-word' }, r.fault || r.status), el('span', { class: 'e2when' }, r.last_run ? hhmm(Date.parse(r.last_run)) : '—'), el('span', { class: 'e2by' }, r.run_by ? r.run_by.replace(/^seat:/, '') : ''),
      r.failing || r.partial ? el('span', { class: 'e2note' }, r.partial ? el('span', { class: 'e2miss' }, 'partial · missing: ' + r.missing.join('; ')) : null, ...(r.tracked_by || []).map(t => el('span', { class: 'e2task ' + (E2E_OPEN.includes(t.status) ? 'open' : 'closed') }, t.ref + ' ' + t.status)), r.tracked ? null : el('b', { class: 'e2untr' }, 'untracked'), r.reason ? el('span', { class: 'e2why' }, r.reason) : null)
        : r.status === 'manual' ? el('span', { class: 'e2note' }, el('span', { class: 'e2why' }, 'Oracle’s eye' + (r.reason ? ' · ' + r.reason : ''))) : null);
    const body = el('div', { class: 'e2e-groups' }, ...groups.map(g => { const rs = S.rows.filter(r => r.group === g), ok = rs.filter(r => r.status === 'pass').length;
      return el('div', { class: 'e2g' }, el('div', { class: 'e2g-h' }, el('b', {}, g), el('span', { class: 'e2s' }, ...rs.map(cell)), el('span', { class: 'e2g-n' }, ok + ' / ' + rs.length + ' pass')), ...rs.map(row)); }));
    return el('div', { class: 'e2e' }, head, body);
  }
  function graphPage(opts) {
    opts = opts || {};
    const seats = opts.seats || [], epics = opts.epics || [], swarmName = opts.swarm || '';
    const root = el('div', { class: 'graph' });
    const agents = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Agents · ' + swarmName)));
    const hdr = el('div', { class: 'g-row g-hdr' }, el('span', {}, 'seat'), el('span', {}, 'role'), el('span', {}, 'now'), el('span', {}, 'blocked on'));
    agents.append(hdr);
    for (const s of seats) {
      const live = (s.tasks || []).find(t => ['claimed', 'running', 'reported'].includes(t.status));
      const blocked = (s.tasks || []).find(t => t.status === 'blocked');
      const tileCol = (LAUNCH.find(l => l.harness === s.harness) || {}).tile || 'var(--surface-2)';
      agents.append(el('div', { class: 'g-row', 'data-key': s.key }, el('span', { class: 'g-seat' }, el('span', { class: 'tile', style: 'background:' + tileCol }, harnessIcon(s.harness)), el('b', { style: 'color:' + seatColor(s) }, s.name), glyph(s.state, s)), el('span', {}, s.role && s.role !== 'shell' ? roleMark(s.role) : '', ' ', s.role || 'shell'), el('span', { class: 'g-now' }, live ? [el('span', { class: 'tglyph ' + live.status }, taskMark(live.status)), ' ', live.title] : (s.state === 'waiting' ? 'waiting · ' + (s.waitKind || 'answer') : s.state)), el('span', { class: 'g-block' }, blocked ? 'waits on ' + blocked.waits : (s.state === 'waiting' ? 'the operator' : '—'))));
    }
    const tree = el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Graph · epics → work')));
    const pre = el('div', { class: 'g-tree' });
    for (const ep of epics) {
      const done = (ep.items || []).filter(i => ['accepted', 'done'].includes(i.status)).length;
      pre.append(el('div', { class: 'g-epic' }, el('span', { class: 'ekind' }, 'epic'), el('b', {}, ep.title), el('span', { class: 'ecount' }, done + '/' + (ep.items || []).length)));
      (ep.items || []).forEach((it, i, arr) => {
        const last = i === arr.length - 1;
        const kind = it.type || 'task', glyphOf = kind === 'bug' ? harnessIcon('bug') : kind === 'invariant' ? '◆' : taskMark(it.status);
        const line = el('div', { class: 'g-item ' + it.status + ' k-' + kind }, el('span', { class: 'g-branch' }, last ? '└─' : '├─'), el('span', { class: 'tglyph' }, glyphOf), el('span', { class: 'ekind' }, kind === 'invariant' ? 'inv' : kind), el('span', { class: 'g-title' }, it.title));
        if (it.violates) line.append(invChip(it.violates, 'violates'));
        if (it.constrains) line.append(invChip(it.constrains, 'constrains'));
        if (kind === 'invariant') { pre.append(line); for (const b of (it.binds || [])) pre.append(el('div', { class: 'g-bind' }, el('span', { class: 'g-branch' }, (last ? '   ' : '│  ') + '└─'), el('span', { class: 'g-bound' + (b.rel === 'violated-by' ? ' bugb' : '') }, b.rel === 'violated-by' ? harnessIcon('bug') : '○'), el('span', { class: 'g-title' }, b.title), el('span', { class: 'g-rel ' + (b.rel || 'constrains') }, b.rel || 'constrains'))); return; }
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
    if (opts.channels && want('channels')) root.append(channelGraphCard(opts.channels));
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
    const items = epic.items || [], done = items.filter(i => ['accepted', 'done'].includes(i.status)).length;
    const g = el('div', { class: 'epic' + (epic.open === false ? ' shut' : '') });
    const head = el('div', { class: 'ehead', onclick: () => { epic.open = epic.open === false; if (opts.onToggle) opts.onToggle(epic); } }, el('span', { class: 'ecaret' }, epic.open === false ? '▸' : '▾'), el('span', { class: 'ekind' }, 'epic'), el('span', { class: 'etitle' }, epic.title), el('span', { class: 'ecount' }, done + '/' + items.length));
    g.append(head);
    if (epic.open !== false) for (const it of items) {
      // TaskCard: status is the work-items name; delegation sets claimed (task_delegate, work-items.md), so a fresh hand-off reads claimed
      const line = el('div', { class: 'eitem ' + it.status, title: it.title + ' · ' + it.status + (it.delegated_by ? ' · delegated by ' + it.delegated_by : '') }, el('span', { class: 'tglyph' }, taskMark(it.status)), el('span', { class: 'ekind' }, it.type || 'task'), el('span', { class: 'etitle' }, it.title));
      if (it.assignee) line.append(el('span', { class: 'eass' }, (it.status === 'claimed' ? '→ ' : '') + it.assignee));
      if (it.status === 'claimed') line.append(el('span', { class: 'echip' }, 'claimed'));
      if ((opts.delegate || opts.onDelegate) && it.status === 'open' && !it.assignee) line.append(el('button', { class: 'edel', onclick: e => { e.stopPropagation(); if (opts.onDelegate) opts.onDelegate(it); } }, 'Delegate…'));
      if (it.status === 'blocked' && it.waits) line.append(el('span', { class: 'twaits' }, 'waits on ' + it.waits));
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
  // Numbered records (decisions #47, #48): every record has one global number; [[#N]] and [[slug|#N]] link to it. RECORDS is the index a
  // page loads (number -> { n, title, type, path, slug }). With it, a numbered link resolves (the chip opens the record through a
  // bubbling doclink event, detail the record) or is missing ('#N · no record', dim, inert). A page with no index draws plain chips.
  const RECORDS = new Map();
  const recordOf = target => { const n = /^#(\d+)$/.exec(target); if (n) return RECORDS.get(+n[1]) || null; for (const r of RECORDS.values()) if (r.slug === target) return r; return null; };
  function renderLinks(text) {
    const frag = el('span', { class: 'links' }); let last = 0, m;
    while ((m = LINK_RE.exec(text))) {
      if (m.index > last) frag.append(text.slice(last, m.index));
      const numbered = RECORDS.size && (/^#\d+$/.test(m[1]) || /^#\d+$/.test(m[2] || ''));
      if (numbered) {
        const rec = recordOf(m[1]), label = /^#\d+$/.test(m[1]) ? m[1] : m[2];
        const chip = rec ? el('span', { class: 'wl rec', role: 'link', tabindex: '0', title: 'opens #' + rec.n + ' · ' + rec.type + ' · ' + rec.title }, el('span', { class: 'wl-slug' }, label))
          : el('span', { class: 'wl missing', title: label + ' · no record with that number' }, el('span', { class: 'wl-slug' }, label + ' · no record'));
        if (rec) chip.addEventListener('click', () => chip.dispatchEvent(new CustomEvent('doclink', { detail: rec, bubbles: true })));
        frag.append(chip); last = m.index + m[0].length; continue;
      }
      const chip = el('span', { class: 'wl', title: m[1] + (m[3] ? ' · ' + m[3] + (m[4] ? ' · weight ' + m[4] : '') : '') }, el('span', { class: 'wl-slug' }, m[2] || m[1]));
      if (m[3]) chip.append(el('span', { class: 'wl-rel ' + m[3] }, m[3] + (m[4] ? ' ' + m[4] : '')));
      frag.append(chip); last = m.index + m[0].length;
    }
    if (last < text.length) frag.append(text.slice(last));
    return frag;
  }

  // Event bus (events.md, Bus design): subject tokens coloured by root, the kind (last token) emphasised, seq per stream,
  // tier mark (stored ● appended before delivery, fan-out ○ live only), request-reply pairs linked, gaps spelled.
  const ordered = (src, keys) => Object.fromEntries(keys.map(k => [k, src[k]]));   // tokens.js sorts keys; legends keep semantic order
  const EV_ROOT = ordered(T.color.subject, ['seat', 'doc', 'task', 'swarm', 'ext', 'mux', 'inbox', 'artifact', 'dm', 'channel', 'cloud']);   // tokens.toml [color.subject]
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
  const DG_ROLE = { architect: 'var(--role-architect)', oracle: 'var(--role-oracle)', executor: 'var(--role-executor)', creativity: 'var(--role-creativity)', idle: 'var(--c-idle)', waiting: 'var(--c-waiting)', errored: 'var(--c-errored)', accent: 'var(--accent)' };
  function parseMermaid(src) {
    const lines = src.split('\n').map(l => l.replace(/%%.*$/, '').trim()).filter(Boolean);
    const head = (lines.shift() || '').toLowerCase();
    if (/^(flowchart|graph)\b/.test(head)) {
      const dir = /\b(lr|rl)\b/.test(head) ? 'LR' : 'TD', nodes = new Map(), edges = [];
      const node = tok => { const m = tok.trim().match(/^([A-Za-z0-9_]+)(?:(\[\[?|\(\(?|\{)(.*?)(\]\]?|\)\)?|\}))?(?::::(\w+))?$/); if (!m) throw new Error('node: ' + tok);
        const id = m[1], n = nodes.get(id) || { id, label: id, shape: 'rect' }; if (m[3] != null) { n.label = m[3]; n.shape = m[2] === '(' ? 'stadium' : m[2] === '((' ? 'circle' : m[2] === '{' ? 'diamond' : 'rect'; if (m[2] === '[' && /^\(.*\)$/.test(m[3])) { n.label = m[3].slice(1, -1); n.shape = 'store'; } if (m[2] === '[[') n.link = m[3]; } if (m[5]) n.cls = m[5]; nodes.set(id, n); return id; };
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
  // Diagram figure (7.10 DocumentModal pick, diagrams.html 'in the modal · shapes'): every Mermaid block in a document is this
  // card. Header: kind · node count · Drawing / Source · ⤢ (opts.onExpand: the pan-and-zoom canvas for big graphs). Body: the
  // drawing at true size, scrolling sideways under a fade when wider than the column. [[slug]] nodes are live links.
  const DG_KIND = { flow: 'flowchart', seq: 'sequence' };
  function diagramFigure(src, opts) {
    opts = opts || {};
    const draw = diagram(src); if (draw.classList.contains('dg-raw')) return draw;
    let g = {}; try { g = parseMermaid(src); } catch (e) {}
    const head = src.trim().split(/\s/)[0], kind = /^state/.test(head) ? 'state' : /^sequence/.test(head) ? 'sequence' : 'flowchart';
    const n = g.nodes ? g.nodes.filter(x => x.label).length : (g.parts || []).length, unit = kind === 'sequence' ? ' participants' : ' nodes';
    const body = el('div', { class: 'dgfig-b' }, draw), pre = el('pre', { class: 'dgfig-src' }, src.trim());
    const seg = k => el('button', { class: k === 'Drawing' ? 'on' : '', onclick: () => { body.replaceChildren(k === 'Drawing' ? draw : pre); fig.querySelectorAll('.dgfig-h .seg button').forEach(b => b.classList.toggle('on', b.textContent === k)); } }, k);
    const fig = el('figure', { class: 'dgfig' }, el('div', { class: 'dgfig-h' }, el('span', { class: 'kind' }, kind), el('span', { class: 'cnt' }, n + unit), el('span', { class: 'seg' }, seg('Drawing'), seg('Source')),
      el('button', { class: 'exp', title: 'open on its own canvas: pan, zoom, minimap', onclick: () => opts.onExpand && opts.onExpand(src) }, '⤢')), body);
    // the fade marks hidden width; it shows only while the drawing is wider than the column
    requestAnimationFrame(() => { const w = draw.classList.contains('dg-wrap') ? draw : draw.querySelector('.dg-wrap'); if (w && w.scrollWidth > w.clientWidth + 1) w.classList.add('wide'); });
    return fig;
  }
  // Illustrative layout only (renderer.md §4.4c): diagram bodies are laid out by the Rust engine bbt-mermaid and are not a
  // parity reference here; the figure card around them (diagramFigure) still is.
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
      if (n.label) txt(p.x, p.y, n.label, 'dg-t', gr);
      // live link node: A[[slug]] names a document; click (⌘-click in a pane) opens it in the modal stack via a bubbling doclink event
      if (n.link) { gr.classList.add('link'); const t = document.createElementNS(NS, 'title'); t.textContent = 'opens [[' + n.link + ']]'; gr.append(t); gr.addEventListener('click', () => gr.dispatchEvent(new CustomEvent('doclink', { detail: n.link, bubbles: true }))); } }
    svg.querySelectorAll('text.pill').forEach(t => { const w = t.textContent.length * 5.8 + 10; const r = mk('rect', { x: +t.getAttribute('x') - w / 2, y: +t.getAttribute('y') - 8, width: w, height: 16, rx: 8, class: 'dg-pill' }); svg.append(r, t); });
    svg.setAttribute('viewBox', `0 0 ${W} ${H}`); svg.setAttribute('width', W);
    return el('div', { class: 'dg-wrap' }, svg);
  }
  // Document view (work-items.md): frontmatter as a header, Markdown body with annotated wikilinks, a provenance strip, and a right
  // rail with the item's neighbourhood in the markdown graph (weights as stroke). Broken frontmatter shows as broken, never skipped.
  const DOC_HUE = ordered(T.color.doc, ['epic', 'spec', 'task', 'bug', 'note', 'invariant', 'artifact']);   // tokens.toml [color.doc]
  // The header already shows the frontmatter title; a body that opens with '# <same title>' would repeat it. Drop that one
  // leading H1 when it equals the title (case, whitespace and dash style ignored); any other first heading stays.
  const sameTitle = (a, b) => { const n = x => String(x || '').toLowerCase().replace(/[\u2012-\u2015-]/g, '-').replace(/\s+/g, ' ').trim(); return !!a && n(a) === n(b); };
  function bodyAfterTitle(body, title) {
    const lines = String(body || '').split('\n'); let i = 0; while (i < lines.length && !lines[i].trim()) i++;
    const h = (lines[i] || '').match(/^# (.*)/);
    return h && sameTitle(h[1], title) ? lines.slice(i + 1).join('\n').replace(/^\s*\n/, '') : String(body || '');
  }
  function mdLite(src) {
    const out = el('div', { class: 'md' }); let list = null, code = null;
    for (const line of src.split('\n')) {
      if (line.startsWith('```')) { if (list && !code) { out.append(list); list = null; } if (code) { out.append(code.mermaid ? diagramFigure(code.textContent) : code); code = null; } else { code = el('pre', {}, ''); code.mermaid = /mermaid/.test(line); } continue; }
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
    const svg = document.createElementNS(NS, 'svg'); svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H); svg.setAttribute('width', W); svg.setAttribute('height', H); svg.setAttribute('class', 'nbhd');
    const mk = (tag, a) => { const e = document.createElementNS(NS, tag); for (const k in a) e.setAttribute(k, a[k]); svg.append(e); return e; };
    (doc.near || []).forEach((x, i) => {
      const t = -Math.PI / 2 + i * 2 * Math.PI / n, r = 78 - (x.w || 1) * 6, X = cx + Math.cos(t) * r, Y = cy + Math.sin(t) * r;
      mk('line', { x1: cx, y1: cy, x2: X, y2: Y, style: 'stroke:var(--fg-dim);stroke-opacity:.45;stroke-width:' + (0.8 + (x.w || 1) * 0.7) + (x.back ? ';stroke-dasharray:3 3' : '') });
      mk('circle', { cx: X, cy: Y, r: 5, style: 'fill:' + (DOC_HUE[x.type] || 'var(--fg-dim)') });
      const tx = mk('text', { x: X, y: Y + (Y < cy ? -9 : 15), 'text-anchor': 'middle', style: 'fill:var(--fg);font-size:9.5px;font-family:var(--font-mono)' }); tx.textContent = x.slug;
      if (x.rel) { const tr = mk('text', { x: (cx + X) / 2, y: (cy + Y) / 2 - 3, 'text-anchor': 'middle', style: 'fill:var(--fg-dim);font-size:var(--t-micro)' }); tr.textContent = x.rel + (x.w > 1 ? ' ' + x.w : ''); }
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
    const glyphFor = doc.type === 'task' ? taskMark(doc.status) : null;
    const head = el('div', { class: 'doc-head', style: '--doc:' + hue },
      el('div', { class: 'doc-top' }, el('span', { class: 'doc-type' }, doc.type), el('span', { class: 'doc-status ' + (doc.status || 'none') }, glyphFor, doc.status || 'current'), el('code', { class: 'doc-path' }, doc.path || '')),
      el('h1', {}, doc.title));
    if (doc.superseded_by) head.append(el('div', { class: 'doc-super' }, 'superseded by ', renderLinks(doc.superseded_by), ' · kept for provenance'));
    if (doc.broken) head.append(el('div', { class: 'doc-broken' }, 'frontmatter does not parse · ' + doc.broken + ' · shown, never skipped'));
    const pv = doc.prov || {};
    const prov = el('div', { class: 'doc-prov' + (pv.signed ? ' signed' : ' unsigned') },
      el('span', {}, 'created by ', el('b', { style: 'color:' + (pv.createdColor || 'var(--fg)') }, pv.created_by || 'unknown')),
      pv.updated_by ? el('span', {}, 'updated by ', el('b', { style: 'color:' + (pv.updatedColor || 'var(--fg)') }, pv.updated_by)) : null,
      pv.derived_from ? el('span', {}, 'derived from ', renderLinks(pv.derived_from)) : null,
      el('span', { class: 'dig' }, el('code', {}, (pv.digest || '').slice(0, 12)), pv.signed ? el('i', { class: 'ok' }, '✓ signed · ' + pv.signed) : el('i', { class: 'no' }, 'unsigned · no write is anonymous: updated_by stamped')));
    const main = el('div', { class: 'doc-main' }, head, fm, mdLite(bodyAfterTitle(doc.body, doc.title)), prov);
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
    const view = plain ? el('div', { class: 'doc-view plain' }, el('div', { class: 'doc-main' }, el('div', { class: 'doc-head' }, el('div', { class: 'doc-top' }, el('code', { class: 'doc-path', style: 'margin-left:0' }, doc.path || '')), el('h1', {}, doc.title || doc.path)), mdLite(bodyAfterTitle(doc.body, doc.title || doc.path)))) : docView(doc);
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
  // First run (identity.md bootstrap, I2): select and pin an existing GPG primary; sign through gpg-agent (pinentry, never our
  // field); then create this device's key, certified by the primary, and account head epoch 1. Never create a root, never show
  // words as a root. No key: point at gpg. Primary elsewhere: a signed enrollment request, awaiting the primary's signature.
  const fp4 = f => f.match(/.{4}/g).join(' ');
  function firstRun(st, opts) {
    opts = opts || {};
    const keys = opts.keys || [
      { uid: 'Boxy (bboxy450) <bboxy450@gmail.com>', fp: '9B07E849480C68328CA20684844ECC0587DE7AB0', algo: 'ed25519', created: '2026-03-02', expires: 'never', where: 'secret key here · gpg-agent' },
      { uid: 'Boxy (work laptop) <boxy@example.com>', fp: '3C51A9E0D4F2B78811C6E0A9D27F4B13E5C08A66', algo: 'rsa4096', created: '2019-08-14', expires: '2027-08-14', where: 'secret key here · gpg-agent', note: 'RSA primaries are not supported yet (identity.md gap)' },
    ];
    let sel = opts.selected || 0;
    const panel = el('div', { class: 'first-run', role: 'dialog', 'aria-label': 'first run' });
    const head = (t, sub) => el('div', { class: 'fr-head' }, logo(40, { square: true }), el('div', {}, el('div', { class: 'fr-title' }, t), el('div', { class: 'fr-sub' }, sub)));
    const steps = n => el('ol', { class: 'fr-steps' }, ...['Pin your GPG primary', 'Certify this Mac’s device key', 'Write account head, epoch 1'].map((t, i) => el('li', { class: i < n ? 'done' : i === n ? 'now' : '' }, el('i', {}, i < n ? '✓' : String(i + 1)), t)));
    const keyRow = (k, i) => el('label', { class: 'fr-key' + (i === sel ? ' on' : '') + (k.note ? ' unsupported' : ''), onclick: () => { if (k.note) return; sel = i; panel.replaceWith(firstRun(st, { ...opts, selected: i })); } },
      el('span', { class: 'radio' }), el('div', { class: 'fr-k' }, el('b', {}, k.uid), el('code', { class: 'fr-fp' }, fp4(k.fp)), el('span', { class: 'fr-meta' }, k.algo + ' · created ' + k.created + ' · expires ' + k.expires + ' · ' + k.where), k.note ? el('span', { class: 'fr-note' }, k.note) : null));
    const btn = (t, cls, on) => el('button', { class: 'fr-btn ' + (cls || ''), onclick: on || null }, t);
    if (st === 'keys found') panel.append(head('Who are you?', 'bboxy450-term uses the GPG key you already sign with. Pick it; nothing new is created.'), steps(0),
      el('div', { class: 'fr-keys' }, ...keys.map(keyRow)),
      el('div', { class: 'fr-acts' }, el('span', { class: 'fr-hint' }, 'Pinning signs one bootstrap record through gpg-agent. If the key is locked, pinentry asks in its own window.'), btn('Pin this key', 'primary', () => opts.onPin && opts.onPin(keys[sel]))));
    if (st === 'signing') panel.append(head('Signing with your key', 'gpg-agent has the request. If pinentry is open, answer it there; this window never asks for your passphrase.'), steps(0),
      el('div', { class: 'fr-wait' }, el('i', { class: 'spin' }), el('div', {}, el('b', {}, 'Waiting for gpg-agent'), el('code', { class: 'fr-fp' }, fp4(keys[0].fp)))), el('div', { class: 'fr-acts' }, btn('Cancel', 'ghost')));
    if (st === 'pinned') panel.append(head('You are Boxy', 'Pinned. This Mac now has its own device key, certified by your primary.'), steps(3),
      el('div', { class: 'fr-done' }, el('div', {}, el('span', { class: 'k' }, 'primary'), el('code', { class: 'fr-fp' }, fp4(keys[0].fp))), el('div', {}, el('span', { class: 'k' }, 'uid'), el('span', {}, keys[0].uid)), el('div', {}, el('span', { class: 'k' }, 'device'), el('span', {}, 'this Mac · certified leaf key · epoch 1'))),
      el('div', { class: 'fr-acts' }, el('span', { class: 'fr-hint' }, 'Google can be linked later for cloud; it is a link, never a sign-in.'), btn('Open the window', 'primary', () => opts.onDone && opts.onDone())));
    if (st === 'no secret key') panel.append(head('No signing key found', 'There is no secret key in your GnuPG keyring. Make one with gpg, then come back; bboxy450-term will not create a root for you.'), steps(0),
      el('pre', { class: 'fr-cmd' }, "gpg --quick-generate-key 'Your Name <you@example.com>' ed25519 sign never"),
      el('div', { class: 'fr-acts' }, el('span', { class: 'fr-hint' }, 'Back it up the way you back up any GPG key. Recovery restores identity only.'), btn('Look again', 'primary')));
    if (st === 'primary unavailable') panel.append(head('Your primary is not on this Mac', 'The keyring has the public certificate for Boxy but not the secret key: it lives on another machine or a card.'), steps(0),
      el('div', { class: 'fr-keys' }, el('div', { class: 'fr-key on' }, el('span', { class: 'radio' }), el('div', { class: 'fr-k' }, el('b', {}, keys[0].uid), el('code', { class: 'fr-fp' }, fp4(keys[0].fp)), el('span', { class: 'fr-meta' }, 'ed25519 · public certificate only · secret key elsewhere')))),
      el('div', { class: 'fr-acts' }, el('span', { class: 'fr-hint' }, 'This Mac makes a signed enrollment request; sign it wherever your primary lives.'), btn('Insert the card and look again', 'ghost'), btn('Make an enrollment request', 'primary', () => opts.onRequest && opts.onRequest())));
    if (st === 'enrollment awaiting signature') panel.append(head('Waiting for your primary', 'This Mac made its device key and an enrollment request. Sign the request where your primary lives; nothing here pretends it is signed.'), steps(1),
      el('div', { class: 'fr-done' }, el('div', {}, el('span', { class: 'k' }, 'request'), el('code', {}, '~/Library/Application Support/bboxy450-term/enroll-2026-10-21.req')), el('div', {}, el('span', { class: 'k' }, 'digest'), el('code', {}, 'sha256 4be1 77c0 9d2a 61f3')), el('div', {}, el('span', { class: 'k' }, 'for'), el('code', { class: 'fr-fp' }, fp4(keys[0].fp)))),
      el('pre', { class: 'fr-cmd' }, 'bboxy450-term enroll sign enroll-2026-10-21.req   # where the primary is'),
      el('div', { class: 'fr-acts' }, el('span', { class: 'fr-hint' }, 'Until it is signed this Mac runs local seats only; account messaging waits.'), btn('Continue with local seats', 'ghost', () => opts.onDone && opts.onDone()), btn('Import the signed request', 'primary')));
    return panel;
  }
  // subchannels as tabs in the stream dock (messaging.md Subchannels): unread counts, per-subchannel mute (right-click or long-press a tab)
  function streamTabs(chs, opts) {
    opts = opts || {};
    const bar = el('div', { class: 'ch-tabs', role: 'tablist' });
    // the server's swarm total (opts.total) is authoritative when given: it also counts bare-stream messages, so the subchannels
    // need not sum to it; without it, 'all' is 'more' when a subchannel's count is not proven, else the sum
    const gap = chs.some(c => !c.muted && c.unread === 'more');
    const all = { name: 'all', unread: opts.total != null ? opts.total : gap ? 'more' : chs.reduce((n, c) => n + (c.muted ? 0 : c.unread || 0), 0) };
    for (const c of [all, ...chs]) {
      const t = el('button', { class: 'ch-tab' + (opts.sel === c.name ? ' on' : '') + (c.muted ? ' muted' : ''), role: 'tab', title: c.name === 'all' ? 'the swarm stream: chunks, decisions, presence' : '#' + c.name + (c.muted ? ' · muted (right-click to unmute)' : ' · right-click to mute'), onclick: () => opts.onSel && opts.onSel(c.name) },
        c.name === 'all' ? 'all' : '#' + c.name, c.muted ? harnessIcon('muted') : c.unread ? el('b', { class: c.unread === 'more' ? 'more' : null }, String(c.unread)) : null);
      if (c.name !== 'all') t.addEventListener('contextmenu', e => { e.preventDefault(); opts.onMute && opts.onMute(c.name); });
      bar.append(t);
    }
    return bar;
  }
  // channel graph (channel-tree.md): pubsub as a filesystem of sinks under bboxy450://. Folders (channels, subchannels, groups,
  // ext sources) on the left as a tree; seat sinks (seats/<seat>/inbox) on the right; each subscription is a symlink drawn as a
  // thin dashed edge, traffic adds weight; filters and rates ride on the link as a pill; a paused (muted) entry is dotted;
  // folder links (bridges) curve between folders.
  // Drawn 1:1: the viewBox is the graph's own pixel box (no scaling to the card), labels sit on the type steps (caption folders and
  // sinks, micro notes and paths, tiny pills) measured with the real font, strokes are pixel widths. The card scrolls if narrower.
  const CG_TEXT = { folder: ['caption', 400, 'mono'], note: ['micro', 400, 'chrome'], sink: ['caption', 600, 'chrome'], path: ['micro', 400, 'mono'], pill: ['tiny', 400, 'mono'] };
  let cgCtx;
  const cgWidth = (kind, text) => {
    const [step, weight, fam] = CG_TEXT[kind], css = getComputedStyle(document.documentElement);
    cgCtx = cgCtx || document.createElement('canvas').getContext('2d');
    cgCtx.font = weight + ' ' + window.STYLE.type[step] + 'px ' + css.getPropertyValue('--font-' + fam).trim();
    return cgCtx.measureText(text).width;
  };
  function channelGraphCard(g) {
    const NS = 'http://www.w3.org/2000/svg', rowH = 22, top = 12, span = 220;
    const fy = {}, sy = {};
    g.folders.forEach((f, i) => { fy[f.path] = top + i * rowH + rowH / 2; });
    const sinkGap = Math.max(rowH, (g.folders.length * rowH) / Math.max(1, g.sinks.length));
    g.sinks.forEach((x, i) => { sy[x.id] = top + i * sinkGap + sinkGap / 2; });
    const H = Math.ceil(Math.max(g.folders.length * rowH, g.sinks.length * sinkGap) + top * 2);
    const fx = f => 12 + f.depth * 14, fEnd = f => fx(f) + cgWidth('folder', f.label) + (f.filter ? 8 + cgWidth('note', f.filter) : 0);
    const bridgeX = br => { const a = g.folders.find(x => x.path === br.from), b = g.folders.find(x => x.path === br.to); return a && b ? Math.max(fEnd(a), fEnd(b)) + 14 : 0; };
    const left = Math.max(0, ...g.folders.map(fEnd), ...(g.bridges || []).map(br => bridgeX(br) + 28 + cgWidth('pill', 'bridge')));
    const sinkX = Math.round(left + span), W = Math.ceil(sinkX + 10 + Math.max(0, ...g.sinks.map(x => Math.max(cgWidth('sink', x.name), cgWidth('path', 'seats/' + x.id + '/inbox')))) + 12);
    const svg = document.createElementNS(NS, 'svg'); svg.setAttribute('viewBox', '0 0 ' + W + ' ' + H); svg.setAttribute('width', W); svg.setAttribute('height', H); svg.setAttribute('class', 'chgraph');
    const mk = (tag, a, txt) => { const e = document.createElementNS(NS, tag); for (const k in a) e.setAttribute(k, a[k]); if (txt != null) e.textContent = txt; svg.append(e); return e; };
    const col = id => (g.sinks.find(x => x.id === id) || {}).color || 'var(--accent)';
    for (const l of g.links) { const f = g.folders.find(x => x.path === l.from); if (!f || sy[l.to] == null) continue;
      const x1 = fEnd(f) + 6, y1 = fy[l.from], x2 = sinkX - 8, y2 = sy[l.to], mx = (x1 + x2) / 2, d = `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`;
      mk('path', { d, class: 'cg-link' + (l.paused ? ' paused' : '') });
      if (l.w) mk('path', { d, class: 'cg-traffic', style: 'stroke-width:' + (1 + l.w * 5) + ';stroke:' + col(l.to) });
      // the pill rides 30% along its own curve, near its folder, so pills from neighbouring folders (rowH apart) never stack
      if (l.filter || l.rate) { const t = [l.filter, l.rate].filter(Boolean).join(' · '), w = cgWidth('pill', t) + 12, u = .3, v = 1 - u;
        const cx = v * v * v * x1 + 3 * v * v * u * mx + 3 * v * u * u * mx + u * u * u * x2, cy = y1 + (y2 - y1) * (3 * u * u - 2 * u * u * u);
        mk('rect', { x: cx - w / 2, y: cy - 7, width: w, height: 14, rx: 7, class: 'cg-pill' }); mk('text', { x: cx, y: cy, 'text-anchor': 'middle', 'dominant-baseline': 'central', class: 'cg-pill-t' }, t); } }
    for (const br of (g.bridges || [])) { const a = g.folders.find(x => x.path === br.from), b = g.folders.find(x => x.path === br.to); if (!a || !b) continue; const x = bridgeX(br); mk('path', { d: `M${fEnd(a) + 4},${fy[a.path]} C${x + 20},${fy[a.path]} ${x + 20},${fy[b.path]} ${fEnd(b) + 4},${fy[b.path]}`, class: 'cg-bridge' }); mk('text', { x: x + 24, y: (fy[a.path] + fy[b.path]) / 2, class: 'cg-pill-t', 'dominant-baseline': 'central' }, 'bridge'); }
    g.folders.forEach(f => { const y = fy[f.path]; if (f.depth) mk('path', { d: `M${fx(f) - 8.5},${y - rowH + 7} V${y + .5} H${fx(f) - 3}`, class: 'cg-branch' });
      mk('text', { x: fx(f), y, 'dominant-baseline': 'central', class: 'cg-folder ' + (f.kind || 'channel') }, f.label);
      if (f.filter) mk('text', { x: fx(f) + cgWidth('folder', f.label) + 8, y, 'dominant-baseline': 'central', class: 'cg-note' }, f.filter); });
    g.sinks.forEach(x => { const y = sy[x.id]; mk('circle', { cx: sinkX, cy: y, r: 3.5, style: 'fill:' + (x.color || 'var(--fg)') }); mk('text', { x: sinkX + 10, y: y - 6, 'dominant-baseline': 'central', class: 'cg-sink', style: 'fill:' + (x.color || 'var(--fg)') }, x.name); mk('text', { x: sinkX + 10, y: y + 6, 'dominant-baseline': 'central', class: 'cg-path' }, 'seats/' + x.id + '/inbox'); });
    return el('div', { class: 's-card' }, el('div', { class: 's-card-h' }, el('span', {}, 'Channels · ' + (g.root || 'bboxy450://')), el('span', { class: 'cg-key' }, el('i', { class: 'a' }), 'link', el('i', { class: 't' }), 'traffic', el('i', { class: 'p' }), 'paused')), el('div', { class: 'cg-wrap' }, svg));
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
      for (const t of (s.tasks || [])) { w.append(taskRow(t, s, { onAccept: opts.onAccept, review: !!(opts.review || opts.onAccept) })); for (const a of (t.artifacts || [])) w.append(artifactRow(a)); }
      return w;
    }
    // the tab's alive border reads its worst seat (rim-glow §8): state and colour
    const worst = rollup(seats)[0];
    const row = el('div', { class: 'tab-row', 'data-rim-key': tab.name, 'data-rim-state': worst.state, style: '--seat:' + seatColor(worst) }, el('span', { class: 'caret' }, opts.expanded ? '▾' : '▸'), el('span', { class: 'tname' }, tab.name));
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
      for (const s of seats) { body.append(seatRow(s, { focused: s.key === opts.focusedKey }), ...subagentRows(s)); for (const t of (s.tasks || [])) { body.append(taskRow(t, s, { onAccept: opts.onAccept, review: !!(opts.review || opts.onAccept) })); for (const a of (t.artifacts || [])) body.append(artifactRow(a)); } }
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
      // closed, in grace (T7): the process still runs; the seconds left and a hairline draining over the grace window (timestamped, so
      // re-renders resume it); undo or ⌘Z restores it in place
      for (const c of (space.closed || [])) body.append(el('div', { class: 'tab-row closed-strip', 'data-closed': c.seat.key, title: 'closed · the process keeps running until the grace ends · ⌘Z restores it in place' }, el('span', { class: 'x' }, '×'), harnessIcon(c.seat.harness), el('span', { class: 'tname' }, c.seat.name), el('span', { class: 'right' }, el('span', { class: 'kbd left', title: 'seconds of grace left' }, c.closedAt ? Math.max(0, Math.ceil(((c.graceMs || 60000) - (Date.now() - c.closedAt)) / 1000)) + ' s' : ''), el('button', { class: 'undo', onclick: e => { e.stopPropagation(); st.onUndo && st.onUndo(c); } }, 'undo ⌘Z')), el('i', { class: 'grace', style: 'animation-duration:' + (c.graceMs || 60000) + 'ms;animation-delay:' + (c.closedAt ? '-' + Math.max(0, Date.now() - c.closedAt) + 'ms' : '0ms') })));
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
    // the logic process is reconnecting: rows stay, dimmed and frozen, under a banner naming when the states were last true
    if (opts.stale) { sb.classList.add('stale'); sb.insertBefore(el('div', { class: 'sb-stale', role: 'status' }, el('i', { class: 'spin' }), el('span', {}, 'reconnecting to logic · states as of ' + opts.stale.asOf)), scroll); }
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
    if (seat.resumed) t.prepend(el('div', { class: 'gap-mark resumed', title: seat.resumed.cmd || 'new session; the old output is replayed above, read-only' }, el('i'), el('span', {}, 'resumed · process generation ' + seat.resumed.gen + ' · ' + (seat.resumed.cmd || 'new shell, old output above')), el('i')));
    if (seat.gap) t.prepend(gapMarker(seat.gap));
    return t;
  }
  // scrollback gap (mux ReadRecords → Gap): the caption comes only from the supplied retained range; never an invented line count
  function gapMarker(range) {
    const cap = range && range.from ? 'earlier output evicted · retained from ' + range.from : 'earlier output unavailable';
    return el('div', { class: 'gap-mark', title: cap + (range && range.of ? ' · ' + range.of : '') }, el('i'), el('span', {}, cap), el('i'));
  }
  // authority (identity.md): expired | revoked | forked. Independent of the state glyph; blocks only what needs the seat's signature.
  const AUTH_SAY = { expired: 'authority expired · renew the seat grant', revoked: 'revoked here · peers learn on next contact', forked: 'forked · two heads at one epoch · the operator resolves' };
  const authMark = seat => seat.authority ? el('span', { class: 'authmark ' + seat.authority, title: AUTH_SAY[seat.authority] + ' · signed effects are blocked; the seat keeps running' }, harnessIcon('keyx')) : null;
  // one-shot notices survive re-renders: a timestamp value resumes the animation at its elapsed point (true = start now)
  const since = v => typeof v === 'number' ? '-' + Math.max(0, Date.now() - v) + 'ms' : '0ms';
  function titleStrip(seat, opts) {
    opts = opts || {};
    const s = el('div', { class: 'strip', style: 'background:' + seatColor(seat) }, el('span', { class: 'close', title: 'close seat', onclick: e => { e.stopPropagation(); if (opts.onClose) opts.onClose(seat); } }));
    if (seat.where && seat.where !== 'local') s.append(el('span', { class: 'where', title: seat.where + (seat.host ? ' · ' + seat.host : '') }, harnessIcon(seat.where)));
    s.append(harnessIcon(seat.harness), el('span', { class: 'name' }, seat.name));
    if (seat.role && seat.role !== 'shell') s.append(roleMark(seat.role));
    if (seat.authority) s.append(authMark(seat), el('span', { class: 'authsay' }, AUTH_SAY[seat.authority]));
    s.append(el('span', { class: 'title' }, (seat.title || '') + (seat.state === 'waiting' ? ' · waiting for ' + (seat.waitKind || 'an answer') : '')));
    if (opts.shortcut) s.append(kbd(opts.shortcut));
    // stop (seat_interrupt, Oracle/operator only: the caller passes onInterrupt only when allowed). A request, not a promise.
    if (seat.interruptSent) { s.classList.add('isent'); s.style.setProperty('--isd', since(seat.interruptSent)); s.append(el('span', { class: 'isent-chip', style: 'animation-delay:' + since(seat.interruptSent), title: 'interrupt sent · the seat decides when it stops' }, 'interrupt sent')); }
    if (opts.onInterrupt) s.append(el('span', { class: 'stop', title: 'interrupt (⌘.) · a request, the seat decides', onclick: e => { e.stopPropagation(); opts.onInterrupt(seat); } }, el('i')));
    if (opts.onMute) s.append(el('span', { class: 'mute' + (seat.muted ? ' on' : '') + (seat.authority ? ' blocked' : ''), title: seat.authority ? 'mute notices are signed by the seat · ' + AUTH_SAY[seat.authority] : seat.muted ? 'unmute (⌘M)' : 'mute: 1:1 with the operator (⌘M)', onclick: e => { e.stopPropagation(); if (!seat.authority) opts.onMute(seat); } }, harnessIcon(seat.muted ? 'muted' : 'unmuted')));
    s.append(glyph(seat.state, seat));
    return s;
  }
  // lifecycle: a seat whose harness is booting shows its launch steps (never a blank pane); a seat whose process exited keeps
  // its last screen readable (the evidence) with a bar to bring it back: same name, colour and seat, a new process generation
  const exitSay = x => (x && x.signal ? 'signal ' + x.signal : 'exit code ' + (x && x.code != null ? x.code : '?')) + (x && x.at ? ' · ' + x.at : '');
  function bootView(seat) {
    const label = (LAUNCH.find(l => l.harness === seat.harness) || { label: seat.harness }).label;
    return el('div', { class: 'term boot' }, el('div', { class: 'boot-head' }, el('i', { class: 'spin' }), 'Starting ' + label),
      el('ol', { class: 'boot-steps' }, ...((seat.boot && seat.boot.steps) || []).map(st => el('li', { class: st.done ? 'done' : 'now' }, el('span', { class: 'mk' }, st.done ? '✓' : '·'), el('span', { class: 'lbl' }, st.label), el('span', { class: 'ms' }, st.ms || '')))));
  }
  function exitBar(seat, opts) {
    return el('div', { class: 'exitbar', role: 'status' }, glyph('errored', seat), el('span', { class: 'say' }, el('b', {}, 'Process exited'), ' · ' + exitSay(seat.exit) + (seat.exit && seat.exit.gen ? ' · generation ' + seat.exit.gen : '')),
      el('span', { class: 'spacer' }), el('button', { class: 'bring', onclick: () => opts.onBringBack && opts.onBringBack(seat) }, 'Bring back', kbd('↵')), el('button', { class: 'dismiss', onclick: () => opts.onClose && opts.onClose(seat) }, 'Close'));
  }
  function pane(seat, opts) {
    opts = opts || {};
    const p = el('div', { class: 'pane ' + seat.state + (opts.unfocused ? ' unfocused' : '') + (opts.focused ? ' focused' : ''), style: '--seat:' + seatColor(seat) + (opts.height ? ';min-height:' + opts.height + 'px' : '') });
    p.append(titleStrip(seat, opts), seat.life === 'starting' ? bootView(seat) : fakeTerm(seat));
    if (seat.life === 'exited') { p.classList.add('exited'); p.append(exitBar(seat, opts)); }
    // the GUI lost the daemon: keep the stale frame, dim it, freeze the glow, centred pill; never blank, never modal
    // a remote seat's own link (ssh, cloud) uses the same pill, naming the host and the provider's reason
    const lk = seat.link && seat.where && seat.where !== 'local' ? linkOf(seat.link) : null;
    if (lk && (lk.kind === 'connecting' || linkDown(seat.link))) opts = { ...opts, reconnecting: (seat.host || seat.where) + ' · ' + (lk.kind === 'connecting' ? 'reconnecting…' : linkSay(seat.link)) };
    if (opts.reconnecting || opts.reconnected) { if (opts.reconnecting) p.classList.add('stale'); p.append(el('div', { class: 'reconnect' + (opts.reconnected ? ' out' : '') }, el('i', { class: 'spin' }), opts.reconnected ? 'reconnected' : typeof opts.reconnecting === 'string' ? opts.reconnecting : 'reconnecting…')); }
    p.dataset.key = seat.key;
    return syncPulse(p, seat.state);
  }

  // Host link (bbt-mux provider LinkState, its serde shape): 'Offline' | 'Connecting' | { Connected: { rtt_ms } } |
  // { Degraded: { rtt_ms } } | { Disconnected: { reason } }. One dot and one line of text. A host that is offline or
  // disconnected never launches (D1b: down hosts never execute); its row stays listed and says why.
  const linkOf = st => typeof st === 'string' ? { kind: st.toLowerCase() } : (([k, v]) => ({ kind: k.toLowerCase(), ...v }))(Object.entries(st)[0]);
  const linkDown = st => ['offline', 'disconnected'].includes(linkOf(st).kind);
  function linkSay(st) { const l = linkOf(st); return l.kind === 'connected' ? l.rtt_ms + ' ms' : l.kind === 'degraded' ? 'slow · ' + l.rtt_ms + ' ms' : l.kind === 'disconnected' ? l.reason : l.kind === 'connecting' ? 'connecting…' : 'offline'; }
  function linkDot(st, text) { const d = el('span', { class: 'link ' + linkOf(st).kind, title: linkSay(st) }, el('i')); if (text !== false) d.append(el('span', {}, linkSay(st))); return d; }

  // Perf overlay (⌘⌥F, runtime.md meter): the hud panel. A sparkline of the last 120 frames, each bar stacked by phase (drain,
  // layout, draw, encode on the CPU, present wait above), a dashed budget line at the display interval, the phase legend doubling
  // as the table. A hitch flashes the phase that overran, in the bar and in its legend row. Idle intervals draw as stubs, never
  // as hitches (the meter does not count them). The record is the meter's own numbers; nothing here is measured from the DOM.
  // Record (bbt-rt meter.rs names): { hz, interval_ms, budget_ms, cpu: { p50, p99 }, gpu_ms: mean GPU active time per frame,
  // phases: per-phase p50 over the window, frames: [{ drain, layout, draw, encode, present_wait, hitch?: phase } | { idle: true }],
  // hitch: the latest { phase, ms, frame } or null, late: hitches in the window (meter rule: max(cpu, present_wait) > 1.5 × interval),
  // idle_ms: time since the last frame, or null }.
  const PERF_PHASES = ['drain', 'layout', 'draw', 'encode', 'present_wait'];
  function perfHud(m, opts) {
    opts = opts || {};
    const W = 240, H = 44, k = H / (m.budget_ms * 1.5), ms = v => v.toFixed(2); // budget at two thirds; a hitch clips at the top
    const spark = el('div', { class: 'perf-spark', style: 'width:' + W + 'px;height:' + H + 'px' });
    for (const f of m.frames) {
      const bar = el('div', { class: 'perf-bar' + (f.idle ? ' idle' : '') + (f.hitch ? ' hitch' : '') });
      if (!f.idle) for (const ph of PERF_PHASES) bar.append(el('i', { class: 'ph-' + ph + (f.hitch === ph ? ' over' : ''), style: 'height:' + (f[ph] * k).toFixed(2) + 'px' }));
      spark.append(bar);
    }
    const budget = window.UI.Path({ size: [W, 1], points: [{ move: [0, 0.5] }, { line: [W, 0.5] }], stroke: { width: 1, color: 'fg', opacity: 0.55, dash: [3, 2] } });
    budget.classList.add('perf-budget'); budget.style.bottom = (m.budget_ms * k).toFixed(2) + 'px';
    spark.append(budget);
    const idle = m.idle_ms != null, ago = v => v < 1000 ? Math.round(v) + ' ms ago' : (v / 1000).toFixed(1) + ' s ago', label = ph => ph.replace('_', ' ');
    const head = el('div', { class: 'perf-head' }, el('b', {}, idle ? 'idle' : m.hz + ' Hz'), el('span', {}, idle ? 'last frame ' + ago(m.idle_ms) : ms(m.interval_ms) + ' ms · budget ' + ms(m.budget_ms)));
    const over = m.hitch && m.hitch.phase;
    const legend = el('div', { class: 'perf-legend' }, ...PERF_PHASES.map(ph => el('div', { class: 'perf-ph' + (ph === over ? ' over' : '') }, el('i', { class: 'ph-' + ph }), el('span', {}, label(ph)), el('b', {}, ms(ph === over ? m.hitch.ms : m.phases[ph])))));
    const foot = el('div', { class: 'perf-foot' }, el('span', {}, 'CPU ' + ms(m.cpu.p50) + ' · p99 ' + ms(m.cpu.p99) + ' ms'), el('span', {}, 'GPU ' + ms(m.gpu_ms) + ' ms'),
      el('span', { class: m.hitch ? 'hit' : '' }, 'late ' + m.late + (m.hitch ? ' · ' + label(m.hitch.phase) + ' ' + ms(m.hitch.ms) + ' ms · #' + m.hitch.frame : ' / ' + m.frames.length)));
    const card = window.UI.Box('hud', head, spark, legend, foot);
    card.classList.add('perf-hud');
    return card;
  }

  // ⌘T picker: vertical, keyboard-first, typed prefix, name reveal with reroll
  // Launchables: harnesses and zsh today; browser tab, vnc and ssh later. Rows are data, the picker does not care.
  const LAUNCH = [
    // role: the default label (decision #262, AGENTS.md): Claude oracle, Codex executor and stays one, a shell is shell (no prompt, no chip)
    { id: 'claude', label: 'Claude', sub: 'oracle by default', harness: 'claude', role: 'oracle', kind: 'Harness', tile: '#D97757', section: 'Suggestions' },
    { id: 'codex', label: 'Codex', sub: 'executor · never an oracle', harness: 'codex', role: 'executor', kind: 'Harness', tile: '#1F2937', section: 'Suggestions' },
    { id: 'pi', label: 'pi', sub: 'executor by default', harness: 'pi', role: 'executor', kind: 'Harness', tile: '#7C3AED', section: 'Harnesses' },
    { id: 'omp', label: 'omp', sub: 'executor by default', harness: 'omp', role: 'executor', kind: 'Harness', tile: '#0EA5E9', section: 'Harnesses' },
    { id: 'zsh', label: 'zsh', sub: '~ · shell, no role', harness: 'zsh', role: 'shell', kind: 'Shell', tile: '#111827', section: 'Shells' },
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
  // Split view: a binary tree of seats, each split a flex row (v) or column (h) with the gutter as a divider between its halves.
  // opts.leaf(seat) draws a leaf (default: pane); opts.onDivider(div, node, box, A, B) attaches resize behaviour (the showcase's drag).
  function splitView(t, opts) {
    opts = opts || {};
    // a leaf is a borderless box around its pane, so a leaf and a subtree share the extent by ratio alone, as the native split solver
    // does (ui-model layout/split.rs); the box carries the pane's alive rim (the showcase draws its own and passes opts.leaf)
    if (t.key) {
      if (opts.leaf) return opts.leaf(t);
      const po = opts.paneOpts ? opts.paneOpts(t) : {}, w = el('div', { class: 'split-leaf pane-wrap' }, pane(t, po));
      if (window.Glow && !document.body.classList.contains('live')) requestAnimationFrame(() => window.Glow.attach(w, t, po));
      return w;
    }
    const A = splitView(t.a, opts), B = splitView(t.b, opts);
    A.style.flex = t.ratio || 0.5; B.style.flex = 1 - (t.ratio || 0.5);
    const box = el('div', { class: 'split ' + t.dir }), div = el('div', { class: 'divider' + (t.active ? ' active' : '') });
    if (opts.onDivider) opts.onDivider(div, t, box, A, B);
    box.append(A, div, B);
    return box;
  }
  // ⌘T launcher items (design system › modal): launchables ▸ placements; SSH ▸ machine ▸ harness ▸ placement; Cloud ▸ region ▸
  // harness ▸ placement; Go to ▸ every seat and swarm; Graph; Settings ▸ section. Shared by the showcase and launcher.html.
  function launcherItems(ctx) {
    const { seats, spaces, sp, anchorSeat, tabOf } = ctx;
    // rows with sublists: each launchable → placements; 'Go to' → every seat and swarm. 'goto <name>' types straight in.
    const tileOf = h => (LAUNCH.find(l => l.harness === h) || {}).tile || '#374151';
    const goSeats = () => seats.filter(x => !x.settings && !x.graph).map(x => { const t = tabOf(x.key); return { id: 'seat:' + x.key, label: x.name, sub: (t ? t.sp.name + ' · ' : '') + (x.title || '') + (x.where && x.where !== 'local' ? ' · ' + x.where + (x.host ? ' ' + x.host : '') : ''), harness: x.harness, kind: 'Seat', tile: tileOf(x.harness), section: 'Seats', seat: x, tags: [x.harness, x.where || 'local', x.host || '', x.role || '', x.state, x.grade || '', x.state === 'waiting' ? (x.waitKind || 'question') : '', t ? t.sp.name : '', x.muted ? 'muted' : '', ...((x.tasks || []).map(t => t.title))].filter(Boolean) }; });
    const goSwarms = () => spaces.map(x => ({ id: 'swarm:' + x.key, label: x.name, sub: x.tabs.reduce((n, t) => n + t.seats.length, 0) + ' seats', harness: 'sidebarfoxopen', kind: 'Swarm', tile: x.color, section: 'Swarms', swarm: x, tags: ['swarm'] }));
    const placements = (l, role) => { role = role || l.role; const pre = role === l.role ? l.id : l.id + ':as:' + role; return [
      { id: pre + ':tab', label: 'New tab', sub: 'in ' + sp.name, harness: l.harness, tile: l.tile, kind: 'Placement', place: 'tab', launch: l, role },
      ...(anchorSeat ? [{ id: pre + ':right', label: 'Split right', sub: 'of ' + anchorSeat.name, harness: l.harness, tile: l.tile, kind: 'Placement', place: 'right', launch: l, role },
                        { id: pre + ':bottom', label: 'Split down', sub: 'of ' + anchorSeat.name, harness: l.harness, tile: l.tile, kind: 'Placement', place: 'bottom', launch: l, role }] : []),
    ]; };
    // role presets (decision #262): a Claude seat chooses its role at spawn (oracle by default, architect, creativity, executor by #18);
    // Codex and shells have none (Codex implements; a shell has no harness to prompt), so → goes straight to placements. When the
    // swarm already has an Oracle the default stays oracle and the launcher says so, information rather than a block
    const oracle = seats.find(x => x.role === 'oracle' && x.harness === 'claude' && (tabOf(x.key) || {}).sp === sp);
    const NOTE = oracle ? 'this swarm already has an Oracle: ' + oracle.name : null;
    const PRESETS = [['oracle', 'Oracle', 'supervises, briefs, verifies'], ['architect', 'Architect', 'specs, invariants, decisions'], ['creativity', 'Creativity', 'diverges before the pick'], ['executor', 'Executor', 'implements a scoped brief']];
    const presets = l => PRESETS.map(([role, label, say]) => ({ id: l.id + ':as:' + role, label, sub: role === l.role ? 'default · ' + (NOTE || say) : say, harness: l.harness, tile: l.tile, kind: 'Role', role, roleMark: role, launch: l, enterPicks: true, aliases: [role], children: () => placements(l, role) }));
    const launchRows = LAUNCH.map(l => ({ ...l, ...(l.harness === 'claude' && NOTE ? { sub: l.sub + ' · ' + NOTE } : {}), enterPicks: true, children: () => placements(l), aliases: [l.harness] }));   // a Claude seat's role is a spawn parameter now (launch-defaults), Settings' default first
    // remote: SSH ▸ harness ▸ placement, Cloud ▸ harness ▸ placement. 'ssh codex' and 'cloud claude' type straight through.
    // SSH ▸ machine ▸ harness ▸ placement; Cloud ▸ region ▸ harness ▸ placement. Machines come from ~/.ssh/config in the
    // real app; here a demo list. 'ssh rig codex' types straight through all three levels.
    const HOSTS = { ssh: [['rig', 'rig.local · M2 Ultra', { Connected: { rtt_ms: 4 } }], ['alpha', 'alpha.tail · Linux x86', { Degraded: { rtt_ms: 380 } }], ['studio', 'studio.local · M1 Max', 'Offline']],
      cloud: [['eu-2', 'Hetzner · Falkenstein', { Connected: { rtt_ms: 31 } }], ['us-1', 'Fly · Ashburn', 'Connecting'], ['gpu-a', 'Lambda · A100', { Disconnected: { reason: 'link dropped' } }]] };
    const harnessesAt = (where, host) => LAUNCH.map(l => ({ ...l, id: where + ':' + host + ':' + l.id, sub: l.sub + ' · ' + host, enterPicks: true, aliases: [l.harness], where, host, children: () => placements(l).map(pl => ({ ...pl, where, host })) }));
    const remoteRow = (id, label, where, tile) => ({ id, label, aliases: [where], sub: where === 'cloud' ? 'pick a region, then a harness' : 'pick a machine, then a harness', harness: where, kind: 'Remote', tile, section: 'Remote',
      children: () => HOSTS[where].map(([host, desc, link]) => ({ id: where + ':' + host, label: host, sub: desc, harness: where, kind: where === 'cloud' ? 'Region' : 'Machine', tile, where, host, link, down: linkDown(link), children: linkDown(link) ? null : () => harnessesAt(where, host) })) });
    const remoteRows = [remoteRow('ssh', 'SSH', 'ssh', '#374151'), remoteRow('cloud', 'Cloud', 'cloud', '#2E6BFF')];
    const goRow = { id: 'goto', label: 'Go to', aliases: ['goto', 'go', 'jump'], sub: 'a seat or a swarm', harness: 'sidebarfoxopen', kind: 'Navigate', tile: '#6B7190', section: 'Navigate', children: () => [...goSeats(), ...goSwarms()] };
    const settingsRow = { id: 'settings', label: 'Settings', aliases: ['prefs', 'preferences', 'config'], sub: 'account · cloud · machines · roles · harnesses · appearance', harness: 'gear', kind: 'Open', tile: '#6B7190', section: 'Navigate', settingsOpen: true, enterPicks: true, children: () => ['Account', 'Cloud', 'Machines', 'Roles', 'Harnesses', 'Appearance', 'Shortcuts'].map(sct => ({ id: 'settings:' + sct.toLowerCase(), label: sct, harness: 'gear', kind: 'Section', tile: '#6B7190', settingsOpen: true, section: sct })) };
    const graphRow = { id: 'graph', label: 'Graph', aliases: ['who', 'status', 'board'], sub: 'who is on what · epics → work', harness: 'sidebarfoxopen', kind: 'Open', tile: '#6B7190', section: 'Navigate', graphOpen: true, enterPicks: true };
    // Resume (mux-daemon closed_retained): seats past their grace keep history and a harness-session mapping; resuming starts a
    // new process generation through the adapter's resume policy and counts toward the spawn cap. zsh has no resume: a new shell.
    const RESUME = { claude: id => 'claude --resume ' + id, codex: id => 'codex resume ' + id };
    const retained = ctx.retained || [];
    const resumeRow = retained.length ? [{ id: 'resume', label: 'Resume', aliases: ['reopen', 'recent', 'closed'], sub: retained.length + ' closed seat' + (retained.length === 1 ? '' : 's') + ' with history', harness: 'archive', kind: 'Resume', tile: '#6B7190', section: 'Resume',
      children: () => retained.map(r => ({ id: 'resume:' + r.seat.key, label: r.seat.name, sub: 'closed ' + r.closedAt + ' · ' + (RESUME[r.seat.harness] ? RESUME[r.seat.harness](r.session) : 'no resume: a new shell, history replayed above it'), harness: r.seat.harness, tile: tileOf(r.seat.harness), kind: 'Seat', resume: { ...r, cmd: RESUME[r.seat.harness] ? RESUME[r.seat.harness](r.session) : null }, enterPicks: true })) }] : [];
    // with a placement preset (⌘D, ⌘⇧D) the split is already chosen: placement sublists drop out, so the harness row is the launch
    // row and → on it opens its defaults (launch-defaults: split → codex → ↵ defaults, → edit)
    const kidsOf = r => typeof r.children === 'function' ? r.children() : (r.children || []);
    const strip = r => { if (!r.children) return r; const k = kidsOf(r); if (k.length && k.every(x => x.kind === 'Placement')) { const { children, ...rest } = r; return rest; } return { ...r, children: () => k.map(strip) }; };
    const out = [...launchRows, ...remoteRows, ...resumeRow, goRow, graphRow, settingsRow];
    return ctx.preset ? out.map(strip) : out;
  }

  // Launch defaults (spec launch-defaults): a launch is the operator's manual sequence written down, transport → connect → cd →
  // command, prefilled in layers (this launch > machine > swarm > harness > built-in). Each field names the layer it came from and
  // has a home layer that ⌘S writes to. Transports are the extension point: ssh, mosh and et are built in; a plugin adds its own.
  const TRANSPORTS = {
    ssh: v => 'ssh -t ' + (v.port && v.port !== '22' ? '-p ' + v.port + ' ' : '') + (v.key ? '-i ' + v.key + ' ' : '') + v.target + " '" + v.inner + "'",
    mosh: v => 'mosh ' + (v.port && v.port !== '22' ? "--ssh='ssh -p " + v.port + "' " : '') + v.target + " -- sh -c '" + v.inner + "'",
    et: v => 'et ' + v.target + (v.port && v.port !== '22' ? ':' + v.port : '') + " -c '" + v.inner + "'",
    cloud: v => 'bboxy450 cloud ' + v.target + " -- sh -c '" + v.inner + "'",
  };
  const HOME = { cwd: 'swarm', env: 'swarm', cmd: 'harness', args: 'harness', via: 'machine', target: 'machine', port: 'machine', key: 'machine', rcwd: 'machine' };
  const ROLE_CHOICES = [['oracle', 'supervises, briefs, verifies'], ['architect', 'specs, invariants, decisions'], ['creativity', 'diverges before the pick'], ['executor', 'implements a scoped brief']];
  function launchFields(it, path, swarm) {
    const at = [...(path || []), it], l = it.launch || it, h = l.harness;
    const where = it.where || (at.find(x => x.where) || {}).where || 'local', host = it.host || (at.find(x => x.host) || {}).host;
    const hr = SET.harness[h] || {}, hd = hr.defaults || {}, sd = (swarm && swarm.defaults) || {}, md = (host && SET.machineDefaults[host]) || {};
    const m = host && SET.machines.find(x => x[0] === host), sw = swarm ? 'swarm ' + swarm.name : 'built-in';
    const f = (key, label, value, from, extra) => Object.assign({ key, label, value: value == null ? '' : String(value), from, home: HOME[key] }, extra || {});
    const out = [];
    // role: prefilled from Settings › Harnesses; Claude chooses one, Codex and other harnesses are executors (decision #262)
    if (h !== 'zsh') out.push(h === 'claude' ? f('role', 'Role', it.role || hr.role, 'Settings', { choices: ROLE_CHOICES.map(([v, say]) => ({ value: v, label: v[0].toUpperCase() + v.slice(1), say })) }) : f('role', 'Role', it.role || hr.role || 'executor', 'Settings', { fixed: true }));
    if (where !== 'local') {
      out.push(f('via', 'Via', md.via || (where === 'cloud' ? 'cloud' : 'ssh'), md.via ? host : 'built-in', { options: where === 'cloud' ? ['cloud'] : ['ssh', 'mosh', 'et'] }));
      out.push(f('target', 'Connect', md.target || (m ? m[1] : host), md.target ? 'last choice' : host, { choices: [...new Set([md.target || (m ? m[1] : host), ...(md.routes || [])])].map(v => ({ value: v, say: 'route' })) }));
      if (where !== 'cloud') { out.push(f('port', 'Port', md.port || '22', md.port ? host : 'built-in')); out.push(f('key', 'Key', md.key || (m ? m[3] : ''), host)); }
      out.push(f('rcwd', 'Directory', md.cwd || '~', md.cwd ? 'last choice' : 'built-in', { choices: [...new Set([md.cwd || '~', ...(md.recent || [])])].map(v => ({ value: v })) }));
    } else out.push(f('cwd', 'Directory', sd.cwd || '~', sd.cwd ? 'last choice · ' + sw : 'built-in', { choices: [...new Set([sd.cwd || '~', ...((swarm && swarm.recent) || []), '~'])].map(v => ({ value: v })) }));
    out.push(f('cmd', 'Command', hd.cmd || hr.command || h, h));
    out.push(f('args', 'Arguments', hd.args || '', hd.args ? h : 'built-in'));
    out.push(f('env', 'Environment', sd.env || '', sd.env ? sw : 'built-in', { hint: 'KEY=value …' }));
    return out;
  }
  function composeLaunch(fields) {
    const v = Object.fromEntries(fields.map(x => [x.key, x.value.trim()]));
    v.inner = [v.cwd || v.rcwd ? 'cd ' + (v.cwd || v.rcwd) : '', [v.env, v.cmd, v.args].filter(Boolean).join(' ')].filter(Boolean).join(' && ');
    return v.via ? (TRANSPORTS[v.via] || TRANSPORTS.ssh)(v) : v.inner;
  }
  // Text-field editing state (Input and the launcher query), deterministic so parity can capture it. Public inputs:
  // edit = { caret: index } | { select: [from, to] } | { caret: index, preedit: { text, caret } }; an edit object means focused.
  // Every index is a UTF-16 code-unit offset (a JS string index), never inside a surrogate pair; the fixtures are ASCII.
  // The real <input> keeps its box (its own text goes transparent), so nothing moves; .edit-field draws the runs over it:
  // text, the selection band, IME marked text (underlined) and a 2 px accent caret centred on its advance, as tall as the line.
  // The caret is steady unless edit.blink (reference states are snapshots). A selection hides the caret (macOS). Focused and
  // empty: the caret sits at 0, before the dimmed placeholder.
  // the runs of one edit state over a value (shared by editField and the composer): text, selection band, IME, caret, placeholder
  function editRuns(v, edit, placeholder) {
    const run = (cls, t) => el('span', { class: cls }, t), caret = () => el('i', { class: 'ed-caret' });
    if (edit.select) { const [a, b] = edit.select; return [run('ed-t', v.slice(0, a)), run('ed-sel', v.slice(a, b)), run('ed-t', v.slice(b))]; }
    const at = Math.max(0, Math.min(v.length, edit.caret != null ? edit.caret : v.length)), pe = edit.preedit, out = [run('ed-t', v.slice(0, at))];
    if (pe) { const k = pe.caret != null ? pe.caret : pe.text.length; out.push(el('span', { class: 'ed-ime' }, pe.text.slice(0, k), caret(), pe.text.slice(k))); }
    else out.push(caret());
    out.push(run('ed-t', v.slice(at)));
    if (!v && !pe && placeholder) out.push(run('ed-ph', placeholder));
    return out;
  }
  function editField(input, edit) {
    const f = el('span', { class: 'edit-field' + (edit.blink ? ' blink' : ''), 'aria-hidden': 'true' }, ...editRuns(input.value, edit, input.placeholder));
    input.tabIndex = -1; input.readOnly = true;
    return el('span', { class: 'ui-edit' }, input, f);
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
    // opts.usedNames (the swarm's seat names): the product pool, reroll steps to the next unused name; without it a demo roll
    function nameFor(seed) { return opts.usedNames ? poolName(opts.usedNames, seed - 1) : rollName(rng('roll' + seed)); }
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
    const climb = () => { if (!stack.length) return; if (stack[stack.length - 1].params) params = null; stack = stack.slice(0, -1); q = ''; input.value = ''; sel = 0; draw(); };
    // spawn cap (launcher_cap_notice): launch rows dim and refuse; Go to, Graph and Settings keep working
    const isNav = it => !!(it.seat || it.swarm || it.graphOpen || it.settingsOpen || it.section === 'Navigate' || it.section === 'Swarms' || it.kind === 'Navigate' || it.kind === 'Open');
    const capped = it => !!opts.cap && !isNav(it);
    const flashCap = () => { const b = list.querySelector('.capnote'); if (b) { b.classList.remove('flash'); void b.offsetWidth; b.classList.add('flash'); } };
    // capped: refuse anything that would launch (a leaf, or a parent whose Enter picks); → still dives into a parent
    // Spawn parameters (spec launch-defaults): ↵ on a harness launches it as is; → opens its parameters, one row each, every row
    // prefilled (role from Settings, the rest with the last choice). ↵ on any parameter launches with all of them; → or a click
    // changes one: a list when it has choices (roles, recent directories, transports, routes; typing a new value and ↵ takes it),
    // else typed in place. The last choices are remembered on launch by the page, at each parameter's home.
    const launchRow = it => !!it && !it.children && !it.down && !isNav(it) && !it.resume && !it.opt && it.val === undefined && !!(it.harness || it.launch) && it.kind !== 'Remote';
    let params = null, editing = null;
    const mods = ev => ({ meta: !!(ev && (ev.metaKey || ev.ctrlKey)), shift: !!(ev && ev.shiftKey), alt: !!(ev && ev.altKey) });
    const paramRows = () => params.fields.map(f => ({ id: 'opt:' + f.key, opt: f, label: f.label, sub: f.value || '—', kind: f.fixed ? 'fixed' : f.from }));
    const openParams = it => { params = { it, base: path(), fields: (opts.fieldsFor || launchFields)(it, path(), opts.swarm) }; stack = [...path(), { id: it.id + ':params', label: it.label, params: true, children: paramRows }]; q = ''; input.value = ''; sel = 0; draw(); };
    const editOpt = f => {
      if (f.fixed) return;
      const choices = f.choices || (f.options ? f.options.map(v => ({ value: v })) : null);
      if (choices) return dive({ id: 'vals:' + f.key, label: f.label, valsOf: f, children: () => choices.map(c => ({ id: 'val:' + c.value, val: c.value, field: f, label: c.label || c.value, sub: [c.value === f.value ? 'current' : '', c.say || ''].filter(Boolean).join(' · ') })) });
      editing = f; input.value = f.value; input.placeholder = f.label + ' · ↵ sets it · esc keeps the old value'; draw(); input.focus(); input.select();
    };
    const setVal = (f, v) => { f.value = v; f.from = 'this launch'; };
    const launchWith = ev => { if (capped(params.it)) return flashCap(); const fs = params.fields; if (opts.onPick) opts.onPick(params.it, nameFor(seed), { ...mods(ev), overrides: Object.fromEntries(fs.map(x => [x.key, x.value])), command: composeLaunch(fs) }, params.base); };
    const pick = (it, ev) => {
      if (it && it.val !== undefined) { setVal(it.field, it.val); return climb(); }
      if (it && it.opt) return launchWith(ev);
      if (!it || it.down) return; if (capped(it) && (!it.children || it.enterPicks)) return flashCap(); if (it.children && !it.enterPicks) return dive(it); if (opts.onPick) opts.onPick(it, nameFor(seed), { meta: !!(ev && (ev.metaKey || ev.ctrlKey)), shift: !!(ev && ev.shiftKey), alt: !!(ev && ev.altKey) }, path()); };
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
        const o = el('div', { class: 'opt' + (i === sel ? ' sel' : '') + (it.children ? ' parent' : '') + (capped(it) ? ' capped' : '') + (it.down ? ' down' : ''), onclick: e => it.opt ? editOpt(it.opt) : pick(it, e), onpointerenter: () => { if (sel !== i) { sel = i; draw(); } } }, el('span', { class: 'tile', style: 'background:' + (it.tile || 'var(--surface-2)') }, harnessIcon(it.harness)));
        // plain text in every row; the only emphasis is the selection band (operator, Raycast reference)
        const label = el('span', { class: 'lbl' }, it.label);
        o.append(label, el('span', { class: 'sub' }, it.sub || ''), it.link ? el('span', { class: 'kind' }, linkDot(it.link)) : el('span', { class: 'kind' }, it.kind || it.hint || ''));
        o.append(it.children || (it.opt && !it.opt.fixed) ? el('span', { class: 'chev', title: it.opt ? '→ changes it' : '→ opens' }, '›') : el('span', { class: 'num' }, String(i + 1)));
        list.append(o);
      });
      if (params && stack.length && stack[stack.length - 1].params) list.append(el('div', { class: 'cmdrow' }, el('span', { class: 'k' }, 'Runs'), el('code', { class: 'cmdline' }, composeLaunch(params.fields))));
      drawFoot(params && stack.length && stack[stack.length - 1].params ? [['Launch', '↵'], ['Change', '→'], ['Back', '←']] : editing ? [['Set', '↵'], ['Keep', 'esc']] : null);
    }
    function drawFoot(over) {
      const nm = nameFor(seed);
      foot.innerHTML = '';
      foot.append(el('span', { class: 'sw', style: 'background:' + seatColor({ key: nm }), title: 'seat colour' }), el('span', { class: 'nm' }, nm), el('button', { onclick: () => { seed++; draw(); }, title: 'reroll name' }, '↻'), (() => { const pill = el('span', { class: 'pill' }); const acts = over || (typeof opts.actions === 'function' ? opts.actions(path()) : opts.actions) || [['Open', '↵'], ['Into', '→'], ['Back', '←'], ['Reroll', '⌘R']]; acts.forEach(([l, kb], i) => { if (i) pill.append(el('span', { class: 'sep' })); pill.append(el('span', {}, l), el('kbd', {}, kb)); }); return pill; })());
    }
    input.addEventListener('input', () => { if (editing) return; q = input.value; sel = 0; draw(); });
    input.addEventListener('keydown', e => {
      const n = matches().length, cur = matches()[sel];
      if (e.key === 'ArrowDown') { e.preventDefault(); sel = (sel + 1) % Math.max(n, 1); draw(); }
      else if (e.key === 'ArrowUp') { e.preventDefault(); sel = (sel + n - 1) % Math.max(n, 1); draw(); }
      else if (e.key === 'ArrowRight') { if (editing) return; if (cur && cur.children) { e.preventDefault(); dive(cur); } else if (cur && cur.opt) { e.preventDefault(); editOpt(cur.opt); } else if (launchRow(cur)) { e.preventDefault(); openParams(cur); } }
      else if (e.key === 'ArrowLeft') { if (!q && stack.length) { e.preventDefault(); climb(); } }
      else if (e.key === 'Backspace' && !q && stack.length) { e.preventDefault(); climb(); }
      else if (e.key === 'Enter') { e.preventDefault(); const top = stack[stack.length - 1];
        if (editing) { setVal(editing, input.value); editing = null; input.value = ''; input.placeholder = opts.placeholder || ''; draw(); }
        else if (!cur && top && top.valsOf && q.trim()) { setVal(top.valsOf, q.trim()); climb(); }
        else pick(cur, e); }
      else if (e.key === 'Escape') { if (editing) { editing = null; input.value = ''; input.placeholder = opts.placeholder || ''; draw(); } else if (stack.length) climb(); else if (opts.onClose) opts.onClose(); }
      else if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'r') { e.preventDefault(); seed++; draw(); }
      else if (/^[1-9]$/.test(e.key) && !q) { e.preventDefault(); pick(matches()[+e.key - 1], e); }
      e.stopPropagation();
    });
    draw();
    const inrow = el('div', { class: 'inrow' + (opts.edit ? ' editing' : '') }, crumbs, opts.edit ? editField(input, opts.edit) : input);
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
  // StreamDock composer (messaging.md): one bar at the dock foot. Picked shape: address chips (seat, role, #channel, ↳ subagent)
  // that typed @ # ↳ tokens turn into, defaulting to the dock's current tab; SendRequest's numeric priority (0-9) as a pip on the
  // send button, whose p0 colour is the operator's class (decision #38a: the operator is Oracle class), and its kind as a chip when
  // not text; the text's caret, selection and IME are the same deterministic edit state as Input (st.edit, UTF-16 offsets).
  // Artifacts and tasks attach as cards above the text. Delegate mode is the same bar with a task attached: the verb
  // becomes Delegate and the address picker lists seats by role with their load. Nothing is sent without its evidence ladder.
  const ADDR_MARK = { seat: '', role: '', channel: '#', subagent: '↳ ' };
  const addrChip = a => el('span', { class: 'addr ' + a.kind, style: a.color ? '--c:' + a.color : '', title: a.kind + (a.n ? ' · fans out to ' + a.n + ' seats' : '') + (a.parent ? ' · inside ' + a.parent : '') },
    a.kind === 'seat' ? el('i', { class: 'dot' }) : null, a.kind === 'role' ? el('span', { class: 'rm' }, ROLE_MARK[a.role] || '') : null, (ADDR_MARK[a.kind] || '') + (a.parent ? a.parent + ' ▸ ' : '') + a.label, a.n ? el('small', {}, a.n) : null, el('span', { class: 'x' }, '×'));
  const attachCard = a => el('span', { class: 'att ' + a.kind, title: a.kind + ' · ' + a.title }, el('span', { class: 'ak' }, a.kind === 'task' ? taskMark(a.status) : ART_GLYPH[a.akind] || '▢'), el('span', { class: 'at' }, a.title), a.status ? el('small', {}, a.status) : a.meta ? el('small', {}, a.meta) : null);
  // one status line for a request's outcome (composer sends, task rejects): pending spins, sent ✓, held ◷ amber, refused ⊘ red
  const noticeLine = n => el('div', { class: 'cnote ' + n.kind, role: 'status' }, el('span', { class: 'cnm' }, { pending: el('i', { class: 'spin' }), sent: '✓', held: '◷', refused: '⊘' }[n.kind]), el('span', { class: 'cnt' }, n.text));
  // [[ attach picker (st.attachPick { query, edit, items, sel }): the swarm's tasks and artifacts, filtered by the typed query on the
  // title, one row each (mark, title, status or meta, kind), the selection band like the launcher's; ↵ attaches, ↑↓ moves, esc closes
  function attachPicker(ap) {
    const q = (ap.query || '').toLowerCase(), rows = (ap.items || []).filter(it => !q || it.title.toLowerCase().includes(q)), sel = Math.min(ap.sel || 0, Math.max(0, rows.length - 1));
    const head = el('div', { class: 'aphead' }, el('span', { class: 'apq' }, '[['), el('span', { class: 'apin edit-text' }, ...editRuns(ap.query || '', ap.edit || {}, 'a task or artifact')));
    const list = el('div', { class: 'aplist', role: 'listbox' }, ...(rows.length ? rows.map((it, i) => el('div', { class: 'apopt' + (i === sel ? ' sel' : ''), role: 'option', 'aria-selected': String(i === sel) },
      el('span', { class: 'ak' }, it.kind === 'task' ? taskMark(it.status) : ART_GLYPH[it.akind] || '▢'), el('span', { class: 'apt' }, it.title), el('span', { class: 'aps' }, it.kind === 'task' ? it.status : it.meta || ''), el('span', { class: 'apk' }, it.kind))) : [el('div', { class: 'apempty' }, 'nothing matches "' + ap.query + '"')]));
    return el('div', { class: 'apick' }, head, list, el('div', { class: 'apfoot' }, '↵ attach · ↑↓ · esc'));
  }
  // record lookup ({ query, edit, sel }; not ⌘F, which searches inside the open document): '#' then a number (prefix of the record number) or words (in the title), over RECORDS; one row
  // each (#N, title, file name, type), the launcher's selection band; ↵ opens it as a document, ↑↓ moves, esc closes
  function recordLookup(st) {
    const q = (st.query || '').trim().toLowerCase(), byNum = /^\d+$/.test(q);
    const rows = [...RECORDS.values()].filter(r => !q || (byNum ? String(r.n).startsWith(q) : r.title.toLowerCase().includes(q))).sort((a, b) => a.n - b.n), sel = Math.min(st.sel || 0, Math.max(0, rows.length - 1));
    const head = el('div', { class: 'aphead' }, el('span', { class: 'apq' }, '#'), el('span', { class: 'apin edit-text' }, ...editRuns(st.query || '', st.edit || {}, 'a number or words from the title')));
    const list = el('div', { class: 'aplist', role: 'listbox' }, ...(rows.length ? rows.map((r, i) => el('div', { class: 'apopt' + (i === sel ? ' sel' : ''), role: 'option', 'aria-selected': String(i === sel) },
      el('span', { class: 'ak rn' }, '#' + r.n), el('span', { class: 'apt' }, r.title), el('span', { class: 'aps', title: r.path || '' }, (r.path || '').split('/').pop()), el('span', { class: 'apk' }, r.type))) : [el('div', { class: 'apempty' }, 'no record matches "' + st.query + '"')]));
    return el('div', { class: 'apick rlook' }, head, list, el('div', { class: 'apfoot' }, '↵ open · ↑↓ · esc'));
  }
  function composer(st, opts) {
    opts = opts || {};
    const mode = st.mode || 'message', to = st.to || [], att = st.attach || [], priority = st.priority != null ? st.priority : 0, kind = st.kind || 'text';
    const c = el('div', { class: 'composer ' + mode + (st.focus ? ' focus' : '') });
    // the last send's outcome, one line (st.notice { kind: pending | sent | held | refused, text }): pending keeps the text, sent and
    // held clear it, refused keeps it so the operator can fix and resend; held and refused name their reason, sent names the stored id
    if (st.notice) c.append(noticeLine(st.notice));
    if (st.attachPick) c.append(attachPicker(st.attachPick));
    if (st.pick) c.append(el('div', { class: 'cpick', role: 'listbox' }, ...st.pick.map(g => el('div', { class: 'cgroup' }, el('div', { class: 'cgh' }, g.title),
      ...g.items.map((it, i) => el('div', { class: 'copt' + (it.sel ? ' sel' : '') }, it.kind === 'seat' ? el('i', { class: 'dot', style: 'background:' + it.color }) : el('span', { class: 'ok' }, it.kind === 'role' ? ROLE_MARK[it.role] : it.kind === 'channel' ? '#' : '↳'),
        el('span', { class: 'ol' }, it.label), it.sub ? el('small', {}, it.sub) : null, it.load != null ? el('span', { class: 'load' + (it.load ? '' : ' free') }, it.load ? it.load + ' running' : 'free') : null))))));
    c.append(el('div', { class: 'cto' }, el('span', { class: 'cl' }, mode === 'delegate' ? 'Delegate to' : 'To'), ...to.map(addrChip), el('span', { class: 'ctyped' }, st.typed || '')));
    if (att.length) c.append(el('div', { class: 'catt' }, ...att.map(attachCard)));
    const ph = mode === 'delegate' ? 'Brief, or leave empty to send the task as written' : 'Message ' + (to[0] ? (ADDR_MARK[to[0].kind] || '') + to[0].label : 'the swarm');
    c.append(el('div', { class: 'ctext edit-text' + (st.text ? '' : ' empty') }, ...(st.focus ? editRuns(st.text || '', st.edit || {}, ph) : [st.text || ph])));
    c.append(el('div', { class: 'cfoot' }, el('button', { class: 'cbtn', title: 'attach an artifact or task ([[)' }, harnessIcon('archive')), el('span', { class: 'chint' }, '[[ attach · @ # ↳ address'), el('span', { class: 'spacer' }),
      // the kind is always visible and is the chooser: text is the default and dim; st.kindMenu opens the three sendable kinds above it
      el('span', { class: 'kpick' }, st.kindMenu ? el('div', { class: 'kmenu', role: 'menu' }, ...['text', 'chunk', 'decision'].map(k => el('div', { class: 'kopt' + (k === kind ? ' on' : ''), role: 'menuitemradio', 'aria-checked': String(k === kind) },
          el('span', { class: 'kck' }, k === kind ? '✓' : ''), el('span', { class: 'kchip ' + k }, k), el('small', {}, { text: 'a message', chunk: 'progress on the stream', decision: 'a decision on the stream' }[k])))) : null,
        el('button', { class: 'kchip kind ' + kind + (st.kindMenu ? ' open' : ''), title: 'message kind', 'aria-haspopup': 'menu' }, kind, el('span', { class: 'kdn' }, '▾'))),
      el('button', { class: 'send p0', title: 'priority ' + priority + ' (0-9) · the operator is Oracle class, p0 · ⌘↵' }, el('span', { class: 'pip' }, String(priority)), mode === 'delegate' ? 'Delegate' : 'Send', el('span', { class: 'kbd' }, '⌘↵'))));
    return c;
  }
  // the desktop notification (attention §3), the lab's picture of the macOS banner. v1 is title + body only, as attention.rs
  // builds them: title 'name · harness', body the first line of the seat's detail, else by wait kind (NOTIFY_BODY)
  const NOTIFY_BODY = { permission: 'Permission requested', auth: 'Authentication requested', question: 'Your answer is needed' };
  function notification(seat, line, onClick) {
    return el('div', { class: 'notif', role: 'button', tabindex: '0', onclick: onClick }, logo(32, { square: true }),
      el('div', { class: 'nb' }, el('div', { class: 'nt' }, el('b', {}, seat.name + ' · ' + seat.harness), el('span', { class: 'nw' }, 'now')),
        el('div', { class: 'nm' }, line || NOTIFY_BODY[seat.waitKind] || NOTIFY_BODY.question)));
  }
  // report-back card: a TaskReport (bbt-proto, decision #38) as the Executor sent it: summary, the signed commit, and verify_result
  // (the command, its exit code, one evidence line per acceptance criterion). A report is a handoff, not proof; Accept and Re-brief
  // belong to the delegator, and accepting needs verify_result at exit 0 (decision #28). r.task and r.status come from the TaskCard.
  function reportCard(r, opts) {
    const v = r.verify_result, status = r.status || 'reported', ok = v && v.exit_code === 0;
    const card = el('div', { class: 'report ' + status, 'data-rim-key': r.task }, el('div', { class: 'rh' }, el('span', { class: 'rk' }, taskMark(status)), el('b', {}, r.task), el('span', { class: 'rs' }, status)),
      el('div', { class: 'rsum' }, r.summary));
    if (v) card.append(el('div', { class: 'rsec' }, el('div', { class: 'rc ' + (ok ? 'ok' : 'bad') }, el('span', { class: 'ck' }, ok ? '✓' : '✕'), el('code', {}, v.command), el('span', { class: 'ex' }, 'exit ' + v.exit_code)),
      ...(v.evidence || []).map(e => el('div', { class: 'rev' }, el('span', { class: 'rcrit' }, e.criterion), el('code', {}, e.reference)))));
    else card.append(el('div', { class: 'rgap' }, 'no verify_result · cannot be accepted'));
    if (r.commit) card.append(el('div', { class: 'rcommit' }, el('code', {}, r.commit)));   // a reference only: the report carries no signature proof
    // review: true grants Accept / Re-brief as data, so the capture carries it (callbacks do not survive JSON)
    // reject needs a real reason (TaskRequest::Reject { reason }): r.reject { reason, edit, notice } opens it inline; Reject stays
    // disabled until the reason is non-empty; the reason is what the seat receives, so it reads as the re-brief
    if (status === 'reported' && opts && (opts.review || opts.onAccept) && r.reject) {
      const rj = r.reject, has = !!(rj.reason || '').trim(), inp = el('input', { value: rj.reason || '', placeholder: 'what to change · sent to the seat', spellcheck: 'false' });
      card.append(el('div', { class: 'rrej' }, el('div', { class: 'rjl' }, 'Reject · reason ', el('small', {}, 'required')), el('label', { class: 'ui-input rjin' + (rj.edit ? ' st-focus' : '') }, rj.edit ? editField(inp, rj.edit) : inp),
        rj.notice ? noticeLine(rj.notice) : null,
        el('div', { class: 'racts' }, el('button', { class: 'rebrief' }, 'Cancel'), el('button', { class: 'reject', ...(has ? {} : { disabled: '', title: 'a reason is required' }) }, 'Reject'))));
    } else if (status === 'reported' && opts && (opts.review || opts.onAccept)) card.append(el('div', { class: 'racts' }, el('button', { class: 'rebrief' }, 'Reject…'), el('button', { class: 'accept', ...(ok ? {} : { disabled: '', title: 'needs verify_result at exit 0 (decision #28)' }), onclick: () => opts.onAccept && opts.onAccept(r) }, 'Accept')));
    return card;
  }
  function messageRow(m, opts) {
    opts = opts || {};
    // MessageRow (decision #38): author { kind, id, name }, issuer_kind, kind, priority_class + priority, text, ts, receipts. role is not
    // in the row: the GUI looks it up from the seat registry by author.id (lab: m.role); an operator author reads OPERATOR.
    if (m.kind === 'presence') return el('div', { class: 'msg presence' }, el('span', { class: 'ptext' }, m.text), el('span', { class: 'time' }, hhmm(m.ts)));
    // audit line (a projection of the Oracle audit log, swarm-member scope): the Oracle's effects in the open, centred like presence
    // routing discipline (messaging.md): a worker's progress note to the stream is rejected with a hint; only chunk, decision, presence land
    if (m.rejected) return el('div', { class: 'msg rejected', title: 'rejected: worker-class stream messages must be chunk, decision or presence' }, el('span', { class: 'rj-t' }, el('span', { class: 'rj-x' }, '⊘ '), el('b', {}, m.author.name), ' tried to post progress to the stream'), el('span', { class: 'rj-hint' }, 'DM your delegator' + (m.delegator ? ' · ' + m.delegator : '')), el('span', { class: 'time' }, hhmm(m.ts)));
    if (m.audit) { const a = m.audit; return el('div', { class: 'msg presence audit ' + (a.outcome || 'applied'), title: 'audit · op ' + (a.op || '') + ' · ' + (a.outcome || 'applied') + (a.reason ? ' · ' + a.reason : '') },
      el('div', { class: 'aline' }, el('span', { class: 'amark' }, ROLE_MARK.oracle + ' '), el('b', {}, a.actor || 'Oracle'), ' ' + a.verb + ' ', el('b', {}, a.target || '')),
      el('div', { class: 'ameta' }, a.op ? el('code', { class: 'aop' }, a.op.slice(0, 6)) : null, a.outcome && a.outcome !== 'applied' ? el('span', { class: 'aout' }, a.outcome) : null, el('span', { class: 'time' }, hhmm(m.ts)))); }
    // a mark only when something real says it: the operator and observers by author kind, a seat by its registry role; never invented
    const op = m.author.kind === 'operator', role = op ? 'oracle' : m.role || null, chip = prioChip(m);
    const mark = op ? 'OPERATOR' : m.author.kind === 'observer' ? 'OBSERVER' : role ? role.toUpperCase() : null;
    const row = el('div', { class: 'msg ' + (role || 'norole') + (op ? ' operator' : '') + (m.unread ? ' unread' : '') + (m.replyTo ? ' nested' : ''), style: role ? '--role:var(--role-' + role + ')' : null });
    const head = el('div', { class: 'head' }, el('span', { class: 'who' }, m.author.name), mark ? el('span', { class: 'role' }, '<' + mark + '>') : null);
    // the chip is the receiver's own (priority_class, priority): a foreign Oracle arrives as worker class here, so it carries none
    if (chip === 'p0') head.append(el('span', { class: 'p0', title: 'Oracle class' }, 'p0'));
    else if (chip === 'p1') head.append(el('span', { class: 'prio p1', title: 'worker class · priority ' + m.priority + ' of 9' }, 'p1'));
    if (m.issuer_kind === 'local-dev') head.append(el('span', { class: 'kchip issuer', title: 'signed under the local-dev issuer, not the GPG root (decision #4)' }, 'local-dev'));
    if (m.ch && opts.showCh !== false) head.append(el('span', { class: 'chchip', title: 'arrived by bboxy450://channels/bboxy450-term/' + m.ch + '/' }, '#' + m.ch));
    if (m.kind && m.kind !== 'text') head.append(el('span', { class: 'kchip ' + m.kind, title: m.kind + ': allowed on the swarm stream' }, m.kind));
    if (m.group) head.append(el('span', { class: 'agchip grp', title: 'to subgroup ' + m.group.name + ': fanned out as DMs to ' + (m.group.n || '') + ' members, one thread' }, '⧉ ' + m.group.name + (m.group.n ? ' · ' + m.group.n : '')));
    // subagent addressing (messaging.md): to main or to a subagent; from a subagent, or relayed via main; held when the agent ended
    if (m.fromAgent) head.append(el('span', { class: 'agchip from', title: 'from subagent ' + m.fromAgent }, '↳ ' + m.fromAgent));
    if (m.via === 'main') head.append(el('span', { class: 'agchip via', title: 'the harness relayed it; the subagent is not addressable directly' }, 'via main'));
    if (m.to) head.append(el('span', { class: 'agchip to' + (m.to === 'main' ? ' main' : ''), title: m.to === 'main' ? 'to the main agent' : 'to subagent ' + m.to }, '→ ' + (m.to === 'main' ? 'main' : '↳ ' + m.to)));
    head.append(el('span', { class: 'time' }, hhmm(m.ts)));
    if (m.held === 'agent_ended') head.append(el('span', { class: 'agchip held', title: 'held: the subagent ended before delivery; never redirected to main on its own' }, 'held · agent ended'));
    else if (m.receipts) { const r = m.receipts; head.append(el('span', { class: 'rcpt' + (r.held ? ' held' : ''), title: 'receipts on this send: delivered, acked by the harness, held' }, r.delivered + ' delivered · ' + r.acked + ' acked' + (r.held ? ' · ' + r.held + ' held' : ''))); }
    else if (m.evidence) head.append(tickLadder(m.evidence));
    const body = el('div', { class: 'body' }, head);
    if (m.replyTo) body.append(el('div', { class: 'reply' }, '↩ ' + m.replyTo));
    if (m.toLabel) body.append(el('div', { class: 'tol', title: 'to ' + m.toLabel }, '→ ' + m.toLabel));
    body.append(el('div', { class: 'text' }, /\[\[/.test(m.text || '') ? renderLinks(m.text) : m.text));
    if (m.attach && m.attach.length) body.append(el('div', { class: 'attach' }, ...m.attach.map(a => attachCard(a))));
    if (m.report) body.append(reportCard(m.report, opts));
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
    // the fifth field is the host's LinkState (see linkOf)
    cloud: [['Hetzner', 'eu-2 · Falkenstein', true, '#D50C2D', { Connected: { rtt_ms: 31 } }], ['Fly', 'us-1 · Ashburn', true, '#7B3FE4', 'Connecting'], ['Lambda', 'gpu-a · A100', false, '#2563EB', 'Offline']],
    machines: [['rig', 'feral@rig.local', 'M2 Ultra · 64 GB', '~/.ssh/id_ed25519', { Connected: { rtt_ms: 4 } }], ['alpha', 'feral@alpha.tail', 'Linux x86 · 128 GB', '~/.ssh/id_ed25519', { Degraded: { rtt_ms: 380 } }], ['studio', 'feral@studio.local', 'M1 Max · 32 GB', '~/.ssh/id_ed25519', 'Offline']],
    // roles: built-ins are editable, not deletable; custom roles carry custom:true. cls = priority class (delivery order and badging)
    roles: {
      oracle: { prompt: 'You are the Oracle. Supervise, verify, never implement. Sign every message Name <ORACLE>.', send: 'all channels · all DMs', color: 'var(--role-oracle)', mark: '♔', cls: 'oracle' },
      executor: { prompt: 'You are an Executor. Do the assigned work, report evidence, never expand scope.', send: 'own swarm · operator DM', color: 'var(--role-executor)', mark: '♟', cls: 'worker' },
      creativity: { prompt: 'You are Creativity. Diverge before converging; return a field, not a pick.', send: 'own swarm · operator DM', color: 'var(--role-creativity)', mark: '♝', cls: 'worker' },
      architect: { prompt: 'You are the Architect. Design the system and write the specs; decide interfaces and invariants; never implement. Sign Name <ARCHITECT>.', send: 'all channels · all DMs', color: 'var(--role-architect)', mark: '♖', cls: 'oracle' },
    },
    // harnesses: built-ins carry command/tile/icon; custom ones run through the PTY adapter
    // launch defaults per machine (launch-defaults §layers): transport, one route per network, port, key, remote directory
    machineDefaults: { rig: { via: 'ssh', routes: ['feral@rig.local', 'feral@rig.tail'], cwd: '~/src/bboxy450-term' }, alpha: { via: 'mosh', routes: ['feral@alpha.tail'], cwd: '~/work' } },
    harness: {
      claude: { role: 'oracle', wake: 'socket', hooks: true, command: 'claude', tile: LT('claude'), icon: 'claude', stateFrom: 'osc7501' },
      codex: { role: 'executor', wake: 'hook', hooks: true, command: 'codex', defaults: { args: '--sandbox workspace-write' }, tile: LT('codex'), icon: 'codex', stateFrom: 'hooks' },
      pi: { role: 'executor', wake: 'pty', hooks: false, command: 'pi', tile: LT('pi'), icon: 'pi', stateFrom: 'output cadence' },
      omp: { role: 'executor', wake: 'pty', hooks: false, command: 'omp', tile: LT('omp'), icon: 'omp', stateFrom: 'output cadence' },
    },
    // notifications (attention §3, decision #44), in the native field names (attention.rs Notifications): on by default; harnesses
    // listed in disabled_harnesses are silenced; optionally only while the window is not frontmost. A muted seat (⌘M) never notifies.
    notifications: { enabled: true, only_when_not_frontmost: false, disabled_harnesses: [] },
    // plugins (spec plugins §1 Registry, §1a tiers, §1b hook ceilings; e2e G3): every discovered binary with its trust state and effective
    // tier, contract compliance, version, auth, health, enable, and the last 60 s of calls and hooks (wall ms; spans when the shape says
    // trace: true). t is ms before now; a hook names its event. A plugin that provides a knowledge root cannot be disabled.
    plugins: [
      { name: 'bboxy450-term-data_source-bboxy', bundled: true, contract: { name: 'data_source', version: '1.0', ok: true }, tier: 'export', local: true, trust: 'trusted', version: '0.1.0', auth: null, health: { state: 'ok' }, enabled: true, root: '.bboxy',
        calls: [[52000, 'resolve', 3], [44000, 'fetch', 12], [37000, 'search', 41], [30000, 'changes_since', 2], [21000, 'fetch', 9], [12000, 'graph', 28], [4000, 'changes_since', 2]].map(([t, m, ms]) => ({ t, method: m, ms })) },
      { name: 'bboxy450-term-event_sink-otel', contract: { name: 'event_sink', version: '1.0', ok: true }, tier: 'export', trust: 'trusted', grant: 'hook budget 12 000 ms, granted at trust', version: '0.3.2', auth: 'ok', health: { state: 'degraded', say: 'slow hooks' }, enabled: true,
        calls: [[58000, 10120, 'after PostToolUse'], [46000, 9980, 'after PostToolUse'], [33000, 10240, 'after PostToolUse'], [20000, 10050, 'after PostToolUse']].map(([t, ms, hook]) => ({ t, method: 'emit', ms, hook })),
        alerts: ['this plugin takes 10 s on every hook', 'budget at its ceiling: 12 000 ms granted'] },
      { name: 'bboxy450-term-decision_model-jev', contract: { name: 'decision_model', version: '1.0', ok: true }, tier: 'export', trust: 'trusted', version: '1.4.0', auth: 'ok', health: { state: 'ok' }, enabled: true, trace: true,
        calls: [{ t: 48000, method: 'decide', ms: 820 }, { t: 9000, method: 'decide', ms: 640, spans: [{ name: 'decide', start: 0, ms: 640, depth: 0 }, { name: 'encode state', start: 10, ms: 120, depth: 1 }, { name: 'POST jev', start: 140, ms: 430, depth: 1 }, { name: 'tls', start: 140, ms: 60, depth: 2 }, { name: 'model', start: 210, ms: 330, depth: 2 }, { name: 'parse', start: 580, ms: 50, depth: 1 }] }] },
      { name: 'bboxy450-term-task_system-linear', contract: { name: 'task_system', version: '1.0', ok: true }, tier: 'export', trust: 'trusted', version: '0.2.0', auth: 'needs login', health: { state: 'setup', say: 'not logged in' }, enabled: true, calls: [] },
      { name: 'bboxy450-term-data_source-obsidian', contract: { name: 'data_source', version: '1.0', ok: false, failing: 'resolve #N (no numbers in a vault)' }, tier: 'export', trust: 'trusted', version: '0.0.4', auth: null, health: { state: 'crashed', say: 'exited 139 at 14:02 · the app is fine' }, enabled: false, calls: [{ t: 41000, method: 'search', ms: 1800 }] },
      { name: 'bboxy450-term-repo_graph-ctags', repo_local: true, contract: { name: 'repo_graph', version: '1.0', ok: null }, tier: 'read', trust: 'untrusted', version: null, auth: null, health: { state: 'never run', say: 'repo-local: listed, never run until trusted' }, enabled: false, calls: [] },
    ],
    // knowledge roots (knowledge-base §3 Roots and the swap; e2e G4): each root names its provider; swapping never moves a file, the new
    // provider reads the same records or refuses with a reason the app shows
    knowledge: { roots: [{ name: '.bboxy', path: '~/mydev/bboxy450-term/.bboxy', provider: 'bboxy450-term-data_source-bboxy', records: 418, answering: 'logic · in process' }],
      offered: ['bboxy450-term-data_source-bboxy', 'bboxy450-term-data_source-stub'] },
    // messaging attention (messaging-attention §3, MA-8): per seat, (destination, class) → wake | inbox | digest. policy null means the
    // role default (an omitted policy on the seat record); deferred is the pending digest; muted is the operator's mute, not the policy
    messaging: { seat: { name: 'Sable Embertail', role: 'executor' }, channels: ['#bboxy450-term', '#bboxy450-crates'], rules: [], deferred: { n: 7, at: '14:30' }, muted: false },
    appearance: { theme: 'auto', density: 'cozy', rim: TOKENS.glow.rim, glow: true, subtitle: false, opacity: TOKENS.window.opacity, blur: TOKENS.window.blur, blurRadius: TOKENS.window.blur_radius },
    shortcuts: [['New tab', '⌘T'], ['Split right', '⌘D'], ['Split down', '⌘⇧D'], ['Mute seat', '⌘M'], ['Undo close', '⌘Z'], ['Settings', '⌘,'], ['Focus next pane', 'Tab'], ['Focus previous pane', '⇧Tab']],
  };
  const SET = JSON.parse(JSON.stringify(SET_DEF));
  SET.roles.creativity.prompt += ' Name the Censor when it fires.'; SET.harness.pi.role = 'creativity';
  // settingsPage({ section, onChange, notifications, custody, plugins, knowledge, messaging }) — onChange(section, key, value): 'appearance' and 'notifications' per field; 'roles' /
  // 'harness' with the record, null on delete. notifications: a state's own values instead of SET.notifications
  function settingsPage(opts) {
    opts = opts || {};
    const emit = (s, k, v) => { if (opts.onChange) opts.onChange(s, k, v); };
    const sections = ['Account', 'Cloud', 'Machines', 'Roles', 'Harnesses', 'Plugins', 'Knowledge', 'Appearance', 'Notifications', 'Messaging', 'Shortcuts'];
    const NAV_ICON = { Account: 'key', Cloud: 'cloud', Machines: 'ssh', Roles: 'roles', Harnesses: 'gear', Plugins: 'plug', Knowledge: 'book', Appearance: 'sun', Notifications: 'bell', Messaging: 'tray', Shortcuts: 'keys' };
    const SENDS = ['all channels · all DMs', 'own swarm · operator DM', 'operator DM only'], WAKES = ['socket', 'hook', 'mcp', 'pty'], STATE_FROM = ['osc7501', 'hooks', 'output cadence'];
    const MARKS = ['♔', '♖', '♟', '♝', '♜', '♞', '♚'], ICONS = ['claude', 'codex', 'pi', 'omp', 'zsh'];
    const HUES = [0, 30, 55, 95, 160, 190, 215, 250, 285, 325], hue = i => 'hsl(' + HUES[((i % HUES.length) + HUES.length) % HUES.length] + ' 85% 64%)';
    // marks the settings page alone needs; everything else comes from ICON_SVG
    const SVG = {
      roles: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M5 19h14M5.5 16 3.5 7l5 3.5L12 5l3.5 5.5 5-3.5-2 9z"/></svg>',
      book: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M4 5a2 2 0 0 1 2-2h13v16H6a2 2 0 0 0-2 2zM4 21V5M8 7h7"/></svg>',
      bell: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M6 16V11a6 6 0 0 1 12 0v5l1.5 2h-15z"/><path d="M10 20.5a2 2 0 0 0 4 0"/></svg>',
      tray: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><path d="M4 13h4l1.5 3h5L16 13h4"/><path d="M4 13 6.5 5h11L20 13v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z"/></svg>',
      keys: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" stroke-linecap="round"><rect x="3" y="6" width="18" height="12" rx="2"/><path d="M7 10h.01M11 10h.01M15 10h.01M7.5 14h9"/></svg>',
      search: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><circle cx="10.5" cy="10.5" r="6"/><path d="M15.5 15.5 20 20"/></svg>',
      copy: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V6a1 1 0 0 1 1-1h9"/></svg>',
      link: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M10 14a4 4 0 0 0 5.7 0l3-3a4 4 0 0 0-5.7-5.7l-1.5 1.5M14 10a4 4 0 0 0-5.7 0l-3 3a4 4 0 0 0 5.7 5.7l1.5-1.5"/></svg>',
      open: '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 4h6v6M20 4l-9 9M19 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1h5"/></svg>',
    };
    const icon = name => ICON_SVG[name] ? harnessIcon(name) : el('span', { class: 'hicon', html: SVG[name] || '' });
    let cur = opts.section || 'Account', draftRole = null, draftHarness = opts.draftHarness || null;
    const NV = opts.notifications || SET.notifications;
    const MV = opts.messaging || SET.messaging;   // a reference state can pass its own
    const CU = opts.custody || null;   // Account custody (#280, #315): drawn only when given, so the default Account is unchanged
    const PL = opts.plugins || SET.plugins, KN = opts.knowledge || SET.knowledge;
    const root = el('div', { class: 'settings' });
    const nav = el('div', { class: 's-nav' });
    const q = el('input', { class: 's-q', placeholder: 'Search settings', type: 'search' });
    const main = el('div', { class: 's-main' });
    // the one rule: a value that differs from config.toml gets the accent dot and a reset
    const diff = (changed, onReset) => changed ? el('span', { class: 'diff' }, el('i', { title: 'differs from config.toml default' }), el('button', { onclick: onReset }, 'reset')) : null;
    const row = (label, hint, control, changed, onReset) => el('div', { class: 's-row' + (changed ? ' changed' : '') }, el('div', { class: 'lbl' }, label, hint ? el('small', {}, hint) : null), el('div', { class: 'ctl' }, control), diff(changed, onReset));
    const select = selectBox;
    const toggle = (on, cb, off) => el('span', { class: 'toggle' + (on ? ' on' : '') + (off ? ' disabled' : ''), role: 'switch', 'aria-checked': String(on), ...(off ? { 'aria-disabled': 'true' } : { onclick: () => cb && cb(!on) }) });
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
        // device-key custody (id1-seat-chain §4a, #280, #315), first when given (a hold is the thing to see): tier, the item's ACL, the hold and its two exits. Unlock is shown only while
        // held: one interactive Keychain access, macOS shows its own dialog, then the no-dialog default returns; the act is journaled first,
        // signature pending. No device secret or device ID is shown (public status hides it under a hold).
        if (CU) {
          const TIER = { keychain: 'login Keychain · native', file: 'file · test account', 'local-dev': 'local-dev issuer' };
          const ACL = { designated: 'keyed to the team-signed requirement (UQB3368A84), verified · every build keeps access', legacy: 'keyed to one build’s signature · migrates during the one unlock', unknown: 'unknown until verified' };
          const SAY = { pending: ['waiting for the macOS Keychain dialog', 'act journaled on account/control · signature pending'], cancelled: ['you cancelled the macOS dialog', 'signing still held · the no-dialog default is back · Unlock again to retry'],
            unknown: ['no answer from the Keychain', 'signing still held · the result is kept under op ' + (CU.op || '—') + ' · Check result reads it, never reopens the dialog'],
            granted: ['macOS granted access', 'signature pending until the device signer returns · the pending entry stays as written'],
            returned: ['the device signer returned', 'signing ready · a device-signed completion links to the pending entry ' + (CU.op || '')] };
          // #315: the pending envelope is immutable; a grant alone proves no signer, so signing stays held (pending) until one returns
          const held = !!CU.hold, at = CU.attempt && SAY[CU.attempt];
          add('Account', card('Custody', [
            row('Device key', 'where the device leaf key lives', chip(TIER[CU.tier] || CU.tier)),
            row('Access rule', 'the Keychain item’s ACL', chip(ACL[CU.acl] || CU.acl, CU.acl === 'designated' ? 'ok' : 'warn')),
            row('Signing', held ? 'held, fail closed; the terminal keeps running' : 'available', chip(held ? (CU.attempt === 'granted' ? 'signature pending' : 'held') : 'ready', held ? 'warn' : 'ok')),
            held ? el('div', { class: 'cu-hold', role: 'status' }, el('b', {}, 'Pending: ' + CU.hold.reason + '; signing held'),
              el('div', {}, 'Clear it with Unlock here or ', el('code', {}, 'bboxy450-term ctl identity unlock'), ': one interactive Keychain access, macOS shows its own dialog, then the no-dialog default returns.'),
              // a hold that remains after a known outcome keeps Unlock (one grant can return the local issuer while a separately
              // certified device key stays held, Kestrel #280/#315); each click is a new explicit act with at most one interactive attempt.
              // unknown offers only Check result: it reads the retained result for the same op and never reopens the dialog
              el('div', { class: 'cu-act' }, CU.attempt === 'unknown' ? btn('Check result', 'open', 'primary') : btn(CU.attempt === 'pending' ? 'Waiting for macOS…' : 'Unlock', 'key', CU.attempt === 'pending' ? 'quiet' : 'primary'))) : null,
            at ? el('div', { class: 'cu-out ' + CU.attempt, role: 'status' }, el('b', {}, at[0]), ' · ' + at[1]) : null,
          ], null, 'custody'));
        }
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
          ...SET.cloud.map(([p, r, on, col, link], i) => { const ch = on !== SET_DEF.cloud[i][2]; return el('div', { class: 'ptile' + (ch ? ' changed' : '') },
            el('div', { class: 'ph' }, el('span', { class: 'tile', style: 'background:' + col }, p[0]), el('div', { class: 'pt' }, el('b', {}, p), el('small', {}, r)), on ? linkDot(link, false) : dot(false)),
            el('div', { class: 'pf' }, el('span', {}, on ? linkSay(link) : 'not connected'), diff(ch, () => { SET.cloud[i][2] = SET_DEF.cloud[i][2]; draw(); }), toggle(on, v => { SET.cloud[i][2] = v; draw(); }))); }),
          el('button', { class: 'ptile add' }, '+ connect a provider')),
        note('Regions appear under Cloud in the launcher. Credentials live in the keychain, never in config.toml.'),
      ]));

      if (cur === 'Machines' || needle) add('Machines', card('SSH machines', [
        ...SET.machines.map(([h, u, hw, k, link]) => el('div', { class: 'mrow' }, linkDot(link, false), el('div', { class: 'mt' }, el('b', {}, h), el('code', {}, u + ' · ' + linkSay(link))), el('span', { class: 'hw' }, hw), el('span', { class: 'kbd' }, k))),
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
          (() => { // rim size (operator 2026-10-23): the alive rim's thickness, centred on the border, half outside and half inside;
            // the gap between panes is the rim size so neighbours' halves meet, so changing it relayouts. Drag previews; release commits
            const val = el('b', { class: 'wo-pct' }, v.rim + ' px');
            const slider = el('input', { type: 'range', min: String(TOKENS.glow.rim_min), max: String(TOKENS.glow.rim_max), step: '1', value: String(v.rim), oninput: e => { val.textContent = e.target.value + ' px'; }, onchange: e => setA('rim')(+e.target.value) });
            return row('Rim size', 'the glowing border, half outside and half inside · panes space to fit · ' + TOKENS.glow.rim_min + ' to ' + TOKENS.glow.rim_max + ' px', el('span', { class: 'wo' }, slider, val), v.rim !== d.rim, rsA('rim'));
          })(),
          (() => { // window transparency: drag previews live without redrawing the page; release commits
            const pct = el('b', { class: 'wo-pct' }, Math.round(v.opacity * 100) + '%');
            const prev = el('span', { class: 'win-prev' + (v.blur ? ' blur' : ''), style: '--wa:' + v.opacity }, el('i', { class: 'wp-side' }), el('i', { class: 'wp-pane' }, el('i'), el('i'), el('i')));
            const slider = el('input', { type: 'range', min: String(TOKENS.window.opacity_min * 100), max: '100', step: '5', value: String(Math.round(v.opacity * 100)), oninput: e => { v.opacity = +e.target.value / 100; pct.textContent = e.target.value + '%'; prev.style.setProperty('--wa', v.opacity); emit('appearance', 'opacity', v.opacity); }, onchange: () => draw() });
            return row('Window opacity', 'panes only; chrome and text stay opaque', el('span', { class: 'wo' }, prev, slider, pct), v.opacity !== d.opacity, rsA('opacity'));
          })(),
          row('Blur behind', 'what is behind the window, like iTerm2', toggle(v.blur, setA('blur')), v.blur !== d.blur, rsA('blur')),
          v.blur ? row('Blur radius', null, seg(['10 px', '20 px', '40 px'], v.blurRadius + ' px', x => setA('blurRadius')(parseInt(x, 10))), v.blurRadius !== d.blurRadius, rsA('blurRadius')) : null,
          row('Border glow', 'particles in the seat colour', toggle(v.glow, setA('glow')), v.glow !== d.glow, rsA('glow')),
          row('Last-output subtitle', 'second line on seat rows', toggle(v.subtitle, setA('subtitle')), v.subtitle !== d.subtitle, rsA('subtitle')),
        ]));
      }

      if (cur === 'Plugins' || needle) {
        // onChange('plugins', <name>, { enabled }) and ('plugins', <name>, 'setup'); a root's provider shows 'provider of <root>' instead
        const WIN = 60000, healthDot = h => el('i', { class: 'sdot pl-' + h.state.replace(' ', '-'), title: h.state + (h.say ? ' · ' + h.say : '') });
        const lane = (calls, hooks) => el('div', { class: 'pl-lane' + (hooks ? ' hooks' : '') }, ...calls.map(k => el('i', { class: 'pl-bar' + (k.ms >= 5000 ? ' slow' : ''), title: (k.hook || k.method) + ' · ' + (k.ms >= 1000 ? (k.ms / 1000).toFixed(1) + ' s' : k.ms + ' ms'),
          style: 'left:' + ((WIN - k.t) / WIN * 100).toFixed(2) + '%;width:max(2px,' + (Math.min(k.ms, k.t) / WIN * 100).toFixed(2) + '%)' })));
        const fmt = ms => ms >= 1000 ? (ms / 1000).toFixed(1) + ' s' : ms + ' ms';
        const timeline = p => { const calls = p.calls.filter(k => !k.hook), hooks = p.calls.filter(k => k.hook), all = p.calls.map(k => k.ms).sort((a, b) => a - b);
          const flame = p.trace && (p.calls.find(k => k.spans) || {}).spans;
          return el('div', { class: 'pl-tl' }, el('div', { class: 'pl-axis' }, el('span', {}, 'calls'), el('span', {}, 'hooks'), el('span', { class: 'pl-ago' }, '60 s ago · now')),
            el('div', { class: 'pl-lanes' }, lane(calls), lane(hooks, true)),
            el('div', { class: 'pl-sum' }, all.length ? all.length + ' in the last minute · p50 ' + fmt(all[Math.floor(all.length / 2)]) + ' · max ' + fmt(all[all.length - 1]) : 'no calls in the last minute'),
            flame ? el('div', { class: 'pl-flame', title: 'spans from the plugin, merged under the host call (trace: true)' }, ...flame.map(sp => el('i', { class: 'pl-span d' + sp.depth, style: 'left:' + (sp.start / flame[0].ms * 100).toFixed(2) + '%;width:' + (sp.ms / flame[0].ms * 100).toFixed(2) + '%;top:' + (sp.depth * 18) + 'px' }, sp.name + ' ' + fmt(sp.ms)))) : null); };
        add('Plugins', card('Plugins', [note('Binaries in plugins/ under the app support directory; repo-local ones in .bboxy/plugins/ are listed and never run until you trust them. Each speaks one contract; the tier is the data that leaves the machine.'),
          ...PL.map(p => { const sys = p.contract.name, prov = p.name.split('-').pop();
            const trust = p.trust === 'trusted' ? chip('trusted', 'ok') : chip(p.repo_local ? 'repo-local · not trusted' : 'not trusted', 'warn');
            const comp = p.contract.ok === true ? chip(sys + ' ' + p.contract.version + ' ✓') : p.contract.ok === false ? chip(sys + ' ' + p.contract.version + ' ✕ ' + p.contract.failing, 'bad') : chip(sys + ' ' + p.contract.version);
            // no optimistic state (P-1 Settings binding): a pending trust or enable keeps the last observed value, with a caption, until the
            // observation arrives; a refusal leaves the observed value and says why. Replay re-sends the same intent and shows pending again
            const pend = p.pending && el('span', { class: 'pl-pend', role: 'status' }, { trust: 'trusting…', enable: 'enabling…', disable: 'disabling…' }[p.pending.verb]);
            const ctl = p.root ? el('span', { class: 'pl-root' }, 'provider of ' + p.root) : p.trust !== 'trusted' ? (pend || btn('Trust…', null, 'primary'))
              : el('span', { class: 'pl-ctl' }, pend, toggle(p.enabled, x => { p.enabled = x; emit('plugins', p.name, { enabled: x }); draw(); }, !!p.pending));
            return el('div', { class: 'pl-card' + (p.alerts ? ' alert' : '') + ' h-' + p.health.state.replace(' ', '-') },
              el('div', { class: 'pl-h' }, healthDot(p.health), el('b', {}, p.name + (p.bundled ? ' (bundled)' : '')), el('span', { class: 'pl-ver' }, p.version || ''), el('span', { class: 'pl-sp' }), ctl),
              el('div', { class: 'pl-chips' }, trust, chip('tier ' + p.tier + (p.local ? ' (local)' : '')), comp, p.auth ? chip(p.auth === 'ok' ? 'logged in' : p.auth, p.auth === 'ok' ? '' : 'warn') : null, p.grant ? chip(p.grant) : null, p.trace ? chip('trace') : null,
                el('span', { class: 'pl-health' }, p.health.state + (p.health.say ? ' · ' + p.health.say : '')), p.auth === 'needs login' || p.health.state === 'crashed' ? btn('Re-run setup', null, 'quiet') : null),
              ...(p.alerts || []).map(a => el('div', { class: 'pl-alert', role: 'status' }, '⚠ ', a)),
              p.refused ? el('div', { class: 'pl-alert', role: 'status' }, '⚠ ' + p.refused.verb + ' refused · ' + p.refused.reason) : null,
              p.trust === 'trusted' ? timeline(p) : null); })]));
      }
      if (cur === 'Knowledge' || needle) {
        // onChange('knowledge', <root name>, <provider>): the swap changes which process answers, never where a record lives
        add('Knowledge', card('Knowledge roots', [note('A root is a place records live; each names the provider that reads it. Swapping the provider moves no file: the new one reads the same records, or refuses with its reason.'),
          ...KN.roots.map(r => el('div', { class: 'kn-root' + (r.refused ? ' refused' : '') },
            el('div', { class: 'kn-h' }, icon('book'), el('b', {}, r.name), el('code', {}, r.path), el('span', { class: 'pl-sp' }), el('span', { class: 'kn-n' }, r.records + ' records')),
            // no optimistic state, as for plugin intents: while a swap is pending the observed provider still answers and the select stays on it
            row('Provider', r.pending ? 'swapping to ' + r.pending.to + '…' : r.provider === KN.offered[0] ? 'bundled, in process' : 'a trusted plugin over stdio', r.pending ? (() => { const sel = select(KN.offered, r.provider); sel.classList.add('disabled'); sel.querySelector('select').disabled = true; return sel; })() : select(KN.offered, r.provider, x => { r.provider = x; emit('knowledge', r.name, x); draw(); }), r.provider !== KN.offered[0], () => { r.provider = KN.offered[0]; emit('knowledge', r.name, r.provider); draw(); }),
            el('div', { class: 'kn-ans' }, 'answering: ', el('b', {}, r.answering), r.digests ? el('span', {}, ' · ' + r.digests) : null),
            r.refused ? el('div', { class: 'pl-alert', role: 'status' }, '⚠ ', r.refused) : null)),
          note('A second provider is offered only after it passes bboxy450-term plugin check and the contract’s conformance fixtures.')]));
      }
      if (cur === 'Notifications' || needle) {
        // onChange('notifications', 'enabled' | 'only_when_not_frontmost', bool) and ('notifications', 'disabled_harnesses', [ids]) from
        // one toggle per harness; with enabled off the rest stays visible but disabled, so turning it back on restores what was set
        const v = NV, d = SET_DEF.notifications, on = v.enabled, setN = k => x => { v[k] = x; emit('notifications', k, x); draw(); };
        const hon = h => !v.disabled_harnesses.includes(h), hdef = h => !d.disabled_harnesses.includes(h);
        const setH = h => x => { v.disabled_harnesses = x ? v.disabled_harnesses.filter(k => k !== h) : [...v.disabled_harnesses, h]; emit('notifications', 'disabled_harnesses', v.disabled_harnesses); draw(); }, rsH = h => () => setH(h)(hdef(h));
        add('Notifications', card('Notifications', [
          row('Desktop notifications', 'a seat that needs your answer posts one; clicking it focuses the seat', toggle(on, setN('enabled')), on !== d.enabled, () => setN('enabled')(d.enabled)),
          row('Only in the background', 'none while this window is frontmost', toggle(v.only_when_not_frontmost, setN('only_when_not_frontmost'), !on), v.only_when_not_frontmost !== d.only_when_not_frontmost, () => setN('only_when_not_frontmost')(d.only_when_not_frontmost)),
          group('Per harness', 'off silences every seat of that harness'),
          ...Object.keys(SET.harness).map(h => row(el('span', { class: 'nh' }, harnessIcon(SET.harness[h].icon || 'zsh'), h), null, toggle(hon(h), setH(h), !on), hon(h) !== hdef(h), rsH(h))),
          note('A muted seat (⌘M) never notifies and keeps the ordinary waiting rim.'),
        ]));
      }
      if (cur === 'Messaging' || needle) {
        // onChange('messaging', '<destination>.<class>', mode). Role defaults (§3): an omitted cell reads its role's default; a cell that
        // differs gets the accent dot and a reset. Always-wake rules and the operator's mute are shown, never set here
        const ATTN = { oracle: { direct: { conversation: 'wake', event: 'inbox' }, channel: { conversation: 'wake', event: 'inbox' } },
          architect: { direct: { conversation: 'wake', event: 'inbox' }, channel: { conversation: 'inbox', event: 'inbox' } },
          executor: { direct: { conversation: 'wake', event: 'inbox' }, channel: { conversation: 'digest', event: 'inbox' } } };
        ATTN.creativity = ATTN.architect;
        // rules as native stores them (registry.rs AttentionRule): one per exact destination and class; no rule means the role default
        const m = MV, def = ATTN[m.seat.role] || ATTN.executor, kind = d => d === 'direct' ? 'direct' : 'channel';
        const rule = (d, c) => (m.rules || []).find(r => r.destination === d && r.class === c);
        const mode = (d, c) => (rule(d, c) || {}).mode || def[kind(d)][c];
        const setM = (d, c) => x => { m.rules = (m.rules || []).filter(r => !(r.destination === d && r.class === c)); if (x !== def[kind(d)][c]) m.rules.push({ destination: d, class: c, mode: x }); emit('messaging', d + ' ' + c, x); draw(); };
        const line = (d, c, label) => row(label, null, seg(['wake', 'inbox', 'digest'], mode(d, c), setM(d, c)), !!rule(d, c), () => setM(d, c)(def[kind(d)][c]));
        add('Messaging', card('Messaging', [
          el('div', { class: 'att-seat' }, el('b', {}, m.seat.name), chip(m.seat.role + ' defaults'), m.deferred ? el('span', { class: 'att-def' }, m.deferred.n + ' deferred · digest at ' + m.deferred.at) : null),
          m.muted ? el('div', { class: 'pl-alert att-mute', role: 'status' }, '⚠ muted by the operator: this seat’s traffic is held both ways until unmuted (⌘M); the policy below applies after') : null,
          group('Direct and by name', 'this seat’s own inbox'), line('direct', 'conversation', 'Conversation'), line('direct', 'event', 'Events'),
          ...(m.channels || []).flatMap(ch => [group(ch), line(ch, 'conversation', 'Conversation'), line(ch, 'event', 'Events')]),
          note('Always wakes, for conversation only: a message from the operator, a task delegated to this seat, a reply in a thread it started; a seat cannot set itself to miss its own work. Events never wake by default, whoever sent them. A digest delivers once, at 10 min or 50 messages, listing every message by id: it delays a wake and never holds a message. A change applies to messages stored after it.'),
        ]));
      }
      if (cur === 'Shortcuts' || needle) add('Shortcuts', card('Shortcuts', [el('div', { class: 'keys' }, ...SET.shortcuts.map(([l, k]) => el('div', { class: 'kr' }, el('span', {}, l), el('span', { class: 'kbd' }, k))))]));
      if (!main.children.length) main.append(note('nothing matches "' + q.value + '"'));
    };
    q.addEventListener('input', draw); draw();
    root.append(el('div', { class: 's-side' }, el('div', { class: 's-qwrap' }, icon('search'), q), nav, el('div', { class: 's-src' }, '~/.config/bboxy/config.toml')), main);
    return root;
  }

  window.C = { launchFields, composeLaunch, TRANSPORTS, poolName, RECORDS, recordLookup, e2eSummary, e2eChecklist, notification, NOTIFY_BODY, circleMark, taskMark, SEAT_MARK, prioChip, atHM, hhmm, editRuns, editField, linkOf, linkDown, linkSay, linkDot, bodyAfterTitle, diagramFigure, selectBox, composer, reportCard, addrChip, attachCard, perfHud, PERF_PHASES, streamTabs, channelGraphCard, splitView, firstRun, invChip, AUTH_SAY, docModal, diagram, parseMermaid, titleBar, launcherItems, eventRow, consumerCard, subjectChip, EV_ROOT, evTier, docView, DOC_HUE, mdLite, purgeModal, rowMenu, SEND_SCOPES, pendingRow, gapMarker, subagentRows, EVIDENCE, tickLadder, subscriptionsCard, externalLane, renderLinks, provenanceCard, artifactRow, ART_GLYPH, graphPage, epicGroup, taskRow, TASK_STATES, TASK_GLYPH, GRADE_TITLE, KIND_MARK, settingsPage, SET, SET_DEF, LAUNCH, hslToRgb, logo, syncPulse, hash32, rng, seatColor, rollName, HARNESSES, DEFAULT_ROLE, STATES, GLYPH, ROLE_MARK, ICON_SVG, glyph, harnessIcon, roleMark, kbd, makeSeat, makeTree, leaves, makeModel, rollup, seatRow, miniGrid, tabRow, spaceHeader, sidebar, titleStrip, pane, picker, messageRow };
})();
