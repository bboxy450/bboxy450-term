// Showcase: full-window mock. Sidebar + a split tree of panes in the DOM; the border glow is a WebGL2 fragment
// shader on a full-window canvas behind the DOM. Per seat: rect, colour, state, focus. Particles advect along the
// border; ripples expand from a sidebar row and the matching pane corner. A timer keeps the scene alive.
// ?scene=15 renders fifteen seats in one tab as a deep uneven split tree. Theme follows the OS unless toggled.
(function () {
  const T = window.TOKENS, G = T.glow, el = Lab.el;
  const q = new URLSearchParams(location.search);
  const SCENE = q.get('scene') === '15' ? 15 : 3;

  // ---------- theme: auto (prefers-color-scheme) | dark | light ----------
  const mq = matchMedia('(prefers-color-scheme: dark)');
  let themeMode = localStorage.getItem('showcase.theme') || 'auto';
  const theme = () => themeMode === 'auto' ? (mq.matches ? 'dark' : 'light') : themeMode;
  function applyTheme() { Lab.prefs.theme = theme(); Lab.prefs.density = 'comfortable'; Lab.applyTokens(); document.documentElement.style.setProperty('--stage-gap', (SCENE === 15 ? 12 : 28) + 'px'); }
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
  const stageKeys = C.leaves(stageTree).map(s => s.key);
  let focused = stageKeys[0];

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
    if (t.key) return C.pane(t, { focused: t.key === focused, unfocused: t.key !== focused, shortcut: '⌘' + (stageKeys.indexOf(t.key) + 1) });
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
      n.addEventListener('click', () => setFocus(n.dataset.key));
      n.addEventListener('pointerenter', () => armHover(n.dataset.key));
      n.addEventListener('pointerleave', disarmHover);
    });
    setFocus(focused, true);
    measure();
  }
  // Focus change touches classes only: no re-render, no re-measure (rows never move).
  function setFocus(key, force) {
    if (key === focused && !force) return;
    focused = key;
    main.querySelectorAll('.seat-row[data-key]').forEach(n => n.classList.toggle('focused', n.dataset.key === key));
    main.querySelectorAll('.pane[data-key]').forEach(n => { const f = n.dataset.key === key; n.classList.toggle('focused', f); n.classList.toggle('unfocused', !f); });
    const f = seats.find(s => s.key === key);
    document.getElementById('crumb').textContent = '· ' + model.spaces[0].name + ' · ' + f.name;
  }
  // Focus follows the mouse after a short dwell. Typing holds focus: a keydown within the last second blocks hover focus.
  let hoverT = null, lastKey = 0, lastMove = 0;
  addEventListener('keydown', () => { lastKey = performance.now(); });
  addEventListener('pointermove', () => { lastMove = performance.now(); }, { passive: true });
  function armHover(key) { disarmHover(); hoverT = setTimeout(() => { if (performance.now() - lastKey > 1000 && performance.now() - lastMove < 2000) setFocus(key); }, 150); }
  function disarmHover() { if (hoverT) { clearTimeout(hoverT); hoverT = null; } }
  // Rect measurement happens once per render and on resize, never per frame: no layout thrash.
  function measure() {
    rects = [];
    main.querySelectorAll('[data-key]').forEach(n => {
      const s = seats.find(x => x.key === n.dataset.key); if (!s) return;
      const b = n.getBoundingClientRect();
      rects.push({ key: s.key, seat: s, pane: n.classList.contains('pane'), x: b.left, y: b.top, w: b.width, h: b.height });
    });
  }

  // ---------- WebGL2 ----------
  const cv = document.getElementById('glow');
  const gl = cv.getContext('webgl2', { antialias: false, alpha: false, premultipliedAlpha: false });
  if (!gl) { document.getElementById('hint').textContent = 'WebGL2 unavailable'; render(); return; }
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
        a *= mix(1.0, uBoost, focus);
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
        float pg = min(pk, 1.5) * a * 0.9;
        acc += col * pg; w += pg;
      }
      for (int j = 0; j < ${MAXRIP}; j++) {
        if (j >= uNR) break;
        float age = uTime - uRip[j].z; if (age < 0.0 || age > 900.0) continue;
        float rr = age * 0.22; float dist = distance(p, uRip[j].xy);
        float ring = exp(-pow(dist - rr, 2.0) / (2.0*9.0)) * (1.0 - age/900.0) * 0.9;
        acc += uRipCol[j].rgb * ring; w += ring;
      }
      if (uLight > 0.5) {
        // light ground: additive washes out, so the halo multiplies the background toward the seat colour (darker than bg)
        vec3 tint = w > 0.001 ? acc / w : vec3(1.0);
        o = vec4(uBg * mix(vec3(1.0), tint, clamp(w * 1.1, 0.0, 0.8)), 1.0);
      } else {
        o = vec4(uBg + acc, 1.0);
      }
    }`;
  function sh(type, src) { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; }
  const prog = gl.createProgram(); gl.attachShader(prog, sh(gl.VERTEX_SHADER, vs)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, fs)); gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
  gl.useProgram(prog);
  const U = {}; for (const n of ['uRes', 'uTime', 'uBg', 'uN', 'uNR', 'uLight', 'uRect', 'uCol', 'uPar', 'uRip', 'uRipCol', 'uPulseMs', 'uActiveMs', 'uIdleA', 'uActMin', 'uActMax', 'uWaitMin', 'uWaitMax', 'uErrA', 'uBoost', 'uSpeed', 'uPSize', 'uAmber', 'uRed']) U[n] = gl.getUniformLocation(prog, n);
  gl.uniform3fv(U.uAmber, C.hslToRgb(T.color.state.waiting)); gl.uniform3fv(U.uRed, C.hslToRgb(T.color.state.errored));
  gl.uniform1f(U.uPulseMs, T.motion.pulse_ms); gl.uniform1f(U.uActiveMs, G.active_pulse_ms); gl.uniform1f(U.uIdleA, G.idle_alpha);
  gl.uniform1f(U.uActMin, G.active_alpha_min); gl.uniform1f(U.uActMax, G.active_alpha_max); gl.uniform1f(U.uWaitMin, G.waiting_alpha_min); gl.uniform1f(U.uWaitMax, G.waiting_alpha_max);
  gl.uniform1f(U.uErrA, G.errored_alpha); gl.uniform1f(U.uBoost, G.focused_boost); gl.uniform1f(U.uSpeed, G.particle_speed); gl.uniform1f(U.uPSize, G.particle_size * Math.min(devicePixelRatio, 2));

  const STATE_IDX = { idle: 0, active: 1, waiting: 2, errored: 3 };
  const rectBuf = new Float32Array(MAXR * 4), colBuf = new Float32Array(MAXR * 4), parBuf = new Float32Array(MAXR * 4);
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
      colBuf.set([rgb[0], rgb[1], rgb[2], s.key === focused ? 1 : 0], n * 4);
      const rad = (R.pane ? G.pane_radius : G.row_radius) * dpr;
      const parts = R.pane ? (s.state === 'active' ? G.particles_active : G.particles_idle * 2) : (s.state === 'active' ? 12 : 4);
      parBuf.set([STATE_IDX[s.state], rad, C.hash32(s.key) % 1000, parts], n * 4);
      n++;
    }
    gl.uniform3fv(U.uBg, C.hslToRgb(T.color[th].bg)); gl.uniform1f(U.uLight, th === 'light' ? 1 : 0);
    gl.uniform2f(U.uRes, cv.width, cv.height); gl.uniform1f(U.uTime, now);
    gl.uniform1i(U.uN, n); gl.uniform4fv(U.uRect, rectBuf); gl.uniform4fv(U.uCol, colBuf); gl.uniform4fv(U.uPar, parBuf);
    ripples.forEach((rp, j) => { ripBuf.set([rp.x * dpr, rp.y * dpr, rp.t0, 0], j * 4); ripCol.set([rp.c[0], rp.c[1], rp.c[2], 1], j * 4); });
    gl.uniform1i(U.uNR, ripples.length); gl.uniform4fv(U.uRip, ripBuf); gl.uniform4fv(U.uRipCol, ripCol);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    if (lastNow) { const ft = now - lastNow; ftAcc += ft; ftMax = Math.max(ftMax, ft); } lastNow = now;
    frames++; if (now - fpsT > 1000) { fps = frames; window.__fps = { fps, avgMs: +(ftAcc / frames).toFixed(2), maxMs: +ftMax.toFixed(1), rects: rects.length, scene: SCENE }; frames = 0; ftAcc = 0; ftMax = 0; fpsT = now; hint(); }
    requestAnimationFrame(frame);
  }
  function hint() { const f = window.__fps || {}; document.getElementById('hint').innerHTML = '<b>' + fps + '</b> fps · ' + (f.avgMs || '–') + ' ms avg · ' + (f.maxMs || '–') + ' ms max · ' + rects.length + ' rects · scene ' + SCENE + ' · hover or click a seat · <a href="?scene=' + (SCENE === 15 ? 3 : 15) + '">scene ' + (SCENE === 15 ? 3 : 15) + '</a>'; }

  // ---------- life ----------
  const CYCLE = { idle: 'active', active: 'waiting', waiting: 'active', errored: 'idle' };
  let tick = 0;
  setInterval(() => {
    tick++;
    const s = seats[Math.floor(r() * seats.length)];
    s.state = tick % 5 === 0 ? 'errored' : CYCLE[s.state];
    render();
    if (tick % 2 === 0 && performance.now() - lastMove > 3000) setFocus(stageKeys[Math.floor(r() * stageKeys.length)]);
  }, 4000);
  setInterval(() => {
    const s = seats[Math.floor(r() * seats.length)];
    const role = s.role || 'unsigned', col = C.hslToRgb(T.color.role[role]);
    for (const R of rects) if (R.key === s.key) ripple(R.pane ? R.x + 8 : R.x + 10, R.pane ? R.y + 8 : R.y + R.h / 2, col);
  }, 2600);

  render(); resize(); requestAnimationFrame(frame);
})();
