// ui.js — the web reference for bbt-ui (gui-components.md §2b, §2c): L2 materials, L3 Style, L5/L6 components.
// Pages built on it (materials.html, style.html, components.html) use only these names, so a native gallery case and its
// web reference are the same composition. Everything resolves from tokens.js (TOKENS + STYLE); nothing is drawn ad hoc.
(function () {
  const T = window.TOKENS, S = window.STYLE, el = (tag, attrs, ...kids) => window.Lab.el(tag, attrs, ...kids);
  const px = v => v + 'px';

  // ---------- scopes: render dark and light, or four densities, side by side on one page ----------
  function themeScope(theme, ...kids) {
    const c = T.color[theme], vars = { '--bg': c.bg, '--surface': c.surface, '--surface-2': c.surface_2, '--line': c.line, '--fg': c.fg, '--fg-dim': c.fg_dim, '--accent': c.accent, '--accent-soft': c.accent_soft, '--sidebar-bg': c.sidebar };
    const d = el('div', { class: 'ui-scope theme-' + theme, 'data-theme': theme }, ...kids);
    for (const k in vars) d.style.setProperty(k, vars[k]);
    d.style.colorScheme = theme; d.style.background = c.bg; d.style.color = c.fg;
    return d;
  }
  function densityScope(name, ...kids) {
    const d = T.density[name], s = el('div', { class: 'ui-scope density-' + name, 'data-density': name }, ...kids);
    s.style.setProperty('--row-h', px(d.row_h)); s.style.setProperty('--icon', px(d.icon)); s.style.setProperty('--font', px(d.font)); s.style.setProperty('--pad-x', px(d.pad_x)); s.style.setProperty('--gap', px(d.gap));
    return s;
  }

  // ---------- L2 materials: flat, gloss, frosted, glow, with the native parameter names ----------
  const mix = (a, b, pct) => `color-mix(in srgb, ${a} ${100 - pct}%, ${b})`;
  const MATERIAL = {
    flat: o => ({ params: { color: o.color }, css: { background: o.color } }),
    // the swarm band: top lifted toward white, bottom toward black, a 1 px inner highlight and a bottom shade (lab .space-head)
    gloss: o => { const p = { color: o.color, top_white: o.top_white ?? 0.12, bottom_black: o.bottom_black ?? 0.18, highlight: o.highlight ?? 0.18, bottom_shade: o.bottom_shade ?? 0.25 };
      return { params: p, css: { background: `linear-gradient(180deg, ${mix(p.color, '#fff', p.top_white * 100)} 0%, ${p.color} 55%, ${mix(p.color, '#000', p.bottom_black * 100)} 100%)`, boxShadow: `inset 0 1px 0 rgba(255,255,255,${p.highlight}), inset 0 -1px 0 rgba(0,0,0,${p.bottom_shade})` } }; },
    // L2 is effects only (Oracle, decided): fill and border belong to Style/Box. frosted = blur and saturate what is behind;
    // the panel tint and the hairline ring are the modal Style's fill and border (styleCss), not the material.
    frosted: o => { const p = { tint: o.tint || 'none (Style fill)', blur_radius: o.blur_radius ?? S.material.panel_blur_px, saturation: o.saturation ?? S.material.panel_saturate };
      const css = { backdropFilter: `blur(${p.blur_radius}px) saturate(${p.saturation})`, webkitBackdropFilter: `blur(${p.blur_radius}px) saturate(${p.saturation})` }; if (o.tint) css.background = o.tint;
      return { params: p, css }; },
    // glow = the state-driven particle border (glow.js, the pane emitter) and its soft halo; the surface and the 1.5 px seat border
    // are the pane-glow Style's fill and border
    glow: o => { const g = T.glow, p = { color: o.color, activity: o.activity || 'active', radius: g.pane_radius, particles: g.particles_active ?? g.particles, speed: g.particle_speed, particle_size: g.particle_size, idle_alpha: g.idle_alpha, active_alpha: [g.active_alpha_min, g.active_alpha_max], focused_boost: g.focused_boost };
      return { params: p, css: { boxShadow: `0 0 ${g.pane_radius}px color-mix(in srgb, ${p.color} 45%, transparent)` }, glow: p }; },
  };
  function material(name, opts) { return MATERIAL[name](opts || {}); }

  // ---------- L3 Style: tokens + overrides, data you can print ----------
  const R = S.radius;
  const STYLES = {
    card: { material: 'flat', fill: 'material.card_alpha', radius: R.card, border: 'hairline outside', padding: [S.spacing.s6, S.spacing.s7], text: 'body' },
    row: { material: 'flat', fill: 'transparent', radius: R.row, height: 'density.row_h', padding: [0, 'density.pad_x'], text: 'row', selection: 'tint.selection_alpha' },
    band: { material: 'gloss', fill: 'swarm colour', radius: R.tile, height: S.spacing.band, padding: [0, S.spacing.s6], text: 'note · 600 · tracked .16em · uppercase · on_accent' },
    // the swarm card (lab .space): the card body, rows set on it, the swarm colour as a bumper; the showcase is the source (Oracle, decided)
    'swarm-card': { material: 'flat', fill: 'material.card_alpha', radius: R.card, border: 'hairline outside', padding: [S.spacing.s2, S.spacing.s2, 0], body: [S.spacing.s1, 0, S.spacing.s2], bumper: 'swarm 4px (closed 3px)', text: 'row' },
    'card-row': { material: 'flat', fill: 'transparent', radius: R.row, height: 'density.row_h', padding: [0, S.spacing.s4], margin: [0, S.spacing.s1], text: 'row', selection: 'tint.selection_alpha' },
    'new-swarm': { material: 'flat', fill: 'material.card_alpha', radius: R.card, border: 'hairline outside', height: S.spacing.new_swarm, text: 'row · 500 · fg_dim' },
    pill: { material: 'flat', fill: 'tint.badge_alpha', radius: R.pill, padding: [0, S.spacing.s3], text: 'tiny' },
    chip: { material: 'flat', fill: 'tint.badge_alpha', radius: R.tag, padding: [0, S.spacing.s2], text: 'micro · mono' },
    modal: { material: 'frosted', fill: 'material.panel_alpha', radius: R.panel, border: 'hairline ring', shadow: 'shadow.floating', padding: [S.spacing.s8, S.spacing.s9], text: 'body' },
    // the panel at instrument size: the perf overlay (Cmd+Opt+F)
    hud: { material: 'frosted', fill: 'material.panel_alpha', radius: R.card, border: 'hairline ring', shadow: 'shadow.floating', padding: [S.spacing.s5, S.spacing.s6], text: 'micro · mono' },
    strip: { material: 'flat', fill: 'seat colour', radius: 0, height: T.strip.height, padding: [0, S.spacing.s4], text: 'caption · 600 · on_accent' },
    input: { material: 'flat', fill: 'var(--bg)', radius: R.row, border: 'hairline', padding: [S.spacing.s4, S.spacing.s5], text: 'body', focus: 'accent ring' },
    'button-primary': { material: 'flat', fill: 'accent', radius: R.pill, padding: [S.spacing.s3, S.spacing.s8], text: 'row · 600 · on_accent' },
    'button-ghost': { material: 'flat', fill: 'transparent', radius: R.pill, border: 'hairline', padding: [S.spacing.s3, S.spacing.s8], text: 'row' },
    'pane-glow': { material: 'glow', fill: 'var(--surface)', radius: R.row, border: 'seat 1.5px', padding: [S.spacing.s6, S.spacing.s7], text: 'body' },
  };
  const resolve = v => typeof v === 'number' ? px(v) : v === 'density.row_h' ? 'var(--row-h)' : v === 'density.pad_x' ? 'var(--pad-x)' : v;
  function styleCss(st, swarm) {
    const css = {};
    const fill = { 'var(--surface)': 'var(--surface)', transparent: 'transparent', 'tint.badge_alpha': 'var(--badge)', 'var(--bg)': 'var(--bg)', accent: 'var(--accent)', 'seat colour': swarm || 'var(--accent)', 'swarm colour': swarm || 'var(--accent)', 'material.panel_alpha': null, 'material.card_alpha': null }[st.fill];
    if (st.material === 'gloss') Object.assign(css, material('gloss', { color: swarm || 'var(--accent)' }).css);
    else { if (st.material === 'frosted' || st.material === 'glow') Object.assign(css, material(st.material, { color: swarm || 'var(--accent)' }).css); if (fill) css.background = fill; }
    if (st.fill === 'material.card_alpha') css.background = `color-mix(in srgb, var(--surface) ${S.material.card_alpha * 100}%, transparent)`;
    if (st.fill === 'material.panel_alpha') css.background = `color-mix(in srgb, var(--surface) ${S.material.panel_alpha * 100}%, transparent)`;
    css.borderRadius = px(st.radius);
    if (st.border === 'hairline') css.border = 'var(--hair-w) solid var(--hair, var(--line))';
    if (st.border === 'hairline outside') css.boxShadow = `0 0 0 ${S.material.hairline_px}px var(--hair, var(--line))`;
    if (st.border === 'hairline ring') css.boxShadow = `0 0 0 ${S.material.hairline_px}px var(--hair, var(--line)), inset 0 1px 0 rgba(255,255,255,${S.material.inner_highlight_alpha})`;
    if (st.border === 'seat 1.5px') css.border = '1.5px solid ' + (swarm || 'var(--accent)');
    if (st.shadow) css.boxShadow = (css.boxShadow ? css.boxShadow + ', ' : '') + 'var(--sh-float)';
    if (st.height) css.height = resolve(st.height);
    if (st.padding) css.padding = st.padding.map(resolve).join(' ');
    if (st.margin) css.margin = st.margin.map(resolve).join(' ');
    return css;
  }

  // the Style record's text field ('row · 600 · on_accent', 'micro · mono', 'small · 600 · tracked .22em · uppercase · on_accent')
  // resolves to Text options, so a page cannot drift from the record (Nyx parity correction: buttons declare row, not body)
  function textOpts(st) { const t = (st && st.text) || 'body'; return { step: t.split(' · ')[0].trim(), weight: +(t.match(/\b([4-7]00)\b/) || [])[1] || null, mono: /\bmono\b/.test(t), caps: /uppercase/.test(t), track: +(t.match(/tracked ([\d.]+)em/) || [])[1] || null, color: /on_accent/.test(t) ? 'var(--on-accent)' : /fg_dim/.test(t) ? 'var(--fg-dim)' : null }; }
  // ---------- L6 components ----------
  const apply = (node, css) => { for (const k in css) node.style[k] = css[k]; return node; };
  function Box(style, ...kids) { const st = typeof style === 'string' ? STYLES[style] : style; const b = el('div', { class: 'ui-box' + (typeof style === 'string' ? ' ui-s-' + style : '') }, ...kids); apply(b, styleCss(st, style && style.swarm)); return b; }
  const STEPS = ['micro', 'tiny', 'small', 'note', 'caption', 'label', 'section', 'row', 'body', 'kind', 'subtitle', 'title', 'input'];
  // public constructor options, consumed as-is by native (never child measurements):
  //   Text  { step, weight, mono, dim, color, caps, track, align: start | center | end }
  //   Stack { dir: column | row, gap, rowGap, columnGap, align: cross axis, justify: start | center | space-between, wrap }
  // Path { points: [{move:[x,y]}, {line:[x,y]}, {cubic:[[cx1,cy1],[cx2,cy2],[x,y]]}], size: [w,h],
  //        stroke: { width, color: theme token (fg, fg_dim, accent, line) or literal, opacity, dash: [on, off, ...], cap: butt | round | square, join: miter | round | bevel } }
  // A retained vector edge, stroke only: circles and pills stay Box. Native tessellates the same record (lyon, analytic AA).
  const SVG = 'http://www.w3.org/2000/svg', PATH_COLOR = { fg: '--fg', fg_dim: '--fg-dim', accent: '--accent', line: '--line' };
  function Path(o) {
    const [w, h] = o.size, st = o.stroke || {}, svg = document.createElementNS(SVG, 'svg'), path = document.createElementNS(SVG, 'path');
    const d = (o.points || []).map(p => p.move ? 'M' + p.move : p.line ? 'L' + p.line : 'C' + p.cubic.join(' ')).join(' ');
    svg.setAttribute('class', 'ui-path'); svg.setAttribute('width', w); svg.setAttribute('height', h); svg.setAttribute('viewBox', '0 0 ' + w + ' ' + h); svg.style.display = 'block'; svg.style.overflow = 'visible';
    const a = { d, fill: 'none', stroke: PATH_COLOR[st.color] ? 'var(' + PATH_COLOR[st.color] + ')' : st.color || 'currentColor', 'stroke-width': st.width ?? 1, 'stroke-opacity': st.opacity ?? 1, 'stroke-linecap': st.cap || 'butt', 'stroke-linejoin': st.join || 'miter' };
    if (st.dash && st.dash.length) a['stroke-dasharray'] = st.dash.join(' ');
    for (const k in a) path.setAttribute(k, a[k]);
    svg.append(path); return svg;
  }
  // Select { value, onChange }: the native select with no UA chrome, our chevdown caret (components.js selectBox)
  const Select = (options, o) => window.C.selectBox(options, o && o.value, o && o.onChange);
  function Text(text, o) { o = o || {}; const t = el('span', { class: 'ui-text' }, text); t.style.fontSize = 'var(--t-' + (o.step || 'body') + ')'; if (o.weight) t.style.fontWeight = o.weight; if (o.mono) t.style.fontFamily = 'var(--font-mono)'; if (o.dim) t.style.color = 'var(--fg-dim)'; if (o.color) t.style.color = o.color; if (o.caps) { t.style.textTransform = 'uppercase'; t.style.letterSpacing = (o.track || .22) + 'em'; } if (o.align) t.style.textAlign = o.align; return t; }
  function Stack(o, ...kids) { o = o || {}; const s = el('div', { class: 'ui-stack' }, ...kids); s.style.display = 'flex'; s.style.flexDirection = o.dir === 'row' ? 'row' : 'column'; s.style.gap = o.gap != null ? px(o.gap) : 'var(--gap)'; if (o.rowGap != null) s.style.rowGap = px(o.rowGap); if (o.columnGap != null) s.style.columnGap = px(o.columnGap); if (o.align) s.style.alignItems = o.align; if (o.justify) s.style.justifyContent = o.justify; if (o.wrap) s.style.flexWrap = 'wrap'; return s; }
  function Spacer() { const s = el('span', { class: 'ui-spacer' }); s.style.flex = '1'; return s; }
  function Badge(text) { return el('span', { class: 'ui-badge' }, text); }
  function Button(label, o) { o = o || {}; const kind = o.kind || 'primary', st = o.state || 'idle'; const b = el('button', { class: 'ui-btn ' + kind + ' st-' + st, disabled: st === 'disabled' ? '' : null }, o.icon ? window.C.harnessIcon(o.icon) : null, label, o.kbd ? Badge(o.kbd) : null); return b; }
  function List(items, o) { o = o || {}; const l = el('div', { class: 'ui-list', role: 'listbox' }); items.forEach((it, i) => l.append(el('div', { class: 'ui-li' + (i === o.selected ? ' sel' : ''), role: 'option' }, it.icon ? window.C.harnessIcon(it.icon) : null, Text(it.label, { step: 'row' }), it.sub ? Text(it.sub, { step: 'caption', dim: true }) : null, Spacer(), it.kbd ? Badge(it.kbd) : null))); if (o.more) l.append(el('div', { class: 'ui-li more' }, Text('… ' + o.more + ' more, virtualised', { step: 'caption', dim: true }))); return l; }
  function Table(cols, rows, o) { o = o || {}; const t = el('div', { class: 'ui-table' + (o.zebra ? ' zebra' : ''), role: 'table' }); t.style.gridTemplateColumns = cols.map(c => c.w || '1fr').join(' ');
    cols.forEach(c => t.append(el('div', { class: 'ui-th' }, Text(c.label, { step: 'small', caps: true, dim: true })))); rows.forEach((r, i) => r.forEach(v => t.append(el('div', { class: 'ui-td' + (i === o.selected ? ' sel' : '') + (i % 2 ? ' odd' : '') }, typeof v === 'string' ? Text(v, { step: 'row' }) : v)))); return t; }
  function Input(o) { o = o || {}; const st = o.state || 'empty'; const box = el('label', { class: 'ui-input st-' + st }, o.icon ? window.C.harnessIcon(o.icon) : null, el('input', { value: st === 'empty' ? '' : (o.value || ''), placeholder: o.placeholder || '', spellcheck: 'false' })); const wrap = el('div', { class: 'ui-input-wrap' }, box); if (st === 'invalid') wrap.append(Text(o.error || 'not a valid value', { step: 'caption', color: 'var(--c-errored)' })); return wrap; }
  function Overlay(...kids) { return el('div', { class: 'ui-overlay' }, ...kids); }
  function Modal(title, body, actions) { const m = Box('modal', Stack({ gap: S.spacing.s5 }, Text(title, { step: 'subtitle', weight: 650 }), body, Stack({ dir: 'row', gap: S.spacing.s3, align: 'center' }, Spacer(), ...(actions || [])))); m.classList.add('ui-modal'); return m; }

  // parity key (Oracle, decided): <page>/<state>-<density>-<theme>, e.g. materials/gloss-cozy-dark. With ?theme= or ?density= in the
  // URL a gallery renders only that case, so one capture is one key; without them it shows every combination side by side.
  const Q = new URLSearchParams(location.search);
  const themes = () => Q.get('theme') && T.color[Q.get('theme')] ? [Q.get('theme')] : ['dark', 'light'];
  const densities = () => Q.get('density') && T.density[Q.get('density')] ? [Q.get('density')] : ['touch', 'comfortable', 'cozy', 'compact'];
  window.UI = { textOpts, themes, densities, themeScope, densityScope, material, MATERIAL, STYLES, styleCss, STEPS, Box, Path, Select, Text, Stack, Spacer, Badge, Button, List, Table, Input, Overlay, Modal };
})();
