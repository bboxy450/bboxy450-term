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
  function glyph(state, seat) {
    const color = state === 'active' ? seatColor(seat || { key: 'x' }) : state === 'idle' ? 'var(--c-idle)' : state === 'waiting' ? 'var(--c-waiting)' : 'var(--c-errored)';
    return el('span', { class: 'glyph ' + state, style: 'color:' + color, title: state }, GLYPH[state]);
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
    ears: '<svg viewBox="0 0 24 24" fill="currentColor"><path d="M2 19 5.5 3l5.5 11.5zM22 19 18.5 3 13 14.5z"/></svg>',
  };
  // Logo: head silhouette, flat two-tone (electric fill, navy ear interiors + hair tuft + hood collar), tilted 12° CCW. Ears-only below 32 px.
  function logoSvg(opts) {
    opts = opts || {};
    const c = T.color.dark, fill = opts.fill || c.accent, navy = opts.navy || c.navy, tilt = opts.tilt == null ? T.meta.brand_tilt_deg : opts.tilt;
    // 100x110 box. Ears tall and set wide, cheeks flare, chin comes to a point: a fox, not a cat.
    const head = `<g transform="rotate(${tilt} 50 56)">
      <path fill="${navy}" d="M16 84 Q50 110 84 84 L92 110 L8 110 Z"/>
      <path fill="${fill}" d="M12 2 L38 36 Q50 31 62 36 L88 2 L96 46 C102 64 86 78 64 86 L50 104 L36 86 C14 78 -2 64 4 46 Z"/>
      <path fill="${navy}" d="M43 88 Q50 83 57 88 L50 97 Z"/>
      <path fill="${navy}" d="M17 17 L30 35 L14 39 Z M83 17 L70 35 L86 39 Z"/>
      <path fill="${navy}" d="M38 46 L42 33 L47 41 L53 29 L59 40 L66 35 L64 48 Q52 42 38 50 Z"/>
      ${opts.face ? `<path fill="${c.fg}" d="M38 66 Q50 60 62 66 Q60 84 50 90 Q40 84 38 66 Z"/><ellipse cx="50" cy="70" rx="4.5" ry="3.5" fill="${navy}"/>` : ''}
    </g>`;
    const ears = `<g transform="rotate(${tilt} 50 55)"><path fill="${fill}" d="M8 102 L24 6 L50 72 L76 6 L92 102 L50 90 Z"/><path fill="${navy}" d="M42 98 L50 90 L58 98 L50 108 Z"/><path fill="${navy}" d="M25 30 L38 66 L20 78 Z M75 30 L62 66 L80 78 Z"/></g>`;
    return `<svg viewBox="0 0 100 110" xmlns="http://www.w3.org/2000/svg">${opts.ears ? ears : head}</svg>`;
  }
  // Logo = the operator's identity portrait (lab/web/bboxy.png, from ~/.agents/identity/bboxy.png). The SVG mark stays only for the logo page.
  function logo(px, opts) { opts = opts || {}; if (opts.svg) { const ears = opts.ears != null ? opts.ears : px < 32; return el('span', { class: 'logo', style: 'width:' + px + 'px;height:' + px + 'px;display:inline-block', html: logoSvg({ ...opts, ears }) }); }
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
    const unread = opts.unread != null ? opts.unread : seat.unread, unreadRole = opts.unreadRole || seat.unreadRole || 'executor';
    if (unread) right.append(el('span', { class: 'badge', style: 'background:var(--role-' + unreadRole + ')' }, unread));
    if (opts.shortcut) right.append(kbd(opts.shortcut));
    right.append(glyph(seat.state, seat));
    row.append(right);
    if (opts.subtitle) row.append(el('span', { class: 'sub' }, opts.subtitle));
    row.dataset.key = seat.key;
    return syncPulse(row, seat.state);
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
      return el('div', {}, seatRow(s, { focused: s.key === opts.focusedKey, shortcut: tab.shortcut }));
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
      for (const s of seats) body.append(seatRow(s, { focused: s.key === opts.focusedKey }));
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
        body.append(tabRow(tab, { expanded: open, focusedKey: st.focusedKey, onToggle: () => { st.openTabs.has(tkey) ? st.openTabs.delete(tkey) : st.openTabs.add(tkey); st.rerender(); } }));
      }
      for (const c of (space.closed || [])) body.append(el('div', { class: 'tab-row closed-strip', 'data-closed': c.seat.key }, el('span', { class: 'x' }, '×'), harnessIcon(c.seat.harness), el('span', { class: 'tname' }, c.seat.name), el('span', { class: 'right' }, el('span', { class: 'kbd left' }, ''), el('button', { class: 'undo', onclick: e => { e.stopPropagation(); st.onUndo && st.onUndo(c); } }, 'undo ⌘Z'))));
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
    const st = { openSpaces: new Set(opts.openSpaces || []), openTabs: new Set(opts.openTabs || []), focusedKey: opts.focusedKey || seats[0].key, autoOpenSingles: seats.length <= 40, onNewSeat: opts.onNewSeat, onColor: opts.onColor, onUndo: opts.onUndo, onRename: opts.onRename, editName: opts.editName };
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
    return el('div', { class: 'term', html: lines[seat.state] });
  }
  function titleStrip(seat, opts) {
    opts = opts || {};
    const s = el('div', { class: 'strip', style: 'background:' + seatColor(seat) }, el('span', { class: 'close', title: 'close seat', onclick: e => { e.stopPropagation(); if (opts.onClose) opts.onClose(seat); } }));
    if (seat.where && seat.where !== 'local') s.append(el('span', { class: 'where', title: seat.where + (seat.host ? ' · ' + seat.host : '') }, harnessIcon(seat.where)));
    s.append(harnessIcon(seat.harness), el('span', { class: 'name' }, seat.name));
    if (seat.role) s.append(roleMark(seat.role));
    s.append(el('span', { class: 'title' }, seat.title || ''));
    if (opts.shortcut) s.append(kbd(opts.shortcut));
    if (opts.onMute) s.append(el('span', { class: 'mute' + (seat.muted ? ' on' : ''), title: seat.muted ? 'unmute (⌘M)' : 'mute: 1:1 with the operator (⌘M)', onclick: e => { e.stopPropagation(); opts.onMute(seat); } }, harnessIcon(seat.muted ? 'muted' : 'unmuted')));
    s.append(glyph(seat.state, seat));
    return s;
  }
  function pane(seat, opts) {
    opts = opts || {};
    const p = el('div', { class: 'pane ' + seat.state + (opts.unfocused ? ' unfocused' : '') + (opts.focused ? ' focused' : ''), style: '--seat:' + seatColor(seat) + (opts.height ? ';min-height:' + opts.height + 'px' : '') });
    p.append(titleStrip(seat, opts), fakeTerm(seat));
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
    const climb = () => { if (!stack.length) return; stack = stack.slice(0, -1); q = ''; input.value = ''; sel = 0; draw(); };
    const pick = (it, ev) => { if (!it) return; if (it.children && !it.enterPicks) return dive(it); if (opts.onPick) opts.onPick(it, nameFor(seed), { meta: !!(ev && (ev.metaKey || ev.ctrlKey)), shift: !!(ev && ev.shiftKey), alt: !!(ev && ev.altKey) }, path()); };
    function draw() {
      list.innerHTML = ''; crumbs.innerHTML = '';
      for (const c of path()) crumbs.append(el('span', { class: 'crumb' }, c.label));
      const items = matches();
      if (!items.length) list.append(el('div', { class: 'empty' }, 'nothing matches "' + q + '"'));
      let lastSec = null;
      items.forEach((it, i) => {
        if (it.section && it.section !== lastSec) { list.append(el('div', { class: 'sec' }, it.section)); lastSec = it.section; }
        const o = el('div', { class: 'opt' + (i === sel ? ' sel' : '') + (it.children ? ' parent' : ''), onclick: e => pick(it, e), onpointerenter: () => { if (sel !== i) { sel = i; draw(); } } }, el('span', { class: 'tile', style: 'background:' + (it.tile || 'var(--surface-2)') }, harnessIcon(it.harness)));
        const label = el('span', { class: 'lbl' }); const L = it.label, qq = scope().qq, m = qq && norm(L).startsWith(qq) ? qq.length : 0;
        if (m) label.append(el('span', { class: 'm' }, L.slice(0, m)), L.slice(m)); else label.textContent = L;
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
  function messageRow(m, opts) {
    opts = opts || {};
    if (m.presence) return el('div', { class: 'msg presence' }, el('span', { class: 'ptext' }, m.text), el('span', { class: 'time' }, m.time || ''));
    const role = m.role || 'unsigned';
    const row = el('div', { class: 'msg ' + role + (m.unread ? ' unread' : '') + (m.replyTo ? ' nested' : ''), style: '--role:var(--role-' + role + ')' });
    const head = el('div', { class: 'head' }, el('span', { class: 'who' }, m.from), el('span', { class: 'role' }, '<' + role.toUpperCase() + '>'));
    if (role === 'oracle' && !m.foreign) head.append(el('span', { class: 'p0' }, 'p0'));  // a foreign Oracle ranks as a worker here
    head.append(el('span', { class: 'time' }, m.time || ''));
    const body = el('div', { class: 'body' }, head);
    if (m.replyTo) body.append(el('div', { class: 'reply' }, '↩ ' + m.replyTo));
    body.append(el('div', { class: 'text' }, m.text));
    row.append(el('div', { class: 'rail' }), body);
    if (opts.ripple) row.append(el('div', { class: 'ripple' }));
    return row;
  }

  // ---------- settings: data-driven page, left nav + cards, read-mostly (config.toml), diff-from-default marks ----------
  const SET_DEF = {
    account: { name: 'Boxy', email: 'bboxy450@gmail.com', fingerprint: '844E CC05 87DE 7AB0', devices: [['this Mac', 'enrolled · root custodian'], ['rig', 'enrolled · 2026-10-19']] },
    cloud: [['Hetzner', 'eu-2 · Falkenstein', true], ['Fly', 'us-1 · Ashburn', true], ['Lambda', 'gpu-a · A100', false]],
    machines: [['rig', 'feral@rig.local', 'M2 Ultra · key ~/.ssh/id_ed25519'], ['alpha', 'feral@alpha.tail', 'Linux x86 · key ~/.ssh/id_ed25519'], ['studio', 'feral@studio.local', 'M1 Max · key ~/.ssh/id_ed25519']],
    roles: { oracle: { prompt: 'You are the Oracle. Supervise, verify, never implement. Sign every message Name <ORACLE>.', send: 'all channels · all DMs' }, executor: { prompt: 'You are an Executor. Do the assigned work, report evidence, never expand scope.', send: 'own swarm · operator DM' }, creativity: { prompt: 'You are Creativity. Diverge before converging; return a field, not a pick.', send: 'own swarm · operator DM' } },
    harness: { claude: { role: 'oracle', wake: 'socket', hooks: true }, codex: { role: 'executor', wake: 'hook', hooks: true }, pi: { role: 'executor', wake: 'pty', hooks: false }, omp: { role: 'executor', wake: 'pty', hooks: false } },
    appearance: { theme: 'auto', density: 'cozy', gutter: 2, glow: true, subtitle: false },
    shortcuts: [['New tab', '⌘T'], ['Split right', '⌘D'], ['Split down', '⌘⇧D'], ['Mute seat', '⌘M'], ['Undo close', '⌘Z'], ['Settings', '⌘,'], ['Focus next pane', 'Tab']],
  };
  const SET = JSON.parse(JSON.stringify(SET_DEF));
  SET.roles.creativity.prompt += ' Name the Censor when it fires.'; SET.harness.pi.role = 'creativity';
  function settingsPage(opts) {
    opts = opts || {};
    const sections = ['Account', 'Cloud', 'Machines', 'Roles', 'Harnesses', 'Appearance', 'Shortcuts'];
    let cur = opts.section || 'Account';
    const root = el('div', { class: 'settings' });
    const nav = el('div', { class: 's-nav' });
    const q = el('input', { class: 's-q', placeholder: 'search settings' });
    const main = el('div', { class: 's-main' });
    const row = (label, hint, control, changed, onReset) => el('div', { class: 's-row' + (changed ? ' changed' : '') }, el('div', { class: 'lbl' }, label, hint ? el('small', {}, hint) : null), el('div', { class: 'ctl' }, control), el('div', { class: 'diff' }, changed ? [el('i', { title: 'differs from config.toml default' }), el('button', { onclick: onReset }, 'reset')] : null));
    const select = (optsArr, val, on) => { const s = el('select', { onchange: e => on && on(e.target.value) }); for (const o of optsArr) s.append(el('option', { value: o, ...(o === val ? { selected: '' } : {}) }, o)); return s; };
    const toggle = (on, cb) => el('span', { class: 'toggle' + (on ? ' on' : ''), onclick: () => cb && cb(!on) });
    const card = (title, kids, extra) => el('div', { class: 's-card' }, title ? el('div', { class: 's-card-h' }, el('span', {}, title), extra || null) : null, ...kids);
    const btn = (label, icon, primary) => el('button', { class: 's-btn' + (primary ? ' primary' : '') }, icon ? harnessIcon(icon) : null, label);
    const draw = () => {
      nav.innerHTML = ''; for (const s of sections) nav.append(el('div', { class: 's-navi' + (s === cur ? ' on' : ''), onclick: () => { cur = s; draw(); } }, s));
      main.innerHTML = '';
      const needle = q.value.toLowerCase();
      const add = (title, node) => { if (!needle || node.textContent.toLowerCase().includes(needle)) main.append(node); };
      if (cur === 'Account' || needle) {
        const a = SET.account;
        add('Account', card('Account', [
          el('div', { class: 'passport' }, logo(56, { square: true }), el('div', {}, el('div', { class: 'pname' }, a.name, el('span', { class: 'pill-ok' }, 'signed in · root key on this Mac')), el('div', { class: 'fp' }, harnessIcon('key'), 'ed25519 ' + a.fingerprint), el('div', { class: 'fp' }, a.email)), el('div', { class: 'acts' }, btn('Sign in with Google', 'google'), btn('Show paper recovery', 'key'))),
          row('Devices', 'enrolled under the root key', el('div', { class: 'list' }, ...a.devices.map(([n, d]) => el('div', { class: 'li' }, el('b', {}, n), el('span', {}, d))), btn('+ enroll a device')), false),
          el('div', { class: 's-note' }, 'Google sign-in binds this account to a provider for cloud; the root key stays the identity. Device enrollment is a one-use ticket approved from a custodian device. Both ship with cloud.'),
        ]));
      }
      if (cur === 'Cloud' || needle) add('Cloud', card('Cloud', SET.cloud.map(([p, r, on], i) => row(p, r, toggle(on, v => { SET.cloud[i][2] = v; draw(); }), on !== SET_DEF.cloud[i][2], () => { SET.cloud[i][2] = SET_DEF.cloud[i][2]; draw(); })), btn('+ connect a provider')));
      if (cur === 'Machines' || needle) add('Machines', card('SSH machines', [...SET.machines.map(([h, u, d]) => row(h, d, el('code', {}, u), false)), el('div', { class: 's-note' }, 'Read from ~/.ssh/config. Machines appear under SSH in the launcher.')], btn('+ add a machine')));
      if (cur === 'Roles' || needle) for (const r of ['oracle', 'executor', 'creativity']) {
        const v = SET.roles[r], d = SET_DEF.roles[r];
        add('Roles', card(r, [
          row('Default system prompt', 'fed to the harness at launch', el('textarea', { html: v.prompt, oninput: e => { v.prompt = e.target.value; } }), v.prompt !== d.prompt, () => { v.prompt = d.prompt; draw(); }),
          row('May send to', 'terminal-enforced', select(['all channels · all DMs', 'own swarm · operator DM', 'operator DM only'], v.send, x => { v.send = x; draw(); }), v.send !== d.send, () => { v.send = d.send; draw(); }),
        ], el('span', { class: 'rmark', style: 'color:var(--role-' + r + ')' }, ROLE_MARK[r])));
      }
      if (cur === 'Harnesses' || needle) for (const h of Object.keys(SET.harness)) {
        const v = SET.harness[h], d = SET_DEF.harness[h];
        add('Harnesses', card(h, [
          row('Default role', null, select(['oracle', 'executor', 'creativity'], v.role, x => { v.role = x; draw(); }), v.role !== d.role, () => { v.role = d.role; draw(); }),
          row('Wake method', null, select(['socket', 'hook', 'mcp', 'pty'], v.wake, x => { v.wake = x; draw(); }), v.wake !== d.wake, () => { v.wake = d.wake; draw(); }),
          row('Hooks', 'state from harness hooks', toggle(v.hooks, x => { v.hooks = x; draw(); }), v.hooks !== d.hooks, () => { v.hooks = d.hooks; draw(); }),
        ], el('span', { class: 'tile', style: 'background:' + ((LAUNCH.find(l => l.harness === h) || {}).tile || '#374151') }, harnessIcon(h))));
      }
      if (cur === 'Appearance' || needle) { const v = SET.appearance, d = SET_DEF.appearance; add('Appearance', card('Appearance', [
        row('Theme', null, select(['auto', 'dark', 'light'], v.theme, x => { v.theme = x; draw(); }), v.theme !== d.theme, () => { v.theme = d.theme; draw(); }),
        row('Density', null, select(['touch', 'comfortable', 'cozy', 'compact'], v.density, x => { v.density = x; draw(); }), v.density !== d.density, () => { v.density = d.density; draw(); }),
        row('Gutter', 'px between panes', select(['2', '4'], String(v.gutter), x => { v.gutter = +x; draw(); }), v.gutter !== d.gutter, () => { v.gutter = d.gutter; draw(); }),
        row('Border glow', 'particles in the seat colour', toggle(v.glow, x => { v.glow = x; draw(); }), v.glow !== d.glow, () => { v.glow = d.glow; draw(); }),
        row('Last-output subtitle', 'second line on seat rows', toggle(v.subtitle, x => { v.subtitle = x; draw(); }), v.subtitle !== d.subtitle, () => { v.subtitle = d.subtitle; draw(); }),
      ])); }
      if (cur === 'Shortcuts' || needle) add('Shortcuts', card('Shortcuts', SET.shortcuts.map(([l, kb]) => row(l, null, el('span', { class: 'kbd' }, kb), false))));
      if (!main.children.length) main.append(el('div', { class: 's-note' }, 'nothing matches "' + q.value + '"'));
    };
    q.addEventListener('input', draw); draw();
    root.append(el('div', { class: 's-side' }, q, nav), main);
    return root;
  }

  window.C = { settingsPage, SET, SET_DEF, LAUNCH, hslToRgb, logo, logoSvg, syncPulse, hash32, rng, seatColor, rollName, HARNESSES, DEFAULT_ROLE, STATES, GLYPH, ROLE_MARK, ICON_SVG, glyph, harnessIcon, roleMark, kbd, makeSeat, makeTree, leaves, makeModel, rollup, seatRow, miniGrid, tabRow, spaceHeader, sidebar, titleStrip, pane, picker, messageRow };
})();
