// Pane glow: the alive rim, the chosen look (operator 2026-10-23: "8px alive rim is the thing we want"; spec rim-glow).
// A 2 px rim just inside the pane border where the seat colour flows round as a moving hue gradient, two crest waves
// running in opposite directions, a bloom breathing from 2 px to glow.depth px inward (Settings › Glow depth, 2 to 8),
// and a pulse that quickens with activity. All of it lies inside the pane's 8 px padding, never over a text cell, and
// never moves layout. One shared WebGL2 context renders each pane and copies into that pane's own canvas (above the
// pane), so a page with many panes never hits the per-page context limit. Each pane runs its clock offset by phase_ms
// (cv.dataset.phaseMs) so neighbours do not move in step. Same equation as glow-inside.html's alive-rim.
// Legacy: { legacy: true } keeps the old particle emitter, only for the Glow material in materials.html.
(function () {
  const G = window.TOKENS.glow, M = window.TOKENS.motion;
  const STATE = {
    idle:    { n: G.particles_idle,   speed: 0.5, alpha: () => G.idle_alpha, tint: null },
    active:  { n: G.particles_active, speed: 1.0, alpha: t => lerp(G.active_alpha_min, G.active_alpha_max, breathe(t, G.active_pulse_ms)), tint: null },
    waiting: { n: G.particles_idle * 2, speed: 0.6, alpha: t => lerp(G.waiting_alpha_min, G.waiting_alpha_max, breathe(t, M.pulse_ms)), tint: window.TOKENS.color.state.waiting },
    errored: { n: G.particles_idle,   speed: 0.2, alpha: () => G.errored_alpha, tint: window.TOKENS.color.state.errored },
  };
  const lerp = (a, b, t) => a + (b - a) * t;
  const breathe = (t, ms) => 0.5 - 0.5 * Math.cos((t / ms) * Math.PI * 2);

  function particles(wrap, seat, opts) {
    const cv = document.createElement('canvas'); cv.className = 'particles'; wrap.prepend(cv);
    const ctx = cv.getContext('2d');
    const color = opts.color || window.C.seatColor(seat);
    let parts = [], last = 0;
    function conf() { return STATE[seat.state] || STATE.idle; }
    function resize() { cv.width = cv.offsetWidth * devicePixelRatio; cv.height = cv.offsetHeight * devicePixelRatio; }
    function spawn() { const c = conf(); parts = Array.from({ length: c.n }, () => ({ u: Math.random(), r: 0.5 + Math.random(), tw: Math.random() * 1000, life: 0.4 + Math.random() * 0.6 })); }
    function frame(ts) {
      if (!cv.isConnected) return;
      // reconnecting: the glow freezes on its last frame (reconnect_overlay)
      if (wrap.querySelector('.pane.stale')) { last = ts; requestAnimationFrame(frame); return; }
      const dt = last ? (ts - last) / 1000 : 0; last = ts;
      const c = conf(); if (parts.length !== c.n) spawn();
      resize();
      const w = cv.width, h = cv.height, pad = 24 * devicePixelRatio;
      const x0 = pad, y0 = pad, x1 = w - pad, y1 = h - pad, per = 2 * ((x1 - x0) + (y1 - y0));
      ctx.clearRect(0, 0, w, h);
      const a = Math.min(1, c.alpha(ts) * (opts.focused ? G.focused_boost : 1));
      ctx.fillStyle = c.tint || color;
      for (const p of parts) {
        p.u = (p.u + dt * G.particle_speed * c.speed * p.r / per * devicePixelRatio) % 1;
        const d = p.u * per; let x, y;
        if (d < x1 - x0) { x = x0 + d; y = y0; }
        else if (d < (x1 - x0) + (y1 - y0)) { x = x1; y = y0 + (d - (x1 - x0)); }
        else if (d < 2 * (x1 - x0) + (y1 - y0)) { x = x1 - (d - (x1 - x0) - (y1 - y0)); y = y1; }
        else { x = x0; y = y1 - (d - 2 * (x1 - x0) - (y1 - y0)); }
        const tw = 0.6 + 0.4 * Math.sin((ts + p.tw) / 300);
        const jitter = (Math.sin((ts + p.tw) / 700) * 3) * devicePixelRatio;
        ctx.globalAlpha = a * tw * p.life;
        ctx.beginPath(); ctx.arc(x + jitter, y + jitter, G.particle_size * devicePixelRatio * p.r, 0, Math.PI * 2); ctx.fill();
      }
      ctx.globalAlpha = 1;
      requestAnimationFrame(frame);
    }
    spawn(); requestAnimationFrame(frame);
    return cv;
  }
  // ---- alive rim ----
  const VS = `#version 300 es
  void main(){ vec2 p = vec2(gl_VertexID == 1 ? 3. : -1., gl_VertexID == 2 ? 3. : -1.); gl_Position = vec4(p, 0., 1.); }`;
  const FS = `#version 300 es
  precision highp float;
  uniform vec2 uOrigin, uSize; uniform float uR, uDepth, uDpr, uH, uTime, uAct, uFocus; uniform vec4 uCol;
  out vec4 o;
  const float TAU = 6.2831853;
  float sdf(vec2 q, vec2 hs, float r){ r = min(r, min(hs.x, hs.y)); vec2 d = abs(q) - hs + vec2(r); return min(max(d.x, d.y), 0.) + length(max(d, vec2(0.))) - r; }
  float perimLen(vec2 hs, float r0){ float r = max(r0, .001); vec2 b = hs - r; return 4.*(b.x + b.y) + TAU*r; }
  float perim(vec2 q, vec2 hs, float r0){
    float r = max(r0, .001); vec2 b = hs - r; float sx = 2.*b.x, sy = 2.*b.y, a = .25*TAU*r;
    if (abs(q.x) <= b.x) return q.y < 0. ? q.x + b.x : sx + 2.*a + sy + (b.x - q.x);
    if (abs(q.y) <= b.y) return q.x > 0. ? sx + a + (q.y + b.y) : 2.*sx + 3.*a + sy + (b.y - q.y);
    if (q.x > 0. && q.y < 0.) { vec2 v = q - vec2(b.x, -b.y); return sx + r*atan(v.x, -v.y); }
    if (q.x > 0.) { vec2 v = q - vec2(b.x, b.y); return sx + a + sy + r*atan(v.y, v.x); }
    if (q.y > 0.) { vec2 v = q - vec2(-b.x, b.y); return 2.*sx + 2.*a + sy + r*atan(-v.x, v.y); }
    vec2 v = q - vec2(-b.x, -b.y); return 2.*sx + 3.*a + 2.*sy + r*atan(-v.y, -v.x);
  }
  float wl(float lambda, float P){ return P / max(1., floor(P/lambda + .5)); }
  vec3 hue(vec3 c, float a){ const vec3 k = vec3(.57735); float ca = cos(a); return c*ca + cross(k, c)*sin(a) + k*dot(k, c)*(1. - ca); }
  void main(){
    vec2 p = vec2(gl_FragCoord.x, uH - gl_FragCoord.y) / uDpr - uOrigin;
    vec2 hs = uSize*.5, q = p - hs; float d = sdf(q, hs, uR);
    float rim = smoothstep(0., .6, -d)*(1. - smoothstep(1.6, 2.4, -d));
    float bloom = step(1.6, -d)*exp(-(-d - 2.)/(.3*uDepth))*(1. - smoothstep(uDepth - 1., uDepth, -d));
    if (rim + bloom <= 0.) discard;
    float P = perimLen(hs, uR), s = perim(q, hs, uR);
    float spd = 0., amp = .65;
    if (uAct == 0.) { spd = .35; amp = .32; } if (uAct == 1.) { spd = 1.; amp = 1.; } if (uAct == 2.) { spd = .6; amp = .9; }
    float t = uTime*.001*spd;
    float a1 = .5 + .5*sin(TAU*(s/wl(170., P) - t*.45)), a2 = .5 + .5*sin(TAU*(s/wl(110., P) + t*.30) + 1.7);
    float crest = max(a1*a1, .8*a2*a2), pulse = .5 - .5*cos(uTime/(2600./(.5 + spd))*TAU);
    float I = (.42 + .58*crest)*(.72 + .28*pulse)*rim + .3*crest*(.6 + .4*pulse)*bloom;
    vec3 c = hue(uCol.rgb, .55*sin(TAU*(s/P - t*.12)) + .3*sin(TAU*(s/wl(230., P) + t*.2)));
    c = clamp(mix(c, vec3(1.), .22*crest*rim), 0., 1.);
    I *= amp*mix(1., 1.3, uFocus);
    if (uAct == 2.) I *= .6 + .4*(.5 - .5*cos(uTime/1600.*TAU));
    I = clamp(I, 0., 1.) * uCol.a;
    o = vec4(c*I, I);
  }`;
  let GL = null;   // { gl, cv, prog, U }; null until first use, false when WebGL2 is unavailable
  function shared() {
    if (GL !== null) return GL;
    const cv = document.createElement('canvas'), gl = cv.getContext('webgl2', { premultipliedAlpha: true, antialias: false });
    if (!gl) return (GL = false);
    const sh = (type, src) => { const x = gl.createShader(type); gl.shaderSource(x, src); gl.compileShader(x); if (!gl.getShaderParameter(x, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(x)); return x; };
    const prog = gl.createProgram(); gl.attachShader(prog, sh(gl.VERTEX_SHADER, VS)); gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, FS)); gl.linkProgram(prog);
    const U = {}; for (const n of ['uOrigin', 'uSize', 'uR', 'uDepth', 'uDpr', 'uH', 'uTime', 'uAct', 'uFocus', 'uCol']) U[n] = gl.getUniformLocation(prog, n);
    return (GL = { gl, cv, prog, U });
  }
  // any CSS colour to [r, g, b] in 0..1
  const probe = document.createElement('canvas').getContext('2d');
  function rgb(css) { probe.clearRect(0, 0, 1, 1); probe.fillStyle = '#000'; probe.fillStyle = css; probe.fillRect(0, 0, 1, 1); const d = probe.getImageData(0, 0, 1, 1).data; return [d[0] / 255, d[1] / 255, d[2] / 255]; }
  const ACT = { idle: 0, active: 1, waiting: 2, errored: 3 };

  function rim(wrap, seat, opts) {
    const box = wrap.querySelector('.pane') || [...wrap.children].find(c => c.tagName !== 'CANVAS');
    const cv = document.createElement('canvas'); cv.className = 'rim'; wrap.append(cv);   // above the pane: the rim is inside it
    const ctx = cv.getContext('2d');
    const phase = opts.phase_ms != null ? opts.phase_ms : window.C.hash32(String(seat.key || '')) % 10000;
    cv.dataset.phaseMs = phase;
    let held = null;
    function frame(ts) {
      if (!cv.isConnected) return;
      // reconnecting: the rim freezes on its last frame (reconnect_overlay)
      if (held === null || !wrap.querySelector('.pane.stale')) held = ts;
      const S = shared(); if (!S || !box) return;
      const dpr = devicePixelRatio, cr = cv.getBoundingClientRect(), br = box.getBoundingClientRect();
      const w = Math.round(cr.width * dpr), h = Math.round(cr.height * dpr);
      if (cv.width !== w || cv.height !== h) { cv.width = w; cv.height = h; }
      const { gl, U } = S; S.cv.width = w; S.cv.height = h;
      const css = getComputedStyle(box), depthPx = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--glow-depth')) || window.TOKENS.glow.depth;
      // the cell edge rule (rim-glow §3): the band is [0, min(depth, padding)), so no pixel of the text grid is ever lit
      const term = box.querySelector('.term'), pad = term ? parseFloat(getComputedStyle(term).paddingLeft) : Infinity, depthEff = Math.min(depthPx, pad);
      const state = seat.state in ACT ? seat.state : 'idle';
      const root = getComputedStyle(document.documentElement), col = rgb(state === 'waiting' ? root.getPropertyValue('--c-waiting') : state === 'errored' ? root.getPropertyValue('--c-errored') : opts.color || window.C.seatColor(seat));
      gl.viewport(0, 0, w, h); gl.clearColor(0, 0, 0, 0); gl.clear(gl.COLOR_BUFFER_BIT); gl.useProgram(S.prog);
      gl.uniform2f(U.uOrigin, br.left - cr.left, br.top - cr.top); gl.uniform2f(U.uSize, br.width, br.height);
      gl.uniform1f(U.uR, parseFloat(css.borderTopLeftRadius) || 0); gl.uniform1f(U.uDepth, depthEff); gl.uniform1f(U.uDpr, dpr); gl.uniform1f(U.uH, h);
      gl.uniform1f(U.uTime, held + phase); gl.uniform1f(U.uAct, ACT[state]); gl.uniform1f(U.uFocus, opts.focused ? 1 : 0);
      gl.uniform4f(U.uCol, col[0], col[1], col[2], seat.grade === 'inferred' ? 0.5 : 1);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
      ctx.clearRect(0, 0, w, h); ctx.drawImage(S.cv, 0, 0);
      requestAnimationFrame(frame);
    }
    requestAnimationFrame(frame);
    return cv;
  }

  function attach(wrap, seat, opts) { opts = opts || {}; return opts.legacy ? particles(wrap, seat, opts) : rim(wrap, seat, opts); }
  window.Glow = { attach, STATE };
})();
