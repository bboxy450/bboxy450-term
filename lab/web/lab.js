// Lab chrome: applies tokens as CSS variables, builds the state strip and density/theme toggles.
// Pages call Lab.page({ title, note, states, render }). render(stateName, container, ctx) draws one state.
(function () {
  const T = window.TOKENS;
  // storage that never throws (private windows, blocked site data)
  const store = { get(k) { try { return window.localStorage.getItem(k); } catch { return null; } }, set(k, v) { try { window.localStorage.setItem(k, v); } catch {} } };
  // ?theme= and ?density= win over storage so headless runs (scripts/parity.sh) can deep-link; a click on the toggle drops the override
  const url = new URLSearchParams(location.search), pick = (k, ok) => { const v = url.get(k); return v && ok[v] ? v : null; };
  const prefs = {
    get theme() { return pick('theme', T.color) || store.get('lab.theme') || 'dark'; },
    set theme(v) { url.delete('theme'); store.set('lab.theme', v); },
    get density() { return pick('density', T.density) || store.get('lab.density') || 'comfortable'; },
    set density(v) { url.delete('density'); store.set('lab.density', v); },
  };

  function applyTokens() {
    const r = document.documentElement.style;
    const c = T.color[prefs.theme];
    const d = T.density[prefs.density];
    const set = (k, v) => r.setProperty(k, v);
    set('--bg', c.bg); set('--surface', c.surface); set('--surface-2', c.surface_2); set('--line', c.line);
    set('--fg', c.fg); set('--fg-dim', c.fg_dim); set('--accent', c.accent); set('--accent-soft', c.accent_soft); set('--navy', c.navy); set('--sidebar-bg', c.sidebar);
    set('--c-idle', T.color.state.idle); set('--on-accent', T.color.ui.on_accent); set('--c-ok', T.color.ui.ok); set('--c-invariant', T.color.ui.invariant); set('--c-waiting', T.color.state.waiting); set('--c-errored', T.color.state.errored);
    for (const [k, v] of Object.entries(T.color.role)) set('--role-' + k, v);
    set('--row-h', d.row_h + 'px'); set('--tab-h', d.tab_h + 'px'); set('--glyph', d.glyph + 'px'); set('--icon', d.icon + 'px');
    set('--font', d.font + 'px'); set('--font-small', d.font_small + 'px'); set('--pad-x', d.pad_x + 'px'); set('--gap', d.gap + 'px');
    set('--r-pill', (window.STYLE ? STYLE.radius.card : 12) + 'px'); set('--r-chip', T.radius.chip + 'px'); set('--r-grid', T.radius.grid + 'px');
    set('--cohort-gap', T.layout.cohort_gap + 'px'); set('--gutter', T.layout.gutter + 'px'); set('--sb-w', T.sidebar.width + 'px'); set('--sb-w-compact', T.sidebar.width_compact + 'px');
    set('--mg-w', T.sidebar.minigrid_w + 'px'); set('--mg-h', T.sidebar.minigrid_h + 'px');
    set('--strip-h', T.strip.height + 'px'); set('--strip-b-idle', T.strip.border_idle + 'px'); set('--strip-b-active', T.strip.border_active + 'px');
    const g = T.glow;
    set('--glow-pane-r', g.pane_radius + 'px'); set('--glow-row-r', g.row_radius + 'px'); set('--glow-idle-a', g.idle_alpha);
    set('--glow-active-min', g.active_alpha_min); set('--glow-active-max', g.active_alpha_max); set('--glow-active-ms', g.active_pulse_ms + 'ms');
    set('--glow-wait-min', g.waiting_alpha_min); set('--glow-wait-max', g.waiting_alpha_max); set('--glow-err-a', g.errored_alpha); set('--glow-boost', g.focused_boost);
    set('--pulse-ms', T.motion.pulse_ms + 'ms'); set('--ripple-ms', T.motion.ripple_ms + 'ms'); set('--hover-ms', T.motion.hover_ms + 'ms');
    set('--dim-sat', T.dim.saturate); set('--dim-opacity', T.dim.opacity); set('--dim-wash', T.dim.wash * 100 + '%');
    set('--font-chrome', T.font.chrome); set('--font-mono', T.font.mono);
    // style.json → CSS vars: one radius scale, one material, one type scale, one tint
    const S = window.STYLE; if (S) {
      for (const [k, v] of Object.entries(S.radius)) set('--r-' + (k === 'pill' ? 'round' : k), v + 'px');   // --r-pill (12) stays the lab chrome's card radius; --r-round is the 999 pill
      set('--mat-alpha', S.material.panel_alpha * 100 + '%'); set('--card-alpha', S.material.card_alpha * 100 + '%'); set('--mat-blur', S.material.panel_blur_px + 'px'); set('--mat-sat', S.material.panel_saturate);
      set('--back-dim', S.material.backdrop_dim_alpha * 100 + '%'); set('--back-blur', S.material.backdrop_blur_px + 'px');
      set('--hair', 'color-mix(in srgb, var(--line) ' + S.material.hairline_alpha * 100 + '%, transparent)'); set('--hi', 'rgba(255,255,255,' + S.material.inner_highlight_alpha + ')');
      set('--sel', 'color-mix(in srgb, var(--fg) ' + S.tint.selection_alpha * 100 + '%, transparent)'); set('--sel-hover', 'color-mix(in srgb, var(--fg) ' + S.tint.selection_hover_alpha * 100 + '%, transparent)');
      set('--badge', 'color-mix(in srgb, var(--fg-dim) ' + S.tint.badge_alpha * 100 + '%, transparent)'); set('--badge-sel', 'color-mix(in srgb, var(--fg-dim) ' + S.tint.badge_selected_alpha * 100 + '%, transparent)');
      for (const [k, v] of Object.entries(S.type)) if (typeof v === 'number') set('--t-' + k, /^weight_/.test(k) ? v : v + 'px');   // weights are unitless: '600px' is an invalid font-weight
      set('--sh-float', S.shadow.floating); set('--sh-pill', S.shadow.pill); set('--appear-ms', S.motion.appear_ms + 'ms'); set('--ease', S.motion.ease);
      set('--grid', S.spacing.grid + 'px'); for (const [k, v] of Object.entries(S.spacing)) if (/^s\d+$/.test(k)) set('--' + k, v + 'px'); set('--hair-w', S.material.hairline_px + 'px'); set('--row-mx', S.spacing.row_margin_x + 'px'); set('--row-px', S.spacing.row_pad_x + 'px'); set('--title-bar-h', S.spacing.title_bar + 'px'); set('--band-h', S.spacing.band + 'px'); set('--new-swarm-h', S.spacing.new_swarm + 'px'); set('--doc-measure', S.spacing.doc_measure + 'px'); set('--empty-measure', S.spacing.empty_measure + 'px');
    }
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
    // ?state= takes the display label or its slug ("subagents · claude" or "subagents-claude"); a miss falls back to "all" and logs
    const slug = x => String(x).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    const resolve = want => { if (!want) return null; const hit = ['all', ...spec.states].find(x => x === want || slug(x) === slug(want)); if (!hit) console.warn('lab: unknown state "' + want + '" on ' + spec.title + '; showing all'); return hit || 'all'; };
    let current = resolve(new URLSearchParams(location.search).get('state')) || resolve(store.get('lab.state.' + spec.title)) || 'all';
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
    for (const s of names) strip.append(el('button', { class: 'chip' + (s === current ? ' on' : ''), onclick: () => { current = s; store.set('lab.state.' + spec.title, s); history.replaceState(null, '', s === 'all' ? location.pathname : '?state=' + slug(s)); page(spec); } }, s));
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

  window.Lab = { page, el, applyTokens, prefs, T, store };
})();
