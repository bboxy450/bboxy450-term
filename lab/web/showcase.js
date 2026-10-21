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
  const mq = matchMedia('(prefers-color-scheme: dark)');
  let themeMode = localStorage.getItem('showcase.theme') || 'auto';
  const theme = () => themeMode === 'auto' ? (mq.matches ? 'dark' : 'light') : themeMode;
  function applyTheme() { Lab.prefs.theme = theme(); Lab.prefs.density = 'compact'; Lab.applyTokens(); document.documentElement.style.setProperty('--gutter', GUTTER + 'px'); }
  mq.addEventListener('change', () => { if (themeMode === 'auto') { applyTheme(); render(); } });
  applyTheme();

  // ---------- model ----------
  const r = C.rng('showcase' + SCENE);
  const PAL = ['#FFC53D', '#FF4FA3', '#B07BFF', '#2EE6A8', '#2E6BFF', '#FF7A45', '#58D7FF', '#9AA3C7', '#F2D16B', '#5BE08C', '#E96BFF', '#FF6B6B', '#4FD1FF', '#C9A96B', '#8AF0D0'];
  const PAL_LIGHT = ['#B8860B', '#C2185B', '#6A3FD1', '#0E8F6A', '#1E4FD1', '#D1541C', '#0E7FA8', '#5B6480', '#9A7B10', '#1F8F4A', '#9C1FBF', '#C63B3B', '#0E86B8', '#8A6B2E', '#1E9C7E'];
  const mk = (i, name, harness, role, title, state) => ({ key: 'sc' + i, name, harness, role, title, state, pal: i });
  const BASE = [
    mk(0, 'Vesper Greywind', 'claude', 'oracle', 'cohort control', 'active'),
    mk(1, 'Nyx Copperwing', 'codex', 'executor', 'messaging spec', 'active'),
    mk(2, 'Lumen Foxglove', 'claude', 'creativity', 'ux lab', 'idle'),
    mk(3, 'Rook Ashwhisker', 'codex', 'executor', 'gpu research', 'waiting'),
    mk(4, 'Sable Quill', 'codex', 'executor', 'harness hooks', 'idle'),
    mk(5, 'Pip Hollow', 'pi', 'executor', 'relay bench', 'errored'),
    mk(6, 'Marrow Finch', 'omp', 'executor', 'parse logs', 'idle'),
    mk(7, 'zsh', 'zsh', null, '~/mydev', 'idle'),
    mk(8, 'Tamsin Voss', 'codex', 'executor', 'cargo test', 'active'),
    mk(9, 'Corvin Ember', 'codex', 'executor', 'review PR 14', 'idle'),
    mk(10, 'Ilka Thistle', 'pi', 'executor', 'relay bench', 'active'),
    mk(11, 'Dax Riven', 'codex', 'executor', 'mux daemon', 'idle'),
    mk(12, 'Wren Starling', 'omp', 'executor', 'tokens', 'waiting'),
    mk(13, 'Halcyon Moss', 'codex', 'executor', 'ssh rig', 'idle'),
    mk(14, 'Juniper Vale', 'claude', 'executor', 'parse logs', 'active'),
  ];
  const seats = BASE.slice(0, SCENE === 15 ? 15 : 8);
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
        { name: 'cohort', shortcut: '⌘1', tree: stageTree, seats: [S(0), S(1), S(3)] },
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
  const stream = []; let streamFilter = null;
  let stageKeys = C.leaves(stageTree).map(s => s.key);
  let focused = stageKeys[0];
  // Close kills the seat: its leaf collapses into the sibling, the row leaves the sidebar, focus moves to the first pane.
  function prune(t, key) { if (t.key) return t.key === key ? null : t; const a = prune(t.a, key), b = prune(t.b, key); if (!a) return b; if (!b) return a; return { ...t, a, b }; }
  function closeSeat(seat) {
    if (stageKeys.length <= 1) return;
    stageTree = prune(stageTree, seat.key); stageKeys = C.leaves(stageTree).map(s => s.key);
    for (const sp of model.spaces) for (const tab of sp.tabs) { if (tab.seats.includes(seat)) { tab.seats = tab.seats.filter(x => x !== seat); tab.tree = prune(tab.tree, seat.key) || tab.seats[0]; } }
    for (const sp of model.spaces) sp.tabs = sp.tabs.filter(t => t.seats.length);
    seats.splice(seats.indexOf(seat), 1); clearTimeout(seat.replay && seat.replay.timer);
    if (focused === seat.key) focused = stageKeys[0];
    render();
  }

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
  const themeBtn = el('span', { class: 'tbtn theme', title: 'theme: auto → dark → light', onclick: () => { themeMode = { auto: 'dark', dark: 'light', light: 'auto' }[themeMode]; localStorage.setItem('showcase.theme', themeMode); applyTheme(); drawThemeBtn(); render(); } });
  function drawThemeBtn() { themeBtn.innerHTML = ''; themeBtn.append(el('span', {}, theme() === 'dark' ? '◑' : '◐'), el('span', {}, themeMode)); }
  drawThemeBtn();
  const titlebar = el('div', { class: 'titlebar' }, el('span', { class: 'win' }, el('i'), el('i'), el('i')), el('span', { class: 'tbtn on' }, C.harnessIcon('sidebarfoxopen')), C.logo(18, { ears: false }), el('span', { class: 'ttl' }, 'bboxy450-term'), el('span', { id: 'crumb' }), el('span', { class: 'spacer' }), themeBtn, el('span', { class: 'tbtn' }, C.harnessIcon('gear')));
  const main = el('div', { id: 'main' });
  app.append(titlebar, main);
  let rects = [];
  function splitDom(t) {
    if (t.key) return C.pane(t, { focused: t.key === focused, unfocused: t.key !== focused, shortcut: '⌘' + (stageKeys.indexOf(t.key) + 1), onClose: closeSeat });
    const A = splitDom(t.a), B = splitDom(t.b);
    A.style.flex = t.ratio || 0.5; B.style.flex = 1 - (t.ratio || 0.5);
    return el('div', { class: 'split ' + t.dir }, A, B);
  }
  function render() {
    main.innerHTML = '';
    const sb = C.sidebar(model, { openSpaces: model.spaces.map(s => s.key), openTabs: model.spaces.flatMap(s => s.tabs.map(t => s.key + '/' + t.name)), focusedKey: focused });
    const stage = el('div', { class: 'stage' }, (() => { const root = splitDom(stageTree); root.style.height = '100%'; return root; })());
    main.append(sb, stage);
    main.querySelectorAll('[data-key]').forEach(n => {
      const ph = phantoms.find(x => x.key === n.dataset.key);
      if (ph) { n.addEventListener('click', () => { streamFilter = streamFilter === ph.key ? null : ph.key; drawStream(); }); return; }  // no focus, no ripple
      n.addEventListener('click', () => { const s = seats.find(x => x.key === n.dataset.key); s.unread = 0; setFocus(n.dataset.key, { locate: true }); });
      n.addEventListener('pointerenter', () => armHover(n.dataset.key));
      n.addEventListener('pointerleave', disarmHover);
    });
    sb.insertBefore(el('div', { class: 'stream-dock', id: 'stream' }), sb.querySelector('.sb-foot'));
    drawStream();
    setFocus(focused, { force: true });
    for (const seat of seats) paintTerm(seat);
    measure();
  }
  // Focus change touches classes only: no re-render, no re-measure (rows never move).
  // Signal semantics (.kb/ux-pick.md): focus = brightness only. The locate ripple fires only with opts.locate,
  // which click and keyboard pass and dwell does not. Timers and messages never ripple.
  function setFocus(key, opts) {
    opts = opts || {};
    if (key === focused && !opts.force && !opts.locate) return;
    const changed = key !== focused;
    focused = key;
    main.querySelectorAll('.seat-row[data-key]').forEach(n => n.classList.toggle('focused', n.dataset.key === key));
    main.querySelectorAll('.pane[data-key]').forEach(n => { const f = n.dataset.key === key; n.classList.toggle('focused', f); n.classList.toggle('unfocused', !f); });
    const f = seats.find(s => s.key === key);
    document.getElementById('crumb').textContent = '· ' + model.spaces[0].name + ' · ' + f.name;
    // locate (DECIDED): every focus change ripples the row and the pane corner in the seat colour, both directions;
    // an off-screen row scrolls into view inside the sidebar (never the window), then rects are re-measured once.
    const row = main.querySelector('.seat-row[data-key="' + key + '"]'), sc = main.querySelector('.sb-scroll');
    if (row && sc) {
      const top = row.offsetTop - sc.offsetTop, bottom = top + row.offsetHeight;
      if (top < sc.scrollTop || bottom > sc.scrollTop + sc.clientHeight) { sc.scrollTop = top - sc.clientHeight / 2; measure(); }
    }
    if (opts.locate) { const col = C.hslToRgb(f.color); for (const R of rects) if (R.key === key) ripple(R.pane ? R.x + R.w / 2 : R.x + 10, R.pane ? R.y + R.h / 2 : R.y + R.h / 2, col); }  // pane: centre; row: left edge
    if (changed) { main.querySelectorAll('.seat-row[data-key="' + key + '"]').forEach(n => { if (f.unread) { f.unread = 0; render(); } }); }
  }
  // Focus follows the mouse after a short dwell. Typing holds focus: a keydown within the last second blocks hover focus.
  let hoverT = null, lastKey = 0, lastMove = 0;
  addEventListener('keydown', () => { lastKey = performance.now(); });
  addEventListener('pointermove', () => { lastMove = performance.now(); }, { passive: true });
  // Dwell passes no locate: it only moves focus brightness.
  function armHover(key) { disarmHover(); hoverT = setTimeout(() => { if (performance.now() - lastKey > 1000 && performance.now() - lastMove < 2000) setFocus(key); }, 150); }
  function disarmHover() { if (hoverT) { clearTimeout(hoverT); hoverT = null; } }
  // Rect measurement happens once per render and on resize, never per frame: no layout thrash.
  function measure() {
    rects = [];
    main.querySelectorAll('[data-key]').forEach(n => {
      const s = everyone.find(x => x.key === n.dataset.key); if (!s) return;
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
      mulBuf[n] = R.pane ? 1 : (s.state === 'idle' && !G.row_idle ? 0 : G.row_alpha);
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
    if (e.metaKey || e.ctrlKey || e.altKey) return;
    const f = seats.find(s => s.key === focused);
    const st = { '1': 'idle', '2': 'active', '3': 'waiting', '4': 'errored' }[e.key];
    if (st) { setState(f, st); if (st === 'active' && f.replay && f.replay.waiting) { f.replay.waiting = false; schedule(f, 200); } }
    else if (e.key === 'm' || e.key === 'M') {
      // a message arrives for the next seat in order, from a sender role that cycles; unread dot only, no ripple
      const s = seats[msgN % seats.length], role = ['executor', 'oracle', 'creativity'][msgN % 3]; msgN++;
      s.unread = (s.unread || 0) + 1;
      if (!s.unreadRole || ROLE_RANK[role] < ROLE_RANK[s.unreadRole]) s.unreadRole = role;
      render();
    }
    else if (e.key === 'Tab') { e.preventDefault(); const i = stageKeys.indexOf(focused); setFocus(stageKeys[(i + (e.shiftKey ? -1 : 1) + stageKeys.length) % stageKeys.length], { locate: true }); }
  });

  // ---------- stream dock: messages land here; clicking a phantom filters to that participant ----------
  function drawStream() {
    const dock = document.getElementById('stream'); if (!dock) return;
    dock.innerHTML = '';
    const items = stream.filter(m => !streamFilter || m.fromKey === streamFilter).slice(-4);
    if (streamFilter) { const ph = phantoms.find(p => p.key === streamFilter); dock.append(el('div', { class: 'filter' }, 'from ', el('b', {}, ph.name), el('button', { onclick: () => { streamFilter = null; drawStream(); } }, '× all'))); }
    for (const m of items) dock.append(C.messageRow(m));
    dock.style.display = stream.length ? '' : 'none';
  }
  const PHRASES = ['relay bench green on eu-2, 41 ms p50', 'claiming the glyphon atlas question', 'B spec amended, envelope has presence now', 'who owns the ssh bootstrap doc?', 'pushed fixtures, four files'];
  let phN = 0;
  addEventListener('keydown', e => {
    if (e.key !== 'p' && e.key !== 'P') return;
    const ph = phantoms[phN % phantoms.length], text = PHRASES[phN % PHRASES.length]; phN++;
    ph.pulseT = performance.now();                                        // row pulses once
    stream.push({ from: ph.name, fromKey: ph.key, role: ph.role, foreign: true, text, time: new Date().toTimeString().slice(0, 5), unread: true });
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
      if (/Allow|\?\s*$/.test(line)) { rp.waiting = true; setState(seat, 'waiting'); return; }
      return schedule(seat, 400 + rnd() * 1600);
    }
    schedule(seat, 30 + rnd() * 90);
  }
  seats.forEach((seat, i) => {
    const lines = fixtureFor(seat, i);
    const start = Math.floor(C.rng('start' + seat.key)() * lines.length);
    seat.replay = { lines, idx: start, buf: lines.slice(Math.max(0, start - 12), start), burst: 0, waiting: false, rnd: C.rng('replay' + seat.key), timer: null };
    if (seat.state === 'waiting') seat.replay.waiting = true; else if (seat.state === 'errored') { /* errored seats hold until the operator presses 2 */ seat.replay.waiting = true; }
  });
  render(); resize(); requestAnimationFrame(frame);
  seats.forEach((seat, i) => { if (!seat.replay.waiting) schedule(seat, 300 + i * 170); });
})();
