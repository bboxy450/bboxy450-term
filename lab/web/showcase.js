// Showcase: full-window mock. Sidebar + three panes in the DOM; the border glow is a WebGL2 fragment shader on a
// full-window canvas behind the DOM. Per seat: rect, colour, state, focus. Particles advect along the border with
// additive blending; ripples expand from a sidebar row and the matching pane corner. A timer keeps the scene alive.
(function () {
  const T = window.TOKENS, G = T.glow, el = Lab.el;
  Lab.prefs.theme = 'dark'; Lab.prefs.density = 'comfortable'; Lab.applyTokens();
  document.body.style.background = T.color.dark.bg;

  // ---------- model: two spaces, eight seats, three of them on stage in a V(H) split ----------
  const r = C.rng('showcase');
  const mk = (i, name, harness, role, title, state) => ({ key: 'sc' + i, name, harness, role, title, state });
  const seats = [
    mk(0, 'Vesper Greywind', 'claude', 'oracle', 'cohort control', 'active'),
    mk(1, 'Nyx Copperwing', 'codex', 'executor', 'messaging spec', 'active'),
    mk(2, 'Lumen Foxglove', 'claude', 'creativity', 'ux lab', 'idle'),
    mk(3, 'Rook Ashwhisker', 'codex', 'executor', 'gpu research', 'waiting'),
    mk(4, 'Sable Quill', 'codex', 'executor', 'harness hooks', 'idle'),
    mk(5, 'Pip Hollow', 'pi', 'executor', 'relay bench', 'errored'),
    mk(6, 'Marrow Finch', 'omp', 'executor', 'parse logs', 'idle'),
    mk(7, 'zsh', 'zsh', null, '~/mydev', 'idle'),
  ];
  const stageKeys = ['sc0', 'sc1', 'sc3'];
  const model = { spaces: [
    { name: 'term', key: 'space-term', tabs: [
      { name: 'cohort', shortcut: '⌘1', tree: { dir: 'v', a: seats[0], b: { dir: 'h', a: seats[1], b: seats[3] } }, seats: [seats[0], seats[1], seats[3]] },
      { name: 'ux lab', shortcut: '⌘2', tree: seats[2], seats: [seats[2]] },
      { name: 'hooks', shortcut: '⌘3', tree: seats[4], seats: [seats[4]] },
    ] },
    { name: 'winnow', key: 'space-winnow', tabs: [
      { name: 'relay', shortcut: '⌘4', tree: { dir: 'v', a: seats[5], b: seats[6] }, seats: [seats[5], seats[6]] },
      { name: 'shell', shortcut: '⌘5', tree: seats[7], seats: [seats[7]] },
    ] },
  ] };
  let focused = 'sc0';

  // ---------- DOM ----------
  const app = document.getElementById('app');
  const titlebar = el('div', { class: 'titlebar' }, el('span', { class: 'win' }, el('i'), el('i'), el('i')), el('span', { class: 'tbtn on' }, C.harnessIcon('sidebarfoxopen')), C.logo(18, { ears: false }), el('span', { class: 'ttl' }, 'bboxy450-term'), el('span', { id: 'crumb' }), el('span', { class: 'spacer' }), el('span', { class: 'tbtn' }, C.harnessIcon('gear')));
  const main = el('div', { id: 'main' });
  app.append(titlebar, main);
  let rects = [];
  function render() {
    main.innerHTML = '';
    const sb = C.sidebar(model, { openSpaces: model.spaces.map(s => s.key), openTabs: model.spaces.flatMap(s => s.tabs.map(t => s.key + '/' + t.name)), focusedKey: focused });
    const stage = el('div', { class: 'stage' });
    for (const k of stageKeys) { const s = seats.find(x => x.key === k); stage.append(C.pane(s, { focused: k === focused, unfocused: k !== focused, shortcut: '⌘' + (stageKeys.indexOf(k) + 1) })); }
    main.append(sb, stage);
    const f = seats.find(s => s.key === focused);
    document.getElementById('crumb').textContent = '· term · ' + f.name;
    main.querySelectorAll('[data-key]').forEach(n => n.addEventListener('click', () => { focused = n.dataset.key; render(); }));
    measure();
  }
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
  const MAXR = 16, MAXRIP = 6;
  const vs = `#version 300 es
    const vec2 q[3] = vec2[3](vec2(-1.,-1.), vec2(3.,-1.), vec2(-1.,3.));
    void main(){ gl_Position = vec4(q[gl_VertexID], 0., 1.); }`;
  const fs = `#version 300 es
    precision highp float;
    uniform vec2 uRes; uniform float uTime; uniform vec3 uBg; uniform int uN; uniform int uNR;
    uniform vec4 uRect[${MAXR}];   // x y w h (css px, y down)
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
      vec3 acc = vec3(0.);
      for (int i = 0; i < ${MAXR}; i++) {
        if (i >= uN) break;
        vec4 R = uRect[i]; vec2 c = R.xy + R.zw*0.5; vec2 hs = R.zw*0.5;
        vec2 q = p - c; vec2 d2 = abs(q) - hs;
        float dOut = length(max(d2, 0.)); float dIn = min(max(d2.x, d2.y), 0.);
        float d = abs(dOut + dIn);                    // distance to the border line
        float rad = uPar[i].y;
        if (d > rad * 4.0) continue;
        float st = uPar[i].x; vec3 col = uCol[i].rgb; float focus = uCol[i].w;
        float a; float speed = 0.5; float count = uPar[i].w;
        if (st < 0.5)      { a = uIdleA; speed = 0.35; }
        else if (st < 1.5) { a = mix(uActMin, uActMax, breathe(uActiveMs)); speed = 1.0; }
        else if (st < 2.5) { a = mix(uWaitMin, uWaitMax, breathe(uPulseMs)); col = uAmber; speed = 0.6; }
        else               { a = uErrA; col = uRed; speed = 0.15; count *= 0.5; }
        a *= mix(1.0, uBoost, focus);
        // soft glow: exponential falloff from the border, slightly thinner inside the rect
        float inside = step(dOut, 0.0);
        float g = exp(-d / (rad * mix(1.0, 0.45, inside)));
        acc += col * g * a * 0.55;
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
          pk += exp(-(dt*dt)/(2.0*sig*sig*2.2)) * exp(-(d*d)/(2.0*sig*sig)) * tw;
        }
        acc += col * pk * a * 1.6;
      }
      for (int j = 0; j < ${MAXRIP}; j++) {
        if (j >= uNR) break;
        float age = uTime - uRip[j].z; if (age < 0.0 || age > 900.0) continue;
        float rr = age * 0.22; float dist = distance(p, uRip[j].xy);
        float ring = exp(-pow(dist - rr, 2.0) / (2.0*9.0)) * (1.0 - age/900.0);
        acc += uRipCol[j].rgb * ring * 0.9;
      }
      o = vec4(uBg + acc, 1.0);
    }`;
  function sh(type, src) { const s = gl.createShader(type); gl.shaderSource(s, src); gl.compileShader(s); if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s)); return s; }
  const prog = gl.createProgram(); gl.attachShader(prog, sh(gl.VERTEX_SHADER, vs)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, fs)); gl.linkProgram(prog);
  if (!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
  gl.useProgram(prog);
  const U = {}; for (const n of ['uRes', 'uTime', 'uBg', 'uN', 'uNR', 'uRect', 'uCol', 'uPar', 'uRip', 'uRipCol', 'uPulseMs', 'uActiveMs', 'uIdleA', 'uActMin', 'uActMax', 'uWaitMin', 'uWaitMax', 'uErrA', 'uBoost', 'uSpeed', 'uPSize', 'uAmber', 'uRed']) U[n] = gl.getUniformLocation(prog, n);
  const bg = C.hslToRgb(T.color.dark.bg);
  gl.uniform3fv(U.uBg, bg); gl.uniform3fv(U.uAmber, C.hslToRgb(T.color.state.waiting)); gl.uniform3fv(U.uRed, C.hslToRgb(T.color.state.errored));
  gl.uniform1f(U.uPulseMs, T.motion.pulse_ms); gl.uniform1f(U.uActiveMs, G.active_pulse_ms); gl.uniform1f(U.uIdleA, G.idle_alpha);
  gl.uniform1f(U.uActMin, G.active_alpha_min); gl.uniform1f(U.uActMax, G.active_alpha_max); gl.uniform1f(U.uWaitMin, G.waiting_alpha_min); gl.uniform1f(U.uWaitMax, G.waiting_alpha_max);
  gl.uniform1f(U.uErrA, G.errored_alpha); gl.uniform1f(U.uBoost, G.focused_boost); gl.uniform1f(U.uSpeed, G.particle_speed); gl.uniform1f(U.uPSize, G.particle_size * 1.4);

  const STATE_IDX = { idle: 0, active: 1, waiting: 2, errored: 3 };
  const rectBuf = new Float32Array(MAXR * 4), colBuf = new Float32Array(MAXR * 4), parBuf = new Float32Array(MAXR * 4);
  const ripples = []; const ripBuf = new Float32Array(MAXRIP * 4), ripCol = new Float32Array(MAXRIP * 4);
  function ripple(x, y, color) { ripples.push({ x, y, t0: performance.now(), c: color }); if (ripples.length > MAXRIP) ripples.shift(); }

  function resize() { const dpr = Math.min(devicePixelRatio, 2); cv.width = innerWidth * dpr; cv.height = innerHeight * dpr; gl.viewport(0, 0, cv.width, cv.height); measure(); }
  addEventListener('resize', resize);

  let frames = 0, fpsT = performance.now(), fps = 0;
  function frame(now) {
    const dpr = cv.width / innerWidth;
    let n = 0;
    for (const R of rects) {
      if (n >= MAXR) break;
      const s = R.seat, rgb = C.hslToRgb(C.seatColor(s));
      rectBuf.set([R.x * dpr, R.y * dpr, R.w * dpr, R.h * dpr], n * 4);
      colBuf.set([rgb[0], rgb[1], rgb[2], s.key === focused ? 1 : 0], n * 4);
      const rad = (R.pane ? G.pane_radius : G.row_radius) * dpr;
      const parts = R.pane ? (s.state === 'active' ? G.particles_active : G.particles_idle * 2) : (s.state === 'active' ? 12 : 4);
      parBuf.set([STATE_IDX[s.state], rad, C.hash32(s.key) % 1000, parts], n * 4);
      n++;
    }
    gl.uniform2f(U.uRes, cv.width, cv.height); gl.uniform1f(U.uTime, now);
    gl.uniform1i(U.uN, n); gl.uniform4fv(U.uRect, rectBuf); gl.uniform4fv(U.uCol, colBuf); gl.uniform4fv(U.uPar, parBuf);
    ripples.forEach((rp, j) => { ripBuf.set([rp.x * dpr, rp.y * dpr, rp.t0, 0], j * 4); ripCol.set([rp.c[0], rp.c[1], rp.c[2], 1], j * 4); });
    gl.uniform1i(U.uNR, ripples.length); gl.uniform4fv(U.uRip, ripBuf); gl.uniform4fv(U.uRipCol, ripCol);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    frames++; if (now - fpsT > 1000) { fps = frames; frames = 0; fpsT = now; hint(); }
    requestAnimationFrame(frame);
  }
  function hint() { document.getElementById('hint').innerHTML = '<b>' + fps + '</b> fps · ' + rects.length + ' rects · click a seat to focus · states cycle every 4 s'; }

  // ---------- life: cycle states, move focus, send messages ----------
  const CYCLE = { idle: 'active', active: 'waiting', waiting: 'active', errored: 'idle' };
  let tick = 0;
  setInterval(() => {
    tick++;
    const s = seats[Math.floor(r() * seats.length)];
    s.state = tick % 5 === 0 ? 'errored' : CYCLE[s.state];
    if (tick % 2 === 0) focused = stageKeys[Math.floor(r() * stageKeys.length)];
    render();
  }, 4000);
  setInterval(() => {
    const s = seats[Math.floor(r() * seats.length)];
    const role = s.role || 'unsigned', col = C.hslToRgb(T.color.role[role]);
    for (const R of rects) if (R.key === s.key) ripple(R.pane ? R.x + 8 : R.x + 10, R.pane ? R.y + 8 : R.y + R.h / 2, col);
  }, 2600);

  render(); resize(); requestAnimationFrame(frame);
})();
