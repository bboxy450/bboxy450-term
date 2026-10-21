// Canvas fake of the border particle emitter. Sparse particles travel along the pane border in the seat colour;
// count and speed scale with activity; alpha breathes per state exactly like the CSS glow. Real one: wgpu shader.
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

  function attach(wrap, seat, opts) {
    opts = opts || {};
    const cv = document.createElement('canvas'); cv.className = 'particles'; wrap.prepend(cv);
    const ctx = cv.getContext('2d');
    const color = opts.color || window.C.seatColor(seat);
    let parts = [], last = 0;
    function conf() { return STATE[seat.state] || STATE.idle; }
    function resize() { cv.width = cv.offsetWidth * devicePixelRatio; cv.height = cv.offsetHeight * devicePixelRatio; }
    function spawn() { const c = conf(); parts = Array.from({ length: c.n }, () => ({ u: Math.random(), r: 0.5 + Math.random(), tw: Math.random() * 1000, life: 0.4 + Math.random() * 0.6 })); }
    function frame(ts) {
      if (!cv.isConnected) return;
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
  window.Glow = { attach, STATE };
})();
