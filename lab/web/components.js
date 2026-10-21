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

  // Task row: a delegated task under its seat. State is a glyph + chip; reported shows an accept pill (Oracle/operator).
  const TASK_STATES = ['assigned', 'acknowledged', 'running', 'reported', 'accepted', 'rejected', 'reassigned'];
  const TASK_GLYPH = { assigned: '○', acknowledged: '◔', running: '◑', reported: '◕', accepted: '●', rejected: '✕', reassigned: '↷' };
  function taskRow(task, seat, opts) {
    opts = opts || {};
    const row = el('div', { class: 'task-row ' + task.state, 'data-task': task.id, title: task.title + ' · ' + task.state + (task.from ? ' · from ' + task.from : '') });
    row.append(el('span', { class: 'tglyph' }, TASK_GLYPH[task.state] || '○'), el('span', { class: 'ttitle' }, task.title));
    const right = el('span', { class: 'right' });
    if (task.state === 'reported' && opts.onAccept) right.append(el('button', { class: 'accept', onclick: e => { e.stopPropagation(); opts.onAccept(task, seat); } }, 'accept'));
    right.append(el('span', { class: 'tstate' }, task.state));
    row.append(right);
    return row;
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
      const w = el('div', {}, seatRow(s, { focused: s.key === opts.focusedKey, shortcut: tab.shortcut }));
      for (const t of (s.tasks || [])) w.append(taskRow(t, s, { onAccept: opts.onAccept }));
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
      for (const s of seats) { body.append(seatRow(s, { focused: s.key === opts.focusedKey })); for (const t of (s.tasks || [])) body.append(taskRow(t, s, { onAccept: opts.onAccept })); }
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
    const st = { openSpaces: new Set(opts.openSpaces || []), openTabs: new Set(opts.openTabs || []), focusedKey: opts.focusedKey || seats[0].key, autoOpenSingles: seats.length <= 40, onNewSeat: opts.onNewSeat, onColor: opts.onColor, onUndo: opts.onUndo, onRename: opts.onRename, editName: opts.editName, onAccept: opts.onAccept };
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
    s.append(el('span', { class: 'title' }, (seat.title || '') + (seat.state === 'waiting' ? ' · waiting for ' + (seat.waitKind || 'an answer') : '')));
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
    let cur = opts.section || 'Account', draftRole = null, draftHarness = null;
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
                el('div', { class: 'act' }, btn('Show paper recovery', 'key', 'primary'), el('small', {}, 'restores identity only · seats, history and grants are not recovered')),
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
        const hTile = h => {
          const v = SET.harness[h], d = SET_DEF.harness[h] || null, ch = k => !!d && v[k] !== d[k], rs = k => () => { v[k] = d[k]; emit('harness', h, v); draw(); }, set = k => x => { v[k] = x; emit('harness', h, v); draw(); };
          return el('div', { class: 's-card htile' },
            el('div', { class: 'hh' }, el('span', { class: 'tile', style: 'background:' + v.tile }, harnessIcon(v.icon || h)), el('div', { class: 'ht' }, el('b', {}, (LAUNCH.find(l => l.harness === h) || {}).label || h), el('code', {}, v.command || h)),
              v.custom ? el('button', { class: 'del', onclick: () => { delete SET.harness[h]; emit('harness', h, null); draw(); } }, 'delete') : chip('built-in', 'bi')),
            row('Default role', null, select(roleNames(), v.role, set('role')), ch('role'), rs('role')),
            row('Wake', null, select(WAKES, v.wake, set('wake')), ch('wake'), rs('wake')),
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
            row('Wake', null, select(WAKES, dh.wake, x => { dh.wake = x; })),
            row('State from', null, select(STATE_FROM, dh.stateFrom, x => { dh.stateFrom = x; })),
            row('Hooks', null, toggle(dh.hooks, x => { dh.hooks = x; draw(); })),
            el('div', { class: 's-note' }, 'unknown harnesses run through the PTY adapter: input is typed, state is inferred'),
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

  window.C = { taskRow, TASK_STATES, TASK_GLYPH, GRADE_TITLE, KIND_MARK, settingsPage, SET, SET_DEF, LAUNCH, hslToRgb, logo, logoSvg, syncPulse, hash32, rng, seatColor, rollName, HARNESSES, DEFAULT_ROLE, STATES, GLYPH, ROLE_MARK, ICON_SVG, glyph, harnessIcon, roleMark, kbd, makeSeat, makeTree, leaves, makeModel, rollup, seatRow, miniGrid, tabRow, spaceHeader, sidebar, titleStrip, pane, picker, messageRow };
})();
