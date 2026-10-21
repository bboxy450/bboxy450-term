// Lab chrome: applies tokens as CSS variables, builds the state strip and density/theme toggles.
// Pages call Lab.page({ title, note, states, render }). render(stateName, container, ctx) draws one state.
(function () {
  const T = window.TOKENS;
  const prefs = {
    get theme() { return localStorage.getItem('lab.theme') || 'dark'; },
    set theme(v) { localStorage.setItem('lab.theme', v); },
    get density() { return localStorage.getItem('lab.density') || 'comfortable'; },
    set density(v) { localStorage.setItem('lab.density', v); },
  };

  function applyTokens() {
    const r = document.documentElement.style;
    const c = T.color[prefs.theme];
    const d = T.density[prefs.density];
    const set = (k, v) => r.setProperty(k, v);
    set('--bg', c.bg); set('--surface', c.surface); set('--surface-2', c.surface_2); set('--line', c.line);
    set('--fg', c.fg); set('--fg-dim', c.fg_dim); set('--accent', c.accent); set('--accent-soft', c.accent_soft); set('--navy', c.navy);
    set('--c-idle', T.color.state.idle); set('--c-waiting', T.color.state.waiting); set('--c-errored', T.color.state.errored);
    for (const [k, v] of Object.entries(T.color.role)) set('--role-' + k, v);
    set('--row-h', d.row_h + 'px'); set('--tab-h', d.tab_h + 'px'); set('--glyph', d.glyph + 'px'); set('--icon', d.icon + 'px');
    set('--font', d.font + 'px'); set('--font-small', d.font_small + 'px'); set('--pad-x', d.pad_x + 'px'); set('--gap', d.gap + 'px');
    set('--r-pill', T.radius.pill + 'px'); set('--r-row', T.radius.row + 'px'); set('--r-chip', T.radius.chip + 'px'); set('--r-grid', T.radius.grid + 'px');
    set('--cohort-gap', T.layout.cohort_gap + 'px'); set('--sb-w', T.sidebar.width + 'px'); set('--sb-w-compact', T.sidebar.width_compact + 'px');
    set('--mg-w', T.sidebar.minigrid_w + 'px'); set('--mg-h', T.sidebar.minigrid_h + 'px');
    set('--strip-h', T.strip.height + 'px'); set('--strip-b-idle', T.strip.border_idle + 'px'); set('--strip-b-active', T.strip.border_active + 'px');
    const g = T.glow;
    set('--glow-pane-r', g.pane_radius + 'px'); set('--glow-row-r', g.row_radius + 'px'); set('--glow-idle-a', g.idle_alpha);
    set('--glow-active-min', g.active_alpha_min); set('--glow-active-max', g.active_alpha_max); set('--glow-active-ms', g.active_pulse_ms + 'ms');
    set('--glow-wait-min', g.waiting_alpha_min); set('--glow-wait-max', g.waiting_alpha_max); set('--glow-err-a', g.errored_alpha); set('--glow-boost', g.focused_boost);
    set('--pulse-ms', T.motion.pulse_ms + 'ms'); set('--ripple-ms', T.motion.ripple_ms + 'ms'); set('--hover-ms', T.motion.hover_ms + 'ms');
    set('--dim-sat', T.dim.saturate); set('--dim-opacity', T.dim.opacity); set('--dim-wash', T.dim.wash * 100 + '%');
    set('--font-chrome', T.font.chrome); set('--font-mono', T.font.mono);
    document.documentElement.className = 'theme-' + prefs.theme + ' density-' + prefs.density;
  }

  function el(tag, attrs, ...kids) {
    const n = document.createElement(tag);
    for (const [k, v] of Object.entries(attrs || {})) {
      if (k === 'class') n.className = v;
      else if (k === 'style') n.style.cssText = v;
      else if (k.startsWith('on')) n.addEventListener(k.slice(2), v);
      else if (k === 'html') n.innerHTML = v;
      else n.setAttribute(k, v);
    }
    for (const k of kids.flat()) if (k != null) n.append(k.nodeType ? k : document.createTextNode(k));
    return n;
  }

  function seg(options, current, onPick) {
    const s = el('div', { class: 'seg' });
    for (const o of options) s.append(el('button', { class: o === current ? 'on' : '', onclick: () => onPick(o) }, o));
    return s;
  }

  function page(spec) {
    applyTokens();
    let current = new URLSearchParams(location.search).get('state') || localStorage.getItem('lab.state.' + spec.title) || 'all';
    const root = document.body;
    root.innerHTML = '';
    const top = el('div', { class: 'lab-top' },
      el('a', { class: 'back', href: 'index.html' }, '← lab'),
      el('h1', {}, spec.title),
      el('div', { class: 'spacer' }),
      seg(['touch', 'comfortable', 'cozy', 'compact'], prefs.density, v => { prefs.density = v; page(spec); }),
      seg(['dark', 'light'], prefs.theme, v => { prefs.theme = v; page(spec); }),
    );
    const strip = el('div', { class: 'state-strip' });
    const names = ['all', ...spec.states];
    for (const s of names) strip.append(el('button', { class: 'chip' + (s === current ? ' on' : ''), onclick: () => { current = s; localStorage.setItem('lab.state.' + spec.title, s); history.replaceState(null, '', s === 'all' ? location.pathname : '?state=' + encodeURIComponent(s)); page(spec); } }, s));
    const main = el('div', { class: 'lab-main' + (current === 'all' && !spec.wide ? '' : ' single') });
    const ctx = { theme: prefs.theme, density: prefs.density, tokens: T, el };
    const draw = (s) => {
      const body = el('div', { class: 'body' });
      const card = el('div', { class: 'card' + (spec.checker ? ' checker' : '') }, el('div', { class: 'label' }, el('span', {}, s), el('span', {}, prefs.density)), body);
      spec.render(s, body, ctx);
      return card;
    };
    if (current === 'all') for (const s of spec.states) main.append(draw(s)); else main.append(draw(current));
    root.append(top, strip);
    if (spec.note) root.append(el('div', { class: 'lab-note', html: spec.note }));
    root.append(main);
  }

  window.Lab = { page, el, applyTokens, prefs, T };
})();
