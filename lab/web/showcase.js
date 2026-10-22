// Showcase: full-window mock. Sidebar + a split tree of panes in the DOM; the border glow is a WebGL2 fragment
// shader on a full-window canvas behind the DOM. Per seat: rect, colour, state, focus. Particles advect along the
// border; ripples expand from a sidebar row and the matching pane corner. A timer keeps the scene alive.
// ?scene=15 renders fifteen seats in one tab as a deep uneven split tree. Theme follows the OS unless toggled.
(function () {
  const T = window.TOKENS, G = T.glow, el = Lab.el;
  const q = new URLSearchParams(location.search);
  const SCENE = q.get('scene') === '15' ? 15 : 3;
  const GUTTER = q.has('gutter') ? Math.max(0, +q.get('gutter')) : T.layout.gutter;

  // ---------- theme: auto (prefers-color-scheme) | dark | light ----------
  let density = 'cozy';   // live from Settings › Appearance
  const mq = matchMedia('(prefers-color-scheme: dark)');
  let themeMode = Lab.store.get('showcase.theme') || 'auto';
  const theme = () => themeMode === 'auto' ? (mq.matches ? 'dark' : 'light') : themeMode;
  function applyTheme() { Lab.prefs.theme = theme(); Lab.prefs.density = density; Lab.applyTokens(); if (!document.documentElement.style.getPropertyValue('--gutter')) document.documentElement.style.setProperty('--gutter', GUTTER + 'px'); }
  mq.addEventListener('change', () => { if (themeMode === 'auto') { applyTheme(); render(); } });
  applyTheme();

  // ---------- model ----------
  const r = C.rng('showcase' + SCENE);
  const PAL = ['#FFC53D', '#FF4FA3', '#B07BFF', '#2EE6A8', '#2E6BFF', '#FF7A45', '#58D7FF', '#9AA3C7', '#F2D16B', '#5BE08C', '#E96BFF', '#FF6B6B', '#4FD1FF', '#C9A96B', '#8AF0D0'];
  const PAL_LIGHT = ['#B8860B', '#C2185B', '#6A3FD1', '#0E8F6A', '#1E4FD1', '#D1541C', '#0E7FA8', '#5B6480', '#9A7B10', '#1F8F4A', '#9C1FBF', '#C63B3B', '#0E86B8', '#8A6B2E', '#1E9C7E'];
  const mk = (i, name, harness, role, title, state) => ({ key: 'sc' + i, name, harness, role, title, state, pal: i });
  const BASE = [
    mk(0, 'Vesper Greywind', 'claude', 'oracle', 'swarm control', 'active'),
    mk(1, 'Nyx Copperwing', 'codex', 'executor', 'messaging spec', 'active'),
    mk(2, 'Lumen Foxglove', 'claude', 'creativity', 'ux lab', 'idle'),
    Object.assign(mk(3, 'Rook Ashwhisker', 'codex', 'executor', 'gpu research', 'waiting'), { where: 'cloud', host: 'eu-2' }),
    Object.assign(mk(4, 'Sable Quill', 'codex', 'executor', 'harness hooks', 'idle'), { where: 'ssh', host: 'rig' }),
    mk(5, 'Pip Hollow', 'pi', 'executor', 'relay bench', 'errored'),
    mk(6, 'Marrow Finch', 'omp', 'executor', 'parse logs', 'idle'),
    mk(7, 'zsh', 'zsh', null, '~/mydev', 'idle'),
    mk(8, 'Tamsin Voss', 'codex', 'executor', 'cargo test', 'active'),
    mk(9, 'Corvin Ember', 'codex', 'executor', 'review PR 14', 'idle'),
    mk(10, 'Ilka Thistle', 'pi', 'executor', 'relay bench', 'active'),
    mk(11, 'Dax Riven', 'codex', 'executor', 'mux daemon', 'idle'),
    mk(12, 'Wren Starling', 'omp', 'executor', 'tokens', 'waiting'),
    Object.assign(mk(13, 'Halcyon Moss', 'codex', 'executor', 'ssh rig', 'idle'), { where: 'ssh', host: 'rig' }),
    mk(14, 'Juniper Vale', 'claude', 'executor', 'parse logs', 'active'),
  ];
  const seats = BASE.slice(0, SCENE === 15 ? 15 : 8);
  // grade follows the harness record's state source (Settings › Harnesses): osc7501 → reported, hooks → shim, output cadence → inferred
  const GRADE_OF = { osc7501: 'reported', hooks: 'shim', 'output cadence': 'inferred' };
  const gradeFor = h => { const rec = C.SET.harness[h]; return rec ? (GRADE_OF[rec.stateFrom] || (rec.hooks ? 'shim' : 'inferred')) : 'inferred'; };
  for (const s of seats) Object.defineProperty(s, 'grade', { get() { return gradeFor(this.harness); }, enumerable: true });
  // seat colour through the operator-override path, per theme
  Object.defineProperty(Object.prototype, '__noop', { value: 0, configurable: true }); delete Object.prototype.__noop;
  for (const s of seats) Object.defineProperty(s, 'color', { get() { return (theme() === 'light' ? PAL_LIGHT : PAL)[this.pal]; }, enumerable: true });

  const S = i => seats[i];
  const V = (ratio, a, b) => ({ dir: 'v', ratio, a, b }), H = (ratio, a, b) => ({ dir: 'h', ratio, a, b });
  let model, stageTree;
  if (SCENE === 15) {
    stageTree = V(0.36, H(0.4, S(0), H(0.5, S(1), S(2))),
      H(0.5, V(0.3, S(3), V(0.5, S(4), V(0.5, S(5), S(6)))),
        V(0.25, H(0.5, S(7), S(8)), V(0.33, H(0.5, S(9), S(10)), V(0.5, H(0.5, S(11), S(12)), H(0.5, S(13), S(14)))))));
    model = { spaces: [{ name: 'swarm', key: 'space-swarm', tabs: [{ name: 'all fifteen', shortcut: '⌘1', tree: stageTree, seats }] }] };
  } else {
    stageTree = V(0.6, S(0), H(0.5, S(1), S(3)));
    model = { spaces: [
      { name: 'term', key: 'space-term', tabs: [
        { name: 'core', shortcut: '⌘1', tree: stageTree, seats: [S(0), S(1), S(3)] },
        { name: 'ux lab', shortcut: '⌘2', tree: S(2), seats: [S(2)] },
        { name: 'hooks', shortcut: '⌘3', tree: S(4), seats: [S(4)] },
      ] },
      { name: 'winnow', key: 'space-winnow', tabs: [
        { name: 'relay', shortcut: '⌘4', tree: V(0.5, S(5), S(6)), seats: [S(5), S(6)] },
        { name: 'shell', shortcut: '⌘5', tree: S(7), seats: [S(7)] },
      ] },
    ] };
  }
  // Phantoms (ux-pick 'Phantom seats'): other people's harnesses in the cohort's channels. Grey, present, no pane.
  const GREY = { dark: '#8A90B0', light: '#6B7190' };
  const phantoms = [
    { key: 'ph0', name: 'Kestrel Nine', harness: 'claude', role: 'oracle', title: '', state: 'active', phantom: true, owner: 'mara' },
    { key: 'ph1', name: 'Orrin Slate', harness: 'codex', role: 'executor', title: '', state: 'idle', phantom: true, owner: 'mara' },
    { key: 'ph2', name: 'Ione Marsh', harness: 'pi', role: 'executor', title: '', state: 'waiting', phantom: true, owner: 'dov' },
  ];
  for (const ph of phantoms) Object.defineProperty(ph, 'color', { get() { return GREY[theme()]; }, enumerable: true });
  model.spaces[0].participants = phantoms;
  const everyone = [...seats, ...phantoms];
  let streamFilter = null;
  // A space is a cohort: its own seats, participants and stream. One shows at a time; the footer dots switch.
  const open = new Set([model.spaces[0].key]);   // which cohort blocks are open: band click toggles, focus never touches it
  for (const sp of model.spaces) { sp.participants = sp.participants || []; sp.stream = []; sp.epics = []; sp.history = []; }
  const now = () => new Date().toTimeString().slice(0, 5);
  model.spaces[0].history.push(...FIX.history.map(x => ({ ...x })));
  seats[1].touched = 2; seats[3].touched = 1;
  // running subagents nest under their seat (Claude via OSC 7501 id/title, Codex via subagent hooks); ended ones leave the sidebar
  seats[0].subagents = [{ id: 'a1', description: 'Explore render goldens', state: 'active', detail: 'Grep · bbt-render' }, { id: 'a2', description: 'Review W3.2a diff', state: 'waiting', kind: 'permission', detail: 'Bash · cargo test' }];
  seats[1].subagents = [{ id: 'c1', description: 'worker: ring checksum', state: 'active' }];
  seats[3].gap = { from: '09:14:07' };  // a long-running cloud seat whose ring evicted its oldest output
  seats[0].listeners = 2; seats[1].listeners = 1;  // clients on the seat socket: counts only
  // subscriptions and external events (events.md): auto-subscriptions follow delegation; external arrives from webhooks, CI, cloud, cron
  model.spaces[0].subscriptions = FIX.subscriptions.map(x => ({ ...x }));
  model.spaces[0].external = FIX.external.map(x => ({ ...x }));
  let extOpen = true;
  // provenance fixture: items with weighted derived_from in the annotation form, signed digests, a hash-chained log
  model.spaces[0].provenance = JSON.parse(JSON.stringify(FIX.provenance));
  model.spaces[0].epics.push(
    { id: 'e1', title: 'W3 renderer', open: true, items: [
      { id: 's1', type: 'spec', title: 'renderer.md', state: 'accepted' },
      { id: 'w31', type: 'task', title: 'W3.1 renderer harness', state: 'reported', assignee: 'Nyx Copperwing', artifacts: [{ kind: 'pr', ref: 'bboxy450-term#14', digest: 'fcc8eca', produced_by: 'Nyx Copperwing', responsible: 'Nyx Copperwing', responsibleColor: '#FF4FA3' }, { kind: 'image', ref: 'target/render/smoke/jetbrains-mono-16px-2x-dark.png', digest: '9b1c2e4', produced_by: 'CI' }] },
      { id: 'w32', type: 'task', title: 'W3.2 GUI component library', state: 'blocked', waits: 'W3.1' },
      { id: 'w33', type: 'task', title: 'W3.3 showcase parity', state: 'open' } ] },
    { id: 'e2', title: 'W1 term core', open: false, items: [
      { id: 'w11', type: 'task', title: 'W1.1 term-core fork', state: 'done', assignee: 'Nyx Copperwing' },
      { id: 'w12', type: 'task', title: 'W1.2 byte ring', state: 'open' } ] });
  function space() { const hit = tabOf(focused); return hit ? hit.sp : model.spaces[0]; }   // the cohort of the focused seat
  function tabOf(key) { for (const sp of model.spaces) for (const t of sp.tabs) if ((t.seats || []).some(x => x.key === key)) return { sp, t }; return null; }
  function stageOf() { const sp = space(); const hit = tabOf(focused); const t = hit && hit.sp === sp ? hit.t : sp.tabs[0]; return t ? t.tree : null; }
  let stageKeys = C.leaves(stageTree).map(s => s.key);
  let focused = stageKeys[0];
  function jumpSpace(sp) { open.add(sp.key); const first = sp.tabs[0] && sp.tabs[0].seats[0]; if (first) focused = first.key; render(); }   // dot: open it and focus its first seat
  function toggleSpace(sp) { open.has(sp.key) ? open.delete(sp.key) : open.add(sp.key); render(); }                                    // band: open/shut only
  let spaceN = model.spaces.length, editName = null;
  function newSpace() {
    spaceN++;
    const z = { key: 'z' + spaceN, name: 'zsh', harness: 'zsh', role: null, title: '~', state: 'idle', pal: (spaceN + 6) % 15 };
    Object.defineProperty(z, 'color', { get() { return (theme() === 'light' ? PAL_LIGHT : PAL)[this.pal]; }, enumerable: true });
    Object.defineProperty(z, 'grade', { get() { return gradeFor(this.harness); }, enumerable: true });
    seats.push(z); startReplay(z, seats.length - 1);
    const sp = { name: 'swarm ' + spaceN, key: 'space-' + spaceN, tabs: [{ name: 'shell', shortcut: '⌘1', tree: z, seats: [z] }], participants: [], stream: [], history: [], external: [], epics: [{ id: 'e' + spaceN, title: 'swarm ' + spaceN, open: true, items: [] }] };
    model.spaces.push(sp); editName = sp.key; jumpSpace(sp);
  }
  // Placement: a seat goes into a swarm as a new tab, or splits a target pane on one of its four sides.
  // A seat already in a tree is pulled out of it first (its old split collapses), so drag = move.
  function detach(seat) {
    for (const sp of model.spaces) for (const tab of sp.tabs) if (tab.seats.includes(seat)) { tab.seats = tab.seats.filter(x => x !== seat); tab.tree = prune(tab.tree, seat.key); if (!tab.seats.length) sp.tabs.splice(sp.tabs.indexOf(tab), 1); }
  }
  function placeSeat(seat, where, targetKey) {
    const target = targetKey ? seats.find(x => x.key === targetKey) : null;
    if (seat === target) return;
    if (where === 'tab' || !target) { const sp = space(); detach(seat); sp.tabs.push({ name: seat.harness === 'zsh' ? 'shell' : seat.name.split(' ')[0].toLowerCase(), shortcut: sp.tabs.length < 9 ? '⌘' + (sp.tabs.length + 1) : '', tree: seat, seats: [seat] }); open.add(sp.key); }
    else {
      const hit = tabOf(target.key); if (!hit) return;
      detach(seat);
      const hit2 = tabOf(target.key); const tab = hit2.t;
      const dir = (where === 'left' || where === 'right') ? 'v' : 'h', first = where === 'left' || where === 'top';
      const node = { dir, ratio: 0.5, a: first ? seat : target, b: first ? target : seat };
      tab.tree = replaceNode(tab.tree, target, node); tab.seats.push(seat); open.add(hit2.sp.key);
    }
    focused = seat.key; render(); locate(seat.key, 'both');
  }
  // Mute (operator, vision 'Cohort scope and mute'): a muted seat leaves the swarm stream and works 1:1 with the operator;
  // its row stays where it is with a muted mark; mute and unmute each post a presence notice to the whole swarm.
  function toggleMute(seat) {
    seat.muted = !seat.muted;
    const t = tabOf(seat.key); const sp = t ? t.sp : space();
    // unmute commits its notice first, then releases only eligible held work; blocked items stay with reason and original id
    let rel = '';
    if (!seat.muted && (seat.held || []).length) { const before = seat.held.length; seat.held = seat.held.filter(h => h.blocked); const n = before - seat.held.length; rel = ' · released ' + n + (seat.held.length ? ', ' + seat.held.length + ' still held' : ''); if (n) { seat.unread = (seat.unread || 0) + n; seat.unreadRole = seat.unreadRole || 'executor'; } }
    sp.stream.push({ presence: true, text: seat.name + (seat.muted ? ' muted · 1:1 with the operator' : ' unmuted · back in the swarm' + rel), time: new Date().toTimeString().slice(0, 5) });
    render();
  }
  addEventListener('keydown', e => { if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'm' && !spot) { e.preventDefault(); const f = seats.find(x => x.key === focused); if (f) toggleMute(f); } });
  // Stop (seat_interrupt): the operator may interrupt any seat they can act on; a one-shot "interrupt sent" on strip and row and a red
  // sweep. It is a request: the seat's state changes only when the seat reports it. ⌘. interrupts the focused seat.
  function interruptSeat(seat) { if (seat.interruptSent) return; seat.interruptSent = Date.now(); render(); setTimeout(() => { seat.interruptSent = null; render(); }, 3000); }
  addEventListener('keydown', e => { if (e.metaKey && e.key === '.' && !spot) { e.preventDefault(); const f = seats.find(x => x.key === focused); if (f && !f.settings && !f.graph) interruptSeat(f); } });
  // Row menu: right-click, ⌘-click or long-press a seat row; ⇧F10 on the focused seat. Never hover-only. The operator steers.
  let rmenu = null;
  function closeRowMenu() { if (rmenu) { rmenu.remove(); rmenu = null; } }
  function openRowMenu(seat, x, y) {
    closeRowMenu();
    rmenu = C.rowMenu(seat, { steer: true, roles: Object.keys(C.SET.roles), onClose: closeRowMenu, onAction: (a, sd, v) => {
      const sp = (tabOf(sd.key) || {}).sp || space();
      if (a === 'focus') setFocus(sd.key, { locate: 'both' });
      else if (a === 'split-right' || a === 'split-down') { setFocus(sd.key); openSpotlight(null, a === 'split-right' ? 'right' : 'bottom'); }
      else if (a === 'role') { sd.role = v; streamLine(sp, 'operator assigned ' + v + ' to ' + sd.name); render(); }
      else if (a === 'sends') { sd.sends = v; streamLine(sp, sd.name + ' may send to ' + v); render(); }
      else if (a === 'interrupt') interruptSeat(sd);
      else if (a === 'mute') toggleMute(sd);
      else if (a === 'close') closeSeat(sd);
      else if (a === 'purge') openPurge(sd);
    } });
    rmenu.style.position = 'fixed'; rmenu.style.left = Math.min(x, innerWidth - 250) + 'px'; rmenu.style.top = Math.min(y, innerHeight - 380) + 'px';
    document.body.append(rmenu); rmenu.focus();
  }
  // Purge (GUI only): the one confirm. Typed seat name; skips grace; history to cold storage; a stream line says so.
  function openPurge(sd) {
    closeSpotlight();
    const back = el('div', { class: 'spot-back', onclick: e => { if (e.target === back) closeSpotlight(); } });
    const m = C.purgeModal(sd, { graceS: GRACE_MS / 1000, onCancel: closeSpotlight, onPurge: x => {
      closeSpotlight(); const hit = tabOf(x.key); const sp = hit ? hit.sp : space();
      if (hit) { hit.t.seats = hit.t.seats.filter(y => y !== x); hit.t.tree = prune(hit.t.tree, x.key); if (!hit.t.seats.length) sp.tabs.splice(sp.tabs.indexOf(hit.t), 1); }
      seats.splice(seats.indexOf(x), 1); clearTimeout(x.replay && x.replay.timer);
      if (focused === x.key) { const t2 = sp.tabs[0]; focused = t2 ? t2.seats[0].key : null; }
      streamLine(sp, 'operator purged ' + x.name + ' · history moved to cold storage'); render(); } });
    back.append(m); document.body.append(back); spot = back; m.focus();
  }
  // Subagent rows are message targets (messaging.md Subagent addressing). Claude and Codex cannot address subagents, so the
  // message goes to the main agent with a routing header: the row says "via main", the ladder is per target, the reply is relayed.
  function openAgentMenu(sd, ag, x, y) {
    closeRowMenu();
    rmenu = C.rowMenu(sd, { agent: ag, addressable: false, onClose: closeRowMenu, onAction: (a) => {
      const sp = (tabOf(sd.key) || {}).sp || space();
      if (a === 'focus') return setFocus(sd.key, { locate: 'both' });
      if (a !== 'message-agent' && a !== 'message-main') return;
      const toAgent = a === 'message-agent', msg = { from: 'operator', role: 'oracle', time: new Date().toTimeString().slice(0, 5), text: toAgent ? 'how far along is this?' : 'when you are free, summarise the subagents', to: toAgent ? ag.description : 'main', via: toAgent ? 'main' : null, evidence: 'sender-stored' };
      sp.stream.push(msg); render();
      ['recipient-stored', 'native-written'].forEach((ev, i) => setTimeout(() => { msg.evidence = ev; render(); }, 500 + i * 700));
      if (toAgent) setTimeout(() => { sp.stream.push({ from: sd.name, fromKey: sd.key, role: sd.role, time: new Date().toTimeString().slice(0, 5), text: 'for ' + ag.description + ': two thirds done', via: 'main', unread: true }); render(); }, 2600);
    } });
    rmenu.style.position = 'fixed'; rmenu.style.left = Math.min(x, innerWidth - 260) + 'px'; rmenu.style.top = Math.min(y, innerHeight - 220) + 'px';
    document.body.append(rmenu); rmenu.focus();
  }
  addEventListener('contextmenu', e => { const r = e.target.closest && e.target.closest('.sa-row'); if (!r || !r.dataset.agent) return; const sd = seats.find(x => x.key === r.dataset.seat), ag = sd && (sd.subagents || []).find(a => a.id === r.dataset.agent); if (!ag) return; e.preventDefault(); e.stopImmediatePropagation(); openAgentMenu(sd, ag, e.clientX, e.clientY); }, true);
  const rowSeat = t => { const r = t.closest && t.closest('.seat-row'); return r && seats.find(x => x.key === r.dataset.key); };
  addEventListener('contextmenu', e => { const sd = rowSeat(e.target); if (!sd) return; e.preventDefault(); openRowMenu(sd, e.clientX, e.clientY); });
  addEventListener('click', e => { if (rmenu && !rmenu.contains(e.target)) { closeRowMenu(); return; } if (!e.metaKey) return; const sd = rowSeat(e.target); if (sd) { e.preventDefault(); e.stopPropagation(); openRowMenu(sd, e.clientX, e.clientY); } }, true);
  let lp = null;
  addEventListener('pointerdown', e => { const sd = rowSeat(e.target); if (!sd || e.button !== 0) return; const x = e.clientX, y = e.clientY; lp = setTimeout(() => { lp = null; openRowMenu(sd, x, y); }, 550); });
  addEventListener('pointerup', () => { if (lp) { clearTimeout(lp); lp = null; } });
  addEventListener('pointermove', e => { if (lp && (Math.abs(e.movementX) + Math.abs(e.movementY)) > 4) { clearTimeout(lp); lp = null; } });
  addEventListener('keydown', e => { if (e.shiftKey && e.key === 'F10' && !spot) { e.preventDefault(); const f = seats.find(x => x.key === focused); const r = f && document.querySelector('.seat-row[data-key="' + f.key + '"]'); if (r) { const b = r.getBoundingClientRect(); openRowMenu(f, b.left + 40, b.bottom + 2); } } });
  // Spawn cap per swarm (launcher_cap_notice): running, starting (pending) and grace (closed) seats all count; Go to still works.
  const SWARM_CAP = 8;
  function capOf(sp) { sp = sp || space(); const run = sp.tabs.reduce((n, t) => n + (t.seats || []).filter(x => !x.settings && !x.graph).length, 0), start = (sp.pending || []).filter(p => p.status === 'accepted').length, grace = (sp.closed || []).length, used = run + start + grace;
    return used >= SWARM_CAP ? { scope: 'swarm', name: sp.name, used, limit: SWARM_CAP, counts: run + ' running · ' + start + ' starting · ' + grace + ' in grace' } : null; }
  // V: the document modal (document-viewer.md). Open, scroll, copy maybe, close, feed into the harness. ⏎ feeds the document
  // into the focused seat as an operator message (the tick ladder climbs), closes, and focus returns where it was.
  let docN = 0;
  function openDoc(doc) {
    closeSpotlight();
    const back = el('div', { class: 'spot-back', onclick: e => { if (e.target === back) closeSpotlight(); } });
    const f = seats.find(x => x.key === focused), was = focused;
    const m = C.docModal(doc, { seat: f, onClose: () => { closeSpotlight(); focused = was; render(); }, onPin: () => closeSpotlight(), onFeed: (d, sel) => {
      closeSpotlight(); focused = was; if (!f) return;
      const sp = (tabOf(f.key) || {}).sp || space(), msg = { from: 'operator', role: 'oracle', time: new Date().toTimeString().slice(0, 5), text: '→ ' + f.name + ': ' + (sel ? 'selection from ' : '') + '[[' + d.path.replace(/^.*\//, '').replace(/\.md$/, '') + ']]', evidence: 'sender-stored' };
      sp.stream.push(msg); render();
      ['recipient-stored', 'native-written'].forEach((ev, i) => setTimeout(() => { msg.evidence = ev; render(); }, 500 + i * 700));
    } });
    back.append(m); document.body.append(back); spot = back; m.focus();
  }
  addEventListener('keydown', e => { if ((e.key === 'v' || e.key === 'V') && !e.metaKey && !e.ctrlKey && !e.altKey && !spot) { const d = docN++ % 2 ? FIX.docs.readme : FIX.docs.messaging; openDoc(d); } });
  // O: the Oracle spawns a seat over MCP. A dashed pending row with a spinner until applied, then the seat takes a tab of its own.
  // Demo cycle: every fourth spawn is rejected (reason shown), every fifth ends unknown (spelled, not retried).
  let spawnN = 0;
  addEventListener('keydown', e => {
    if ((e.key !== 'o' && e.key !== 'O') || e.metaKey || e.ctrlKey || e.altKey || spot) return;
    const sp = space(), n = ++spawnN, h = ['codex', 'claude', 'pi', 'codex'][n % 4];
    const p = { op: C.hash32('op' + n).toString(16).slice(0, 8), name: C.rollName(C.rng('spawn' + n)), harness: h, by: 'Vesper Greywind', status: 'accepted' };
    sp.pending = sp.pending || []; sp.pending.push(p); open.add(sp.key); render();
    setTimeout(() => {
      const c = capOf(sp); if (c && c.used > SWARM_CAP) { p.status = 'rejected'; p.reason = 'spawn cap for this swarm is ' + SWARM_CAP + ' (' + c.counts + ')'; render(); return; }
      if (n % 4 === 0) { p.status = 'rejected'; p.reason = 'spawn cap for this swarm is 8'; render(); return; }
      if (n % 5 === 0) { p.status = 'unknown'; render(); return; }
      sp.pending = sp.pending.filter(x => x !== p);
      const seat = { key: 'o' + n + Date.now().toString(36), name: p.name, harness: h, role: C.DEFAULT_ROLE[h], title: '', state: 'active', pal: (seats.length + 3) % 15, where: 'local' };
      Object.defineProperty(seat, 'color', { get() { return (theme() === 'light' ? PAL_LIGHT : PAL)[this.pal]; }, enumerable: true });
      Object.defineProperty(seat, 'grade', { get() { return gradeFor(this.harness); }, enumerable: true });
      seats.push(seat); startReplay(seat, seats.length - 1); placeSeat(seat, 'tab');
      streamLine(sp, 'Oracle spawned ' + seat.name + ' · ' + h);
    }, 1800);
  });
  // D: the GUI loses the daemon. Every pane keeps its stale frame, dimmed, glow frozen, "reconnecting…"; on reconnect the
  // frames redraw, the pill fades, and a seat whose process generation changed shows "restarted" for 4 s, then a stream line.
  let daemon = null;
  addEventListener('keydown', e => {
    if ((e.key !== 'd' && e.key !== 'D') || e.metaKey || e.ctrlKey || e.altKey || spot || daemon) return;
    daemon = 'lost'; render();
    setTimeout(() => {
      const s = seats[1]; s.generation = (s.generation || 3) + 1; s.restarted = Date.now(); daemon = 'back'; render();
      setTimeout(() => { daemon = null; render(); }, 1000);
      setTimeout(() => { s.restarted = false; const t = tabOf(s.key); streamLine(t ? t.sp : space(), s.name + ' restarted · process generation ' + s.generation); render(); }, 4000);
    }, 2600);
  });
  // Appearance settings apply live: density, theme and gutter re-render the window; glow toggles the shader canvas.
  function applySetting(section, key, value) {
    if (section !== 'appearance') return;
    if (key === 'density') { density = value; }
    if (key === 'theme') { themeMode = value; Lab.store.set('showcase.theme', value); drawThemeBtn(); }
    if (key === 'gutter') { document.documentElement.style.setProperty('--gutter', value + 'px'); }
    if (key === 'glow') { document.getElementById('glow').style.display = value ? '' : 'none'; }
    const keep = settingsSeat.section; applyTheme(); settingsSeat.section = keep; render();
  }
  // key E: an external event arrives (GitHub / CI / cloud / cron cycle)
  const EXT_DEMO = [['github', 'PR #15 opened · W2.1 mux supervisor'], ['ci', 'macos-15 · workspace tests · 134 passed'], ['cloud', 'us-1 · provider quota at 80%'], ['webhook', 'operator webhook · deploy window opens']];
  let extN = 0;
  addEventListener('keydown', e => { if ((e.key === 'e' || e.key === 'E') && !e.metaKey && !e.ctrlKey && !spot) { const [source, text] = EXT_DEMO[extN++ % EXT_DEMO.length]; (space().external = space().external || []).push({ source, text, time: new Date().toTimeString().slice(0, 5) }); drawStream(); } });
  // Tasks (operator: task_delegate): a durable task bound to a seat; a row under the seat and a line in the stream.
  // Key T delegates a demo task to the focused seat; T again walks it: assigned → acknowledged → running → reported; accept via the pill.
  const TASK_TITLES = ['gpu research spec', 'relay bench on eu-2', 'review PR 14', 'fix glyphon atlas growth', 'write harness adapter'];
  let taskN = 0;
  function streamLine(sp, text) { sp.stream.push({ presence: true, text, time: new Date().toTimeString().slice(0, 5) }); }
  function delegateTask(seat) {
    const sp = (tabOf(seat.key) || {}).sp || space();
    seat.tasks = seat.tasks || [];
    const open_ = seat.tasks.find(t => !['accepted', 'rejected', 'reassigned'].includes(t.state));
    if (open_) {
      const next = { open: 'claimed', claimed: 'running', running: 'reported', reported: 'reported' }[open_.state];
      if (next !== open_.state) {
        open_.state = next; streamLine(sp, seat.name + ' ' + next + ' · ' + open_.title);
        if (next === 'reported') { open_.artifacts = open_.artifacts || []; const art = { kind: 'markdown', ref: '.bboxy/runs/' + open_.title.replace(/\s+/g, '-') + '-report.md', digest: C.hash32(open_.title + seat.key).toString(16).padStart(8, '0'), produced_by: seat.name, responsible: seat.name, responsibleColor: seat.color }; open_.artifacts.push(art); seat.touched = (seat.touched || 0) + 1; sp.history.push({ time: now(), seat: seat.name, seatColor: seat.color, event: 'created', kind: art.kind, ref: art.ref, grade: seat.grade === 'inferred' ? 'inferred' : 'reported' }, { time: now(), seat: seat.name, seatColor: seat.color, event: 'claimed', kind: art.kind, ref: art.ref, grade: 'reported' }); streamLine(sp, seat.name + ' claimed · ' + art.ref); }
      }
    } else {
      const task = { id: 't' + (++taskN), title: TASK_TITLES[taskN % TASK_TITLES.length], state: 'claimed', from: 'Vesper Greywind', assignee: seat.name };
      seat.tasks.push(task); sp.epics[0].items.push(task); streamLine(sp, 'Oracle delegated · ' + task.title + ' → ' + seat.name);
    }
    render();
  }
  function acceptTask(task, seat) { task.state = 'accepted'; const sp = (tabOf(seat.key) || {}).sp || space(); streamLine(sp, 'Oracle accepted · ' + task.title + ' from ' + seat.name); render(); }
  addEventListener('keydown', e => { if ((e.key === 't' || e.key === 'T') && !e.metaKey && !e.ctrlKey && !spot) { const f = seats.find(x => x.key === focused); if (f && !f.settings && !f.graph) delegateTask(f); } });
  // Graph: who is on what and the work tree, as a tab (⌘G or the Graph launcher row). Generated from the same data as the dock.
  const graphSeat = { key: 'graph', name: 'Graph', harness: 'sidebarfoxopen', role: null, title: 'who is on what', state: 'idle', graph: true };
  Object.defineProperty(graphSeat, 'color', { get() { return T.color[theme()].accent_soft; }, enumerable: true });
  function openGraph() { const sp = space(); if (!seats.includes(graphSeat)) seats.push(graphSeat); if (!tabOf('graph')) sp.tabs.push({ name: 'graph', shortcut: '⌘G', tree: graphSeat, seats: [graphSeat] }); open.add(sp.key); focused = 'graph'; render(); }
  function closeGraph() { detach(graphSeat); seats.splice(seats.indexOf(graphSeat), 1); const t = space().tabs[0]; focused = t ? t.seats[0].key : null; render(); }
  addEventListener('keydown', e => { if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'g') { e.preventDefault(); openGraph(); } });
  // Settings open as a tab in the current swarm (⌘, or the Settings row in the launcher); closing it removes the tab, no grace.
  const settingsSeat = { key: 'settings', name: 'Settings', harness: 'gear', role: null, title: 'config.toml', state: 'idle', settings: true, section: 'Account' };
  Object.defineProperty(settingsSeat, 'color', { get() { return T.color[theme()].accent_soft; }, enumerable: true });
  function openSettings(section) {
    settingsSeat.section = section || 'Account';   // opening settings always lands on Account unless a section was asked for
    const sp = space();
    if (!seats.includes(settingsSeat)) seats.push(settingsSeat);
    if (!tabOf('settings')) sp.tabs.push({ name: 'settings', shortcut: '⌘,', tree: settingsSeat, seats: [settingsSeat] });
    open.add(sp.key); focused = 'settings'; render();
  }
  function closeSettings() { detach(settingsSeat); seats.splice(seats.indexOf(settingsSeat), 1); const t = space().tabs[0]; focused = t ? t.seats[0].key : null; render(); }
  addEventListener('keydown', e => { if ((e.metaKey || e.ctrlKey) && e.key === ',') { e.preventDefault(); openSettings(); } });
  // Spotlight launcher (operator): '+ new seat' or ⌘T opens a centred search over the window; Enter launches into the
  // swarm as a new tab and focuses it. The list is data (C.LAUNCH): harnesses and zsh now, browser / vnc / ssh later.
  let spot = null;
  // preset: 'right' (⌘D) or 'bottom' (⌘⇧D) presets the placement, iTerm style; the launcher then asks which harness
  function openSpotlight(sp, preset) {
    closeSpotlight();
    sp = sp || space();
    const back = el('div', { class: 'spot-back', onclick: e => { if (e.target === back) closeSpotlight(); } });
    const anchor = focused, anchorSeat = seats.find(x => x.key === anchor);   // ⌘↵ splits right of the focused pane, ⌘⇧↵ below it, ↵ opens a new tab
    const ph = preset && anchorSeat ? 'split ' + (preset === 'right' ? 'right of ' : 'below ') + anchorSeat.name + ' · pick a harness' : 'launch or go to · type to search';
    const actions = preset ? [[preset === 'right' ? 'Split right' : 'Split down', '↵'], ['New tab instead', '⌥↵'], ['Reroll', '⌘R']] : [['Launch / go', '↵'], ['Split right', '⌘↵'], ['Split down', '⌘⇧↵'], ['Reroll', '⌘R']];
    const items = C.launcherItems({ seats, spaces: model.spaces, sp, anchorSeat, tabOf });
    const acts = p => (p.length && p[p.length - 1].id === 'goto') ? [['Go', '↵'], ['Back', '←']] : (p.length ? [['Place', '↵'], ['Back', '←']] : (preset ? [[preset === 'right' ? 'Split right' : 'Split down', '↵'], ['Placements', '→'], ['New tab instead', '⌥↵']] : [['New tab', '↵'], ['Placements', '→'], ['Split right', '⌘↵'], ['Split down', '⌘⇧↵']]));
    // banner above the panel: the real pane layout, large, with the new cell lit; nothing subtle about it
    if (preset && anchorSeat && stageTree) {
      const ghost = { key: '__new', name: 'new seat', state: 'idle' };
      const previewTree = replaceNode(stageTree, anchorSeat, { dir: preset === 'right' ? 'v' : 'h', ratio: 0.5, a: anchorSeat, b: ghost });
      const grid = C.miniGrid(previewTree, '__new', true);
      back.append(el('div', { class: 'spot-banner' }, grid, el('div', { class: 'txt' }, el('div', { class: 'big' }, preset === 'right' ? 'Split right' : 'Split down'), el('div', { class: 'of' }, 'of ', el('b', { style: 'color:' + anchorSeat.color }, anchorSeat.name), ' · the new seat lands in the lit cell'))));
    } else if (!preset) {
      // ⌘T: a new tab. The banner shows the swarm's tabs as a strip with the new one lit at the end.
      // drawn the way the sidebar draws it: a mini swarm block, band on top, tab rows stacked, the new row lit
      // the real rows, as the sidebar draws them: tab rows with their seats, then the lit new row at the insertion point
      const rows = [];
      for (const t of sp.tabs) {
        if (t.seats.length === 1) rows.push({ txt: t.seats[0].name, color: t.seats[0].color, seat: true });
        else { rows.push({ txt: t.name }); for (const x of t.seats) rows.push({ txt: x.name, color: x.color, seat: true, indent: true }); }
      }
      const strip = el('div', { class: 'tab-stack', style: '--space:' + sp.color }, el('span', { class: 'tband' }, sp.name));
      const MAX = 7, shown = rows.length > MAX ? [rows[0], { more: rows.length - MAX + 1 }, ...rows.slice(-(MAX - 2))] : rows;
      for (const r of shown) strip.append(r.more ? el('span', { class: 'tcell more' }, '… ' + r.more + ' more') : el('span', { class: 'tcell' + (r.indent ? ' indent' : '') + (r.seat ? ' seatrow' : ''), style: r.color ? 'color:' + r.color : '' }, r.txt));
      strip.append(el('span', { class: 'tcell new' }, '+ new seat · here'));
      strip.append(el('span', { class: 'tcell ghost' }, '+ new seat'));
      back.append(el('div', { class: 'spot-banner' }, strip, el('div', { class: 'txt' }, el('div', { class: 'big' }, 'New tab'), el('div', { class: 'of' }, 'in ', el('b', { style: 'color:' + sp.color }, sp.name), ' · the new seat gets a tab of its own · type goto <name> to go somewhere'))));
    }
    const pk = C.picker({ placeholder: ph, actions: acts, items, cap: capOf(sp), onClose: closeSpotlight, onPick: (it, name, mods, path) => {
      if (it.graphOpen) { closeSpotlight(); openGraph(); return; }
      if (it.settingsOpen) { closeSpotlight(); openSettings(it.section); return; }
      if (it.seat) { closeSpotlight(); const t = tabOf(it.seat.key); if (t) open.add(t.sp.key); setFocus(it.seat.key, { locate: 'both' }); return; }
      if (it.swarm) { closeSpotlight(); jumpSpace(it.swarm); return; }
      const l = it.launch || it;
      const where = it.where || (path.find(x => x.where) || {}).where || 'local', host = it.host || (path.find(x => x.host) || {}).host;
      const seat = { key: 'n' + Date.now().toString(36), name: l.harness === 'zsh' ? 'zsh' : name, harness: l.harness, role: C.DEFAULT_ROLE[l.harness], title: l.harness === 'zsh' ? '~' : '', state: 'active', pal: (seats.length + 3) % 15, where, host };
      Object.defineProperty(seat, 'color', { get() { return (theme() === 'light' ? PAL_LIGHT : PAL)[this.pal]; }, enumerable: true });
      Object.defineProperty(seat, 'grade', { get() { return gradeFor(this.harness); }, enumerable: true });
      seats.push(seat); startReplay(seat, seats.length - 1);
      closeSpotlight();
      if (it.place) { if (it.place === 'tab' || !anchor) placeSeat(seat, 'tab'); else placeSeat(seat, it.place, anchor); return; }
      if (preset && anchor && !(mods && mods.alt)) placeSeat(seat, preset, anchor);
      else if (mods && mods.meta && anchor) placeSeat(seat, mods.shift ? 'bottom' : 'right', anchor); else placeSeat(seat, 'tab');
    } });
    back.append(pk); document.body.append(back); spot = back; pk.focus();
  }
  function closeSpotlight() { if (spot) { spot.remove(); spot = null; } main.querySelectorAll('.pane.will-right, .pane.will-bottom').forEach(n => n.classList.remove('will-right', 'will-bottom')); }
  addEventListener('keydown', e => {
    const mod = e.metaKey || e.ctrlKey, kk = e.key.toLowerCase();
    if (mod && kk === 't') { e.preventDefault(); openSpotlight(); }
    else if (mod && kk === 'd') { e.preventDefault(); openSpotlight(null, e.shiftKey ? 'bottom' : 'right'); }   // iTerm: ⌘D side by side, ⌘⇧D stacked
    else if (e.key === 'Escape' && spot) closeSpotlight();
  });
  // Close is reversible (operator): the seat enters a 60 s grace state. The pane leaves the split, the row becomes a
  // closed strip with undo, ⌘Z restores it in place with its split position. After the window it is terminated.
  const GRACE_MS = 60000;
  function prune(t, key) { if (t.key) return t.key === key ? null : t; const a = prune(t.a, key), b = prune(t.b, key); if (!a) return b; if (!b) return a; return { ...t, a, b }; }
  function parentOf(t, key, parent) { if (t.key) return t.key === key ? parent : null; return parentOf(t.a, key, t) || parentOf(t.b, key, t); }
  function replaceNode(t, target, repl) { if (t === target) return repl; if (t.key) return t; return { ...t, a: replaceNode(t.a, target, repl), b: replaceNode(t.b, target, repl) }; }
  function closeSeat(seat) {
    const hit = tabOf(seat.key); if (!hit) return;
    const { sp, t: tab } = hit;
    const parent = parentOf(tab.tree, seat.key, null);
    const rec = { seat, sp, tab, tabIndex: sp.tabs.indexOf(tab), sibling: parent ? (parent.a === seat ? parent.b : parent.a) : null, side: parent ? (parent.a === seat ? 'a' : 'b') : null, dir: parent && parent.dir, ratio: parent && parent.ratio, at: performance.now() };
    tab.seats = tab.seats.filter(x => x !== seat); tab.tree = prune(tab.tree, seat.key);
    if (!tab.seats.length) sp.tabs.splice(rec.tabIndex, 1);
    sp.closed = sp.closed || []; sp.closed.push(rec);
    rec.timer = setTimeout(() => { sp.closed = sp.closed.filter(r => r !== rec); seats.splice(seats.indexOf(seat), 1); clearTimeout(seat.replay && seat.replay.timer); if (focused === seat.key) focused = null; render(); }, GRACE_MS);
    if (focused === seat.key) { const t2 = sp.tabs[0]; focused = t2 ? t2.seats[0].key : null; }
    render();
  }
  function undoClose(rec) {
    clearTimeout(rec.timer); rec.sp.closed = rec.sp.closed.filter(r => r !== rec);
    const { seat, sp, tab } = rec;
    if (!sp.tabs.includes(tab)) { tab.seats = [seat]; tab.tree = seat; sp.tabs.splice(Math.min(rec.tabIndex, sp.tabs.length), 0, tab); }
    else { tab.seats.push(seat); const node = { dir: rec.dir, ratio: rec.ratio, a: rec.side === 'a' ? seat : rec.sibling, b: rec.side === 'b' ? seat : rec.sibling }; tab.tree = replaceNode(tab.tree, rec.sibling, node); }
    focused = seat.key; render();
  }
  addEventListener('keydown', e => { if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'z') { const sp = model.spaces.find(x => x.closed && x.closed.length); if (sp) { e.preventDefault(); undoClose(sp.closed[sp.closed.length - 1]); } } });
  setInterval(() => { for (const sp of model.spaces) for (const r of (sp.closed || [])) { const n = main.querySelector('[data-closed="' + r.seat.key + '"] .left'); if (n) n.textContent = Math.max(0, Math.ceil((GRACE_MS - (performance.now() - r.at)) / 1000)) + 's'; } }, 500);

  // State change without a full render: swap glyphs and classes in place so replay text and scroll survive.
  function setState(seat, st) {
    if (seat.state === st) return;
    seat.state = st;
    for (const n of main.querySelectorAll('[data-key="' + seat.key + '"]')) {
      for (const k of C.STATES) n.classList.remove(k);
      n.classList.add(st);
      const g = n.querySelector('.glyph'); if (g) g.replaceWith(C.glyph(st, seat));
    }
  }

  // ---------- DOM ----------
  const app = document.getElementById('app');
  // one icon: sun in dark mode, moon in light mode; a click flips. The OS preference only sets the start.
  const themeBtn = el('span', { class: 'tbtn theme', title: 'toggle light / dark', onclick: () => { themeMode = theme() === 'dark' ? 'light' : 'dark'; Lab.store.set('showcase.theme', themeMode); applyTheme(); drawThemeBtn(); render(); } });
  function drawThemeBtn() { themeBtn.innerHTML = ''; themeBtn.append(C.harnessIcon(theme() === 'dark' ? 'sun' : 'moon')); }
  drawThemeBtn();
  const titlebar = C.titleBar({ sidebarOpen: true, themeBtn, crumbId: 'crumb' });
  const main = el('div', { id: 'main' });
  app.append(titlebar, main);
  let rects = [];
  function splitDom(t) {
    if (t.key && t.graph) { const sp = space(); const p = el('div', { class: 'pane settings-pane ' + (t.key === focused ? 'focused' : 'unfocused'), style: '--seat:' + t.color }, C.titleStrip(t, { shortcut: '⌘G', onClose: closeGraph }), C.graphPage({ seats: seats.filter(x => !x.settings && !x.graph && (tabOf(x.key) || {}).sp === sp), epics: sp.epics, swarm: sp.name, history: sp.history, provenance: sp.provenance, subscriptions: sp.subscriptions })); p.dataset.key = t.key; return p; }
    if (t.key && t.settings) { const p = el('div', { class: 'pane settings-pane ' + (t.key === focused ? 'focused' : 'unfocused'), style: '--seat:' + t.color }, C.titleStrip(t, { shortcut: '⌘,', onClose: closeSettings }), C.settingsPage({ section: t.section, onChange: (sec, k, v) => { if (sec === 'appearance') settingsSeat.section = 'Appearance'; applySetting(sec, k, v); } })); p.dataset.key = t.key; return p; }
    if (t.key) return C.pane(t, { focused: t.key === focused, unfocused: t.key !== focused, shortcut: '⌘' + (stageKeys.indexOf(t.key) + 1), onClose: closeSeat, onMute: toggleMute, onInterrupt: interruptSeat, reconnecting: daemon === 'lost', reconnected: daemon === 'back' });
    const A = splitDom(t.a), B = splitDom(t.b);
    A.style.flex = t.ratio || 0.5; B.style.flex = 1 - (t.ratio || 0.5);
    const box = el('div', { class: 'split ' + t.dir });
    // the gutter is the divider: drag it to change the split ratio; the tree keeps the ratio, the mini-grid follows on re-render
    const div = el('div', { class: 'divider', onpointerdown: e => {
      e.preventDefault(); div.setPointerCapture(e.pointerId); document.body.classList.add('resizing'); document.body.style.cursor = t.dir === 'v' ? 'col-resize' : 'row-resize';
      const move = ev => { const r = box.getBoundingClientRect(); const f = t.dir === 'v' ? (ev.clientX - r.left) / r.width : (ev.clientY - r.top) / r.height; t.ratio = Math.min(0.9, Math.max(0.1, f)); A.style.flex = t.ratio; B.style.flex = 1 - t.ratio; measure(); };
      const up = () => { div.removeEventListener('pointermove', move); div.removeEventListener('pointerup', up); document.body.classList.remove('resizing'); document.body.style.cursor = ''; render(); };
      div.addEventListener('pointermove', move); div.addEventListener('pointerup', up);
    } });
    box.append(A, div, B);
    return box;
  }
  function render() {
    main.innerHTML = '';
    stageTree = stageOf(); stageKeys = stageTree ? C.leaves(stageTree).map(s => s.key) : [];
    if (focused && !stageKeys.includes(focused) && !phantoms.some(p => p.key === focused)) focused = stageKeys[0] || null;
    const birth = editName; editName = null;
    const sb = C.sidebar(model, { openSpaces: [...open], openTabs: model.spaces.flatMap(s => s.tabs.map(t => s.key + '/' + t.name)), focusedKey: focused, currentSpace: space().key,
      onDismissPending: p => { for (const sp of model.spaces) sp.pending = (sp.pending || []).filter(x => x !== p); render(); },
      onToggleSpace: toggleSpace, onNewSpace: newSpace, onNewSeat: sp => openSpotlight(sp), onUndo: undoClose, onAccept: acceptTask, onRename: () => render(), editName: birth,
      onColor: sp => { const hues = [0, 30, 50, 90, 150, 190, 220, 260, 300, 330]; sp.hueI = ((sp.hueI == null ? -1 : sp.hueI) + 1) % hues.length; sp.color = 'hsl(' + hues[sp.hueI] + ' 70% ' + (theme() === 'light' ? 42 : 58) + '%)'; render(); } });
    const stage = el('div', { class: 'stage' }, (() => { if (!stageTree) return el('div'); const root = splitDom(stageTree); root.style.height = '100%'; return root; })());
    main.append(sb, stage);
    main.querySelectorAll('[data-key]').forEach(n => {
      const ph = space().participants.find(x => x.key === n.dataset.key);
      if (ph) { n.addEventListener('click', () => { streamFilter = streamFilter === ph.key ? null : ph.key; drawStream(); }); return; }  // no focus, no ripple
      n.addEventListener('click', () => { const s = seats.find(x => x.key === n.dataset.key); s.unread = 0; setFocus(n.dataset.key, { locate: n.classList.contains('pane') ? 'row' : 'pane' }); });
      if (n.classList.contains('pane')) { n.addEventListener('pointerenter', () => armHover(n.dataset.key)); n.addEventListener('pointerleave', disarmHover); }   // panes: dwell focus, brightness only; rows: never
    });
    sb.append(el('div', { class: 'stream-dock', id: 'stream' }));
    // drag a seat row or a pane strip onto a pane: the hovered half tints, dropping splits there (left/right/top/bottom)
    const sources = [...main.querySelectorAll('.seat-row[data-key]:not(.phantom)'), ...main.querySelectorAll('.pane[data-key] .strip')];
    for (const src of sources) src.addEventListener('pointerdown', e => {
      if (e.button !== 0 || e.target.closest('.close, .undo, button, input')) return;
      const key = src.dataset.key || src.closest('[data-key]').dataset.key, seat = seats.find(x => x.key === key); if (!seat) return;
      const x0 = e.clientX, y0 = e.clientY; let dragging = false, ghost = null, zone = null, overPane = null;
      const zoneOf = (pane, ev) => { const r = pane.getBoundingClientRect(); const fx = (ev.clientX - r.left) / r.width, fy = (ev.clientY - r.top) / r.height; const dl = fx, dr = 1 - fx, dt = fy, db = 1 - fy; const m = Math.min(dl, dr, dt, db); return m === dl ? 'left' : m === dr ? 'right' : m === dt ? 'top' : 'bottom'; };
      const clearZone = () => { if (overPane) { overPane.classList.remove('drop-left', 'drop-right', 'drop-top', 'drop-bottom'); overPane = null; zone = null; } };
      const move = ev => {
        if (!dragging) { if (Math.hypot(ev.clientX - x0, ev.clientY - y0) < 6) return; dragging = true; document.body.classList.add('dragging'); ghost = el('div', { class: 'drag-ghost' }, seat.name); document.body.append(ghost); }
        ghost.style.transform = 'translate(' + (ev.clientX + 12) + 'px,' + (ev.clientY + 12) + 'px)';
        ghost.style.setProperty('--seat', seat.color);
        const under = document.elementFromPoint(ev.clientX, ev.clientY), pane = under && under.closest('.pane[data-key]');
        if (pane !== overPane) clearZone();
        if (pane && pane.dataset.key !== key) { overPane = pane; const z = zoneOf(pane, ev); if (z !== zone) { pane.classList.remove('drop-left', 'drop-right', 'drop-top', 'drop-bottom'); pane.classList.add('drop-' + z); zone = z; } }
      };
      const up = () => {
        removeEventListener('pointermove', move); removeEventListener('pointerup', up);
        if (!dragging) return;
        document.body.classList.remove('dragging'); if (ghost) ghost.remove();
        const target = overPane && overPane.dataset.key, z = zone; clearZone();
        if (target && z) placeSeat(seat, z, target);
      };
      addEventListener('pointermove', move); addEventListener('pointerup', up);
    });
    // sidebar width: drag the right edge; persisted
    sb.style.width = (Lab.store.get('showcase.sbw') || T.sidebar.width_compact) + 'px';
    sb.append(el('div', { class: 'sb-resize', onpointerdown: e => {
      e.preventDefault(); const hnd = e.currentTarget; hnd.setPointerCapture(e.pointerId); document.body.classList.add('resizing'); document.body.style.cursor = 'col-resize';
      const move = ev => { const w = Math.min(520, Math.max(160, ev.clientX - sb.getBoundingClientRect().left)); sb.style.width = w + 'px'; Lab.store.set('showcase.sbw', w); measure(); };
      const up = () => { hnd.removeEventListener('pointermove', move); hnd.removeEventListener('pointerup', up); document.body.classList.remove('resizing'); document.body.style.cursor = ''; measure(); };
      hnd.addEventListener('pointermove', move); hnd.addEventListener('pointerup', up);
    } }));
    drawStream();
    setFocus(focused, { force: true });
    for (const seat of seats) paintTerm(seat);
    measure();
  }
  // Focus change touches classes only: no re-render, no re-measure (rows never move).
  // Locate ripple: row at its left edge, pane at its centre, in the seat colour.
  // Directional (operator): a row click rings the pane; a pane click bounces the row's border; keyboard does both.
  function locate(key, where) {
    const f = seats.find(x => x.key === key); if (!f) return;
    if (where === 'pane' || where === 'both') { const col = C.hslToRgb(f.color); for (const R of rects) if (R.key === key && R.pane) ripple(R.x + R.w / 2, R.y + R.h / 2, col); }
    if (where === 'row' || where === 'both') { const row = main.querySelector('.seat-row[data-key="' + key + '"]'); if (row) { row.classList.remove('bounce'); void row.offsetWidth; row.classList.add('bounce'); row.addEventListener('animationend', () => row.classList.remove('bounce'), { once: true }); } }
  }
  // Signal semantics (.bboxy/ux-pick.md): focus = brightness only. The locate ripple fires only with opts.locate,
  // which click and keyboard pass and dwell does not. Timers and messages never ripple.
  function setFocus(key, opts) {
    opts = opts || {};
    if (key === focused && !opts.force && !opts.locate) return;
    const changed = key !== focused;
    focused = key;
    if (!opts.force && !stageKeys.includes(key)) { render(); if (opts.locate) locate(key, opts.locate); return; }   // another tab: the stage switches, then the ripple says where you landed
    main.querySelectorAll('.seat-row[data-key]').forEach(n => n.classList.toggle('focused', n.dataset.key === key));
    main.querySelectorAll('.pane[data-key]').forEach(n => { const f = n.dataset.key === key; n.classList.toggle('focused', f); n.classList.toggle('unfocused', !f); });
    const f = seats.find(s => s.key === key);
    document.getElementById('crumb').textContent = '· ' + space().name + ' · ' + f.name;
    // locate (DECIDED): every focus change ripples the row and the pane corner in the seat colour, both directions;
    // an off-screen row scrolls into view inside the sidebar (never the window), then rects are re-measured once.
    const row = main.querySelector('.seat-row[data-key="' + key + '"]'), sc = main.querySelector('.sb-scroll');
    if (row && sc) {
      const top = row.offsetTop - sc.offsetTop, bottom = top + row.offsetHeight;
      if (top < sc.scrollTop || bottom > sc.scrollTop + sc.clientHeight) { sc.scrollTop = top - sc.clientHeight / 2; measure(); }
    }
    if (opts.locate) locate(key, opts.locate);
    if (changed) { main.querySelectorAll('.seat-row[data-key="' + key + '"]').forEach(n => { if (f.unread) { f.unread = 0; render(); } }); }
  }
  // Arrow keys move focus to the nearest pane in that direction (iTerm: ⌘⌥ arrows; plain arrows too while nothing types).
  function neighbour(dir) {
    const me = rects.find(R => R.pane && R.key === focused); if (!me) return null;
    const cx = me.x + me.w / 2, cy = me.y + me.h / 2; let best = null, bd = Infinity;
    for (const R of rects) {
      if (!R.pane || R.key === focused) continue;
      const x = R.x + R.w / 2, y = R.y + R.h / 2, dx = x - cx, dy = y - cy;
      const ahead = dir === 'ArrowRight' ? dx > 0 && R.x >= me.x + me.w - 2 : dir === 'ArrowLeft' ? dx < 0 && R.x + R.w <= me.x + 2 : dir === 'ArrowDown' ? dy > 0 && R.y >= me.y + me.h - 2 : dy < 0 && R.y + R.h <= me.y + 2;
      if (!ahead) continue;
      const along = Math.abs(dir === 'ArrowRight' || dir === 'ArrowLeft' ? dx : dy), across = Math.abs(dir === 'ArrowRight' || dir === 'ArrowLeft' ? dy : dx);
      const d = along + across * 2;                       // prefer the pane most in line
      if (d < bd) { bd = d; best = R.key; }
    }
    return best;
  }
  addEventListener('keydown', e => {
    if (!/^Arrow(Left|Right|Up|Down)$/.test(e.key) || spot) return;
    const typing = document.activeElement && /^(INPUT|TEXTAREA|SELECT)$/.test(document.activeElement.tagName);
    if (typing && !(e.metaKey && e.altKey)) return;
    const next = neighbour(e.key); if (!next) return;
    e.preventDefault(); setFocus(next, { locate: 'both' });
  });
  // Hover focus on panes only (operator): a 150 ms dwell moves focus brightness, no ring; typing blocks it for a second.
  // Sidebar rows never focus on hover: click changes what you see.
  let hoverT = null, lastKey = 0, lastMove = 0;
  addEventListener('keydown', () => { lastKey = performance.now(); });
  addEventListener('pointermove', () => { lastMove = performance.now(); }, { passive: true });
  function armHover(key) { disarmHover(); hoverT = setTimeout(() => { if (performance.now() - lastKey > 1000) setFocus(key); }, 150); }
  function disarmHover() { if (hoverT) { clearTimeout(hoverT); hoverT = null; } }
  // Rect measurement happens once per render and on resize, never per frame: no layout thrash.
  function measure() {
    rects = [];
    main.querySelectorAll('[data-key]').forEach(n => {
      const s = seats.find(x => x.key === n.dataset.key) || space().participants.find(x => x.key === n.dataset.key); if (!s) return;
      const b = n.getBoundingClientRect();
      rects.push({ key: s.key, seat: s, pane: n.classList.contains('pane'), x: b.left, y: b.top, w: b.width, h: b.height });
    });
  }

  // ---------- WebGL2 ----------
  const cv = document.getElementById('glow');
  const gl = cv.getContext('webgl2', { antialias: false, alpha: true, premultipliedAlpha: true });
  if (!gl) { render(); return; }
  const MAXR = 40, MAXRIP = 6;
  const vs = `#version 300 es
    const vec2 q[3] = vec2[3](vec2(-1.,-1.), vec2(3.,-1.), vec2(-1.,3.));
    void main(){ gl_Position = vec4(q[gl_VertexID], 0., 1.); }`;
  const fs = `#version 300 es
    precision highp float;
    uniform vec2 uRes; uniform float uTime; uniform vec3 uBg; uniform int uN; uniform int uNR; uniform float uLight;
    uniform vec4 uRect[${MAXR}];   // x y w h (device px, y down)
    uniform vec4 uCol[${MAXR}];    // rgb, focus(0/1)
    uniform vec4 uPar[${MAXR}];    // state(0 idle,1 active,2 waiting,3 errored), radius, seed, particles
    uniform vec4 uRip[${MAXRIP}];  // x y t0 unused
    uniform vec4 uRipCol[${MAXRIP}];
    uniform float uMul[${MAXR}];   // per-rect alpha multiplier: panes 1, sidebar rows glow.row_alpha
    uniform float uPulseMs, uActiveMs, uIdleA, uActMin, uActMax, uWaitMin, uWaitMax, uErrA, uBoost, uSpeed, uPSize;
    uniform vec3 uAmber, uRed;
    out vec4 o;
    float hash(float n){ return fract(sin(n)*43758.5453); }
    float breathe(float ms){ return 0.5 - 0.5*cos(uTime/ms*6.28318); }
    void main(){
      vec2 p = vec2(gl_FragCoord.x, uRes.y - gl_FragCoord.y);
      vec3 acc = vec3(0.); float w = 0.0;
      for (int i = 0; i < ${MAXR}; i++) {
        if (i >= uN) break;
        vec4 R = uRect[i]; vec2 c = R.xy + R.zw*0.5; vec2 hs = R.zw*0.5;
        vec2 q = p - c; vec2 d2 = abs(q) - hs;
        float dOut = length(max(d2, 0.)); float dIn = min(max(d2.x, d2.y), 0.);
        float d = abs(dOut + dIn);                    // distance to the border line
        float rad = uPar[i].y;
        if (d > rad * 4.0) continue;                  // budget: only rects whose halo reaches this pixel
        float st = uPar[i].x; vec3 col = uCol[i].rgb; float focus = uCol[i].w;
        float a; float speed = 0.5; float count = uPar[i].w;
        if (st < 0.5)      { a = uIdleA; speed = 0.35; }
        else if (st < 1.5) { a = mix(uActMin, uActMax, breathe(uActiveMs)); speed = 1.0; }
        else if (st < 2.5) { a = mix(uWaitMin, uWaitMax, breathe(uPulseMs)); col = uAmber; speed = 0.6; }
        else               { a = uErrA; col = uRed; speed = 0.15; count *= 0.5; }
        a *= mix(1.0, uBoost, focus) * uMul[i];
        float inside = step(dOut, 0.0);
        float g = exp(-d / (rad * mix(1.0, 0.45, inside))) * a * 0.38;
        acc += col * g; w += g;
        // particles: perimeter coordinate t in [0,1), advect with time, gaussian in t and in d
        vec2 e = hs - abs(q); float P = 4.0*(hs.x + hs.y);
        float t;
        if (e.x < e.y) { t = q.x > 0. ? (hs.x + hs.y + (q.y + hs.y)) : (3.0*hs.x + 2.0*hs.y + (hs.y - q.y)); }
        else           { t = q.y < 0. ? (q.x + hs.x) : (2.0*hs.x + hs.y + (hs.x - q.x)); }
        t /= P;
        float sig = uPSize * mix(1.0, 1.6, focus);
        float pk = 0.0;
        int n = int(count);
        for (int k = 0; k < 48; k++) {
          if (k >= n) break;
          float s0 = hash(uPar[i].z + float(k)*7.31);
          float sp = (0.6 + 0.8*hash(uPar[i].z + float(k)*3.7)) * speed * uSpeed / P;
          float tk = fract(s0 + uTime*0.001*sp);
          float dt = abs(fract(t - tk + 0.5) - 0.5) * P;
          float tw = 0.65 + 0.35*sin(uTime*0.004 + float(k)*1.7);
          pk += exp(-(dt*dt)/(2.0*sig*sig*3.0)) * exp(-(d*d)/(2.0*sig*sig*0.6)) * tw;
        }
        float pg = min(pk, 1.5) * a * mix(0.9, 0.45, uLight);
        acc += col * pg; w += pg;
      }
      for (int j = 0; j < ${MAXRIP}; j++) {
        if (j >= uNR) break;
        float age = uTime - uRip[j].z; if (age < 0.0 || age > 900.0) continue;
        float rr = age * 0.22; float dist = distance(p, uRip[j].xy);
        float ring = exp(-pow(dist - rr, 2.0) / (2.0*9.0)) * (1.0 - age/900.0) * 0.9;
        acc += uRipCol[j].rgb * ring; w += ring;
      }
      // The canvas sits above the DOM (zero layout space): output is premultiplied alpha over whatever is beneath.
      if (uLight > 0.5) {
        // light ground: additive washes out, so the halo darkens toward the seat colour
        vec3 tint = w > 0.001 ? acc / w : vec3(1.0);
        float a = clamp(w * 1.1, 0.0, 0.8);
        o = vec4(uBg * tint * a, a);
      } else {
        o = vec4(acc, clamp(w, 0.0, 1.0));
      }
    }`;
  function sh(type, src) { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; }
  const prog = gl.createProgram(); gl.attachShader(prog, sh(gl.VERTEX_SHADER, vs)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, fs)); gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
  gl.useProgram(prog);
  const U = {}; for (const n of ['uRes', 'uTime', 'uBg', 'uN', 'uNR', 'uLight', 'uRect', 'uCol', 'uPar', 'uMul', 'uRip', 'uRipCol', 'uPulseMs', 'uActiveMs', 'uIdleA', 'uActMin', 'uActMax', 'uWaitMin', 'uWaitMax', 'uErrA', 'uBoost', 'uSpeed', 'uPSize', 'uAmber', 'uRed']) U[n] = gl.getUniformLocation(prog, n);
  gl.uniform3fv(U.uAmber, C.hslToRgb(T.color.state.waiting)); gl.uniform3fv(U.uRed, C.hslToRgb(T.color.state.errored));
  gl.uniform1f(U.uPulseMs, T.motion.pulse_ms); gl.uniform1f(U.uActiveMs, G.active_pulse_ms); gl.uniform1f(U.uIdleA, G.idle_alpha);
  gl.uniform1f(U.uActMin, G.active_alpha_min); gl.uniform1f(U.uActMax, G.active_alpha_max); gl.uniform1f(U.uWaitMin, G.waiting_alpha_min); gl.uniform1f(U.uWaitMax, G.waiting_alpha_max);
  gl.uniform1f(U.uErrA, G.errored_alpha); gl.uniform1f(U.uBoost, G.focused_boost); gl.uniform1f(U.uSpeed, G.particle_speed); gl.uniform1f(U.uPSize, G.particle_size * Math.min(devicePixelRatio, 2));

  const STATE_IDX = { idle: 0, active: 1, waiting: 2, errored: 3 };
  const rectBuf = new Float32Array(MAXR * 4), colBuf = new Float32Array(MAXR * 4), parBuf = new Float32Array(MAXR * 4), mulBuf = new Float32Array(MAXR);
  const ripples = []; const ripBuf = new Float32Array(MAXRIP * 4), ripCol = new Float32Array(MAXRIP * 4);
  function ripple(x, y, color) { ripples.push({ x, y, t0: performance.now(), c: color }); if (ripples.length > MAXRIP) ripples.shift(); }
  function resize() { const dpr = Math.min(devicePixelRatio, 2); cv.width = innerWidth * dpr; cv.height = innerHeight * dpr; gl.viewport(0, 0, cv.width, cv.height); measure(); }
  addEventListener('resize', resize);

  let frames = 0, fpsT = performance.now(), fps = 0, ftMax = 0, ftAcc = 0, lastNow = 0;
  function frame(now) {
    const dpr = cv.width / innerWidth, th = theme();
    let n = 0;
    for (const R of rects) {
      if (n >= MAXR) break;
      const s = R.seat, rgb = C.hslToRgb(s.color);
      rectBuf.set([R.x * dpr, R.y * dpr, R.w * dpr, R.h * dpr], n * 4);
      const pulse = s.pulseT ? Math.max(0, 1 - (now - s.pulseT) / 700) : 0;  // phantom message: one pulse, 700 ms, through the brightness channel
      colBuf.set([rgb[0], rgb[1], rgb[2], s.key === focused ? 1 : pulse], n * 4);
      const rad = (R.pane ? G.pane_radius : G.row_radius) * dpr;
      const parts = R.pane ? (s.state === 'active' ? G.particles_active : G.particles_idle * 2) : (s.state === 'active' ? G.row_particles_active : 0);
      parBuf.set([STATE_IDX[s.state], rad, C.hash32(s.key) % 1000, parts], n * 4);
      mulBuf[n] = (R.pane ? 1 : (s.state === 'idle' && !G.row_idle ? 0 : G.row_alpha)) * (s.grade === 'inferred' ? 0.5 : 1);
      n++;
    }
    gl.uniform3fv(U.uBg, C.hslToRgb(T.color[th].bg)); gl.uniform1f(U.uLight, th === 'light' ? 1 : 0);
    gl.uniform2f(U.uRes, cv.width, cv.height); gl.uniform1f(U.uTime, now);
    gl.uniform1i(U.uN, n); gl.uniform4fv(U.uRect, rectBuf); gl.uniform4fv(U.uCol, colBuf); gl.uniform4fv(U.uPar, parBuf); gl.uniform1fv(U.uMul, mulBuf);
    ripples.forEach((rp, j) => { ripBuf.set([rp.x * dpr, rp.y * dpr, rp.t0, 0], j * 4); ripCol.set([rp.c[0], rp.c[1], rp.c[2], 1], j * 4); });
    gl.uniform1i(U.uNR, ripples.length); gl.uniform4fv(U.uRip, ripBuf); gl.uniform4fv(U.uRipCol, ripCol);
    gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    if (lastNow) { const ft = now - lastNow; ftAcc += ft; ftMax = Math.max(ftMax, ft); } lastNow = now;
    frames++; if (now - fpsT > 1000) { fps = frames; window.__fps = { fps, avgMs: +(ftAcc / frames).toFixed(2), maxMs: +ftMax.toFixed(1), rects: rects.length, scene: SCENE }; frames = 0; ftAcc = 0; ftMax = 0; fpsT = now; }
    requestAnimationFrame(frame);
  }

  // ---------- cause and effect: no timers. Keys change one thing at a time. ----------
  const ROLE_RANK = { oracle: 0, executor: 1, creativity: 2, unsigned: 3 };
  let msgN = 0;
  addEventListener('keydown', e => {
    if (e.metaKey || e.ctrlKey || e.altKey || spot) return;
    const f = seats.find(s => s.key === focused); if (!f || f.settings || f.graph) return;
    const st = { '1': 'idle', '2': 'active', '3': 'waiting', '4': 'errored' }[e.key];
    if (st) { if (st === 'waiting' && f.state === 'waiting') { const ks = ['permission', 'question', 'auth']; f.waitKind = ks[(ks.indexOf(f.waitKind || 'question') + 1) % 3]; render(); return; } setState(f, st); if (st === 'active' && f.replay && f.replay.waiting) { f.replay.waiting = false; schedule(f, 200); } }
    else if (e.key === 'm' || e.key === 'M') {
      // a message arrives for the next seat in order, from a sender role that cycles; unread dot only, no ripple
      const s = seats[msgN % seats.length], role = ['executor', 'oracle', 'creativity'][msgN % 3]; msgN++;
      // a muted seat holds swarm traffic instead (demo: every fourth held item is outgoing work its grant no longer permits)
      if (s.muted) { s.held = s.held || []; const k = s.held.length; s.held.push({ id: 'msg-' + C.hash32(s.key + k).toString(16).slice(0, 6), dir: k % 4 === 3 ? 'out' : 'in', blocked: k % 4 === 3 ? 'sent under an old grant epoch · the current grant no longer permits the swarm' : null }); render(); return; }
      s.unread = (s.unread || 0) + 1;
      if (!s.unreadRole || ROLE_RANK[role] < ROLE_RANK[s.unreadRole]) s.unreadRole = role;
      render();
    }
    else if (e.key === 'Tab') { e.preventDefault(); const i = stageKeys.indexOf(focused); setFocus(stageKeys[(i + (e.shiftKey ? -1 : 1) + stageKeys.length) % stageKeys.length], { locate: 'both' }); }
  });

  // ---------- stream dock: messages land here; clicking a phantom filters to that participant ----------
  function drawStream() {
    const dock = document.getElementById('stream'); if (!dock) return;
    dock.innerHTML = '';
    const stream = space().stream;
    for (const ep of (space().epics || [])) if (ep.items.length) dock.append(C.epicGroup(ep, { onToggle: drawStream }));
    if ((space().external || []).length) dock.append(C.externalLane(space().external, { open: extOpen, onToggle: () => { extOpen = !extOpen; drawStream(); } }));
    const items = stream.filter(m => !streamFilter || m.fromKey === streamFilter).slice(-4);
    if (streamFilter) { const ph = space().participants.find(p => p.key === streamFilter); if (!ph) { streamFilter = null; return drawStream(); } dock.append(el('div', { class: 'filter' }, 'from ', el('b', {}, ph.name), el('button', { onclick: () => { streamFilter = null; drawStream(); } }, '× all'))); }
    for (const m of items) dock.append(C.messageRow(m));
    dock.style.display = (stream.length || (space().epics || []).some(e => e.items.length) || (space().external || []).length) ? '' : 'none';
  }
  const PHRASES = ['relay bench green on eu-2, 41 ms p50', 'claiming the glyphon atlas question', 'B spec amended, envelope has presence now', 'who owns the ssh bootstrap doc?', 'pushed fixtures, four files'];
  let phN = 0;
  addEventListener('keydown', e => {
    if ((e.key !== 'p' && e.key !== 'P') || spot) return;
    const phs = space().participants; if (!phs.length) return;
    const ph = phs[phN % phs.length], text = PHRASES[phN % PHRASES.length]; phN++;
    ph.pulseT = performance.now();                                        // row pulses once
    space().stream.push({ from: ph.name, fromKey: ph.key, role: ph.role, foreign: true, text, time: new Date().toTimeString().slice(0, 5), unread: true });
    drawStream();
  });

  // ---------- live log replay ----------
  // Each seat replays one fixture line by line: bursts of 1-6 lines at 30-120 ms, pauses of 0.4-2 s between bursts,
  // a 3-6 s pause at the end of the fixture, then it loops. State follows the replay: active while lines flow,
  // idle at the loop pause, waiting when a burst ends on a line containing 'Allow' or ending in '?', until key 2.
  const L = window.LOGS || {};
  const SHELL = [['cargo build --release', 'cargo-build'], ['git log -3', 'git-log'], ['icq --human status', 'icq-status'], ['ls -R .kb', 'ls-kb']];
  function fixtureFor(seat, i) {
    if (seat.harness === 'claude') return (L['claude-session'] || '').split('\n');
    if (seat.harness === 'codex') return (L['codex-session'] || '').split('\n');
    const out = []; // shell seats: prompt + each shell fixture, rotated by seat index so panes differ
    for (let k = 0; k < SHELL.length; k++) { const [cmd, name] = SHELL[(k + i) % SHELL.length]; out.push('$ ' + cmd, ...(L[name] || '').split('\n'), ''); }
    return out;
  }
  const MAXLINES = 400;
  function paintTerm(seat) {
    const term = main.querySelector('.pane[data-key="' + seat.key + '"] .term'); if (!term || !seat.replay) return;
    const rp = seat.replay;
    term.innerHTML = '';
    const lines = el('div', { class: 'lines' }, rp.buf.join('\n'));
    lines.append(el('span', { class: 'cur', style: 'margin-left:2px' }));
    term.append(lines);
  }
  function emit(seat, line) {
    const rp = seat.replay; rp.buf.push(line); if (rp.buf.length > MAXLINES) rp.buf.shift();
    paintTerm(seat);
  }
  function schedule(seat, ms) { clearTimeout(seat.replay.timer); seat.replay.timer = setTimeout(() => step(seat), ms); }
  function step(seat) {
    const rp = seat.replay, rnd = rp.rnd;
    if (rp.waiting) return;
    if (rp.idx >= rp.lines.length) {                     // end of fixture: idle at the loop pause, then restart
      setState(seat, 'idle');
      rp.idx = 0; rp.burst = 0; rp.buf.push(''); 
      return schedule(seat, 3000 + rnd() * 3000);
    }
    if (rp.burst === 0) rp.burst = 1 + Math.floor(rnd() * 6);
    setState(seat, 'active');
    const line = rp.lines[rp.idx++]; emit(seat, line); rp.burst--;
    if (rp.burst === 0) {                                 // burst ended
      if (/Allow|\?\s*$/.test(line)) { seat.waitKind = /Allow/.test(line) ? 'permission' : /auth|login|token|password/i.test(line) ? 'auth' : 'question'; rp.waiting = true; setState(seat, 'waiting'); return; }
      return schedule(seat, 400 + rnd() * 1600);
    }
    schedule(seat, 30 + rnd() * 90);
  }
  function startReplay(seat, i) {
    if (seat.settings || seat.graph) return;
    const lines = fixtureFor(seat, i);
    const start = Math.floor(C.rng('start' + seat.key)() * lines.length);
    seat.replay = { lines, idx: start, buf: lines.slice(Math.max(0, start - 12), start), burst: 0, waiting: false, rnd: C.rng('replay' + seat.key), timer: null };
    if (seat.state === 'waiting' || seat.state === 'errored') seat.replay.waiting = true;   // held until the operator presses 2
    else schedule(seat, 300 + i * 170);
  }
  render(); resize(); requestAnimationFrame(frame);
  seats.slice().forEach((seat, i) => startReplay(seat, i));
})();
