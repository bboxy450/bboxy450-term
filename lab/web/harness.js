// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//
// Rich harness face: folds bboxy450-harness protocol envelopes (src/protocol/mod.rs, Envelope/Event/Item) into UI, never
// into terminal lines. One tool call is one card: its approval, its terminal and its file change fold into it, so
// "approval · ▸ call · ✗ error" is a single row with a badge, not three log lines. Reasoning collapses to "Thought for 1.2s".
// Usage items feed the header meter. The same fold drives the lab page, the showcase pane and, later, the native face.
//   const v = Hx.view({ seat });  v.apply(envelope) …  v.node
//   Hx.play(view, envelopes, { loop })   replays a stored session at reading pace
(function () {
  const el = Lab.el;
  const esc = s => String(s).replace(/[&<>"]/g, c => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]);
  const parse = s => { try { return JSON.parse(s); } catch { return {}; } };
  const ms = d => d < 1000 ? Math.max(1, Math.round(d)) + 'ms' : (d / 1000).toFixed(d < 10000 ? 1 : 0) + 's';
  const kfmt = n => n >= 1e6 ? (n / 1e6).toFixed(1) + 'M' : n >= 1e3 ? (n / 1e3).toFixed(n >= 1e4 ? 0 : 1) + 'k' : String(n);
  const svg = (d, extra) => '<svg viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"' + (extra || '') + '>' + d + '</svg>';
  const ICON = {
    read: svg('<path d="M4 1.75h5.5L12.5 4.75V14.25H4z"/><path d="M9.5 1.75v3h3M6 8h4.5M6 10.5h4.5"/>'),
    search: svg('<circle cx="7" cy="7" r="4.25"/><path d="M10.25 10.25 14 14"/>'),
    execute: svg('<rect x="1.75" y="2.75" width="12.5" height="10.5" rx="2"/><path d="m4.5 6.5 2 1.75-2 1.75M8.5 10.5h3"/>'),
    edit: svg('<path d="M10.5 2.5 13.5 5.5 6 13H3v-3z"/><path d="M9 4l3 3"/>'),
    delete: svg('<path d="M3 4.5h10M6.5 4.5V3h3v1.5M4.5 4.5l.75 9h5.5l.75-9"/>'),
    move: svg('<path d="M2.5 8h11M10 4.5 13.5 8 10 11.5"/>'),
    fetch: svg('<circle cx="8" cy="8" r="6"/><path d="M2 8h12M8 2c2 2 2 10 0 12M8 2c-2 2-2 10 0 12"/>'),
    think: svg('<path d="M8 1.75v2M8 12.25v2M1.75 8h2M12.25 8h2M3.6 3.6l1.4 1.4M11 11l1.4 1.4M3.6 12.4 5 11M11 5l1.4-1.4"/>'),
    other: svg('<path d="M8 2l1.6 4.4L14 8l-4.4 1.6L8 14l-1.6-4.4L2 8l4.4-1.6z"/>'),
    check: svg('<path d="m3.5 8.5 3 3 6-7"/>'),
    x: svg('<path d="m4.5 4.5 7 7M11.5 4.5l-7 7"/>'),
    shield: svg('<path d="M8 1.75 13.25 4v4c0 3-2.25 5.25-5.25 6.25C5 13.25 2.75 11 2.75 8V4z"/><path d="m5.75 8 1.6 1.6 2.9-3.1"/>'),
    shieldx: svg('<path d="M8 1.75 13.25 4v4c0 3-2.25 5.25-5.25 6.25C5 13.25 2.75 11 2.75 8V4z"/><path d="m6.25 6.25 3.5 3.5M9.75 6.25l-3.5 3.5"/>'),
    chev: svg('<path d="m6 4 4 4-4 4"/>'),
    send: svg('<path d="M8 13V3M3.5 7.5 8 3l4.5 4.5"/>'),
    branch: svg('<circle cx="4.5" cy="3.5" r="1.5"/><circle cx="4.5" cy="12.5" r="1.5"/><circle cx="11.5" cy="5.5" r="1.5"/><path d="M4.5 5v6M11.5 7c0 2.5-3 2.5-7 4"/>'),
    plan: svg('<path d="M6 4h7.5M6 8h7.5M6 12h7.5"/><circle cx="3" cy="4" r=".6"/><circle cx="3" cy="8" r=".6"/><circle cx="3" cy="12" r=".6"/>'),
    stop: svg('<rect x="4" y="4" width="8" height="8" rx="1.5" fill="currentColor" stroke="none"/>'),
    clock: svg('<circle cx="8" cy="8" r="6"/><path d="M8 4.75V8l2.25 1.5"/>'),
    retry: svg('<path d="M13 8a5 5 0 1 1-1.5-3.55"/><path d="M13 2.5v3h-3"/>'),
    loop: svg('<path d="M3 7a4.5 4.5 0 0 1 8.2-2.5M13 9a4.5 4.5 0 0 1-8.2 2.5"/><path d="M11.5 2v2.75H8.75M4.5 14v-2.75h2.75"/>'),
    plug: svg('<path d="M6 2v3M10 2v3M4.5 5h7v2.5a3.5 3.5 0 0 1-7 0zM8 11v3"/>'),
  };
  const icon = (name, cls) => el('span', { class: 'hx-i ' + (cls || ''), html: ICON[name] || ICON.other });

  // ---------- text: a small Rust/JS/shell highlighter and Markdown ----------
  const KW = /^(?:fn|let|mut|pub|struct|impl|use|mod|if|else|for|in|return|match|self|Self|super|crate|true|false|const|static|enum|while|loop|as|where|async|await|move|trait|type|unsafe|ref|dyn|function|var|import|export|from|new|class|def|None|null|undefined)$/;
  function highlight(line) {
    let out = '', m; const re = /(\/\/.*$|#!?\[.*?\]|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])'|\b\d[\d_.]*\b|\b[A-Za-z_]\w*!?|[^\w\s"'/#]+|\s+|.)/g;
    while ((m = re.exec(line))) {
      const t = m[0];
      if (t.startsWith('//')) out += '<span class="c">' + esc(t) + '</span>';
      else if (t.startsWith('#[') || t.startsWith('#![')) out += '<span class="a">' + esc(t) + '</span>';
      else if (t[0] === '"' || (t[0] === "'" && t.length > 2)) out += '<span class="s">' + esc(t) + '</span>';
      else if (/^\d/.test(t)) out += '<span class="n">' + esc(t) + '</span>';
      else if (t.endsWith('!')) out += '<span class="m">' + esc(t) + '</span>';
      else if (KW.test(t)) out += '<span class="k">' + esc(t) + '</span>';
      else if (/^[A-Z][A-Za-z0-9]*$/.test(t)) out += '<span class="ty">' + esc(t) + '</span>';
      else out += esc(t);
    }
    return out;
  }
  const inline = s => esc(s)
    .replace(/`([^`]+)`/g, '<code>$1</code>')
    .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
    .replace(/(^|[^*\w])\*([^*\s][^*]*)\*/g, '$1<i>$2</i>')
    .replace(/\[([^\]]+)\]\((https?:[^)\s]+)\)/g, '<a href="$2" target="_blank" rel="noopener">$1</a>')
    .replace(/(→|←)/g, '<span class="arw">$1</span>');
  // figs collects closed ```mermaid fences; the caller swaps each placeholder for a diagram figure
  function markdown(src, figs) {
    const out = []; let list = null, code = null, lang = '', table = null;
    const cells = l => l.trim().replace(/^\||\|$/g, '').split('|').map(c => c.trim());
    const flushTable = () => { if (table) { const [hd, ...rows] = table; out.push('<table class="hx-table"><thead><tr>' + hd.map(c => '<th>' + inline(c) + '</th>').join('') + '</tr></thead><tbody>' + rows.map(r => '<tr>' + r.map(c => '<td>' + inline(c) + '</td>').join('') + '</tr>').join('') + '</tbody></table>'); table = null; } };
    const flush = () => { flushTable(); if (list) { out.push('<' + list.tag + '>' + list.items.map(x => '<li>' + inline(x) + '</li>').join('') + '</' + list.tag + '>'); list = null; } };
    for (const line of src.split('\n')) {
      if (line.startsWith('```')) {
        if (code) {
          if (lang === 'mermaid' && figs) { out.push('<div class="hx-mmd" data-i="' + figs.length + '"></div>'); figs.push(code.join('\n')); }
          else out.push('<pre class="hx-code">' + code.map(highlight).join('\n') + '</pre>');
          code = null;
        } else { flush(); code = []; lang = line.slice(3).trim().split(/\s/)[0]; }
        continue;
      }
      if (code) { code.push(line); continue; }
      // GFM table: a pipe row, then a --- separator row; body rows follow until a non-pipe line
      if (/^\s*\|.*\|\s*$/.test(line)) {
        if (table) { if (!/^[\s|:-]+$/.test(line)) table.push(cells(line)); continue; }
        table = [cells(line)]; continue;
      }
      flushTable();
      const h = line.match(/^(#{1,4}) (.*)/), ul = line.match(/^\s*[-*] (.*)/), ol = line.match(/^\s*\d+[.)] (.*)/);
      if (ul || ol) { const tag = ul ? 'ul' : 'ol'; if (list && list.tag !== tag) flush(); list = list || { tag, items: [] }; list.items.push((ul || ol)[1]); continue; }
      flush();
      if (h) out.push('<h' + (h[1].length + 2) + '>' + inline(h[2]) + '</h' + (h[1].length + 2) + '>');
      else if (line.trim()) out.push('<p>' + inline(line) + '</p>');
    }
    flush(); if (code) out.push('<pre class="hx-code">' + code.map(highlight).join('\n') + '</pre>');
    return out.join('');
  }

  // ---------- diffs: re-derive a minimal line diff from any unified text ----------
  // The harness's edit diff can mark the whole file -/+ (a known core gap); both sides are present, so an LCS over the hunk
  // recovers the real change and folds the unchanged runs.
  function hunks(unified) {
    const files = []; let f = null, h = null;
    for (const l of unified.split('\n')) {
      if (l.startsWith('--- ')) { f = { from: l.slice(4).replace(/^a\//, ''), to: '', hunks: [] }; files.push(f); continue; }
      if (l.startsWith('+++ ')) { if (f) f.to = l.slice(4).replace(/^b\//, ''); continue; }
      const m = l.match(/^@@ -(\d+)(?:,\d+)? \+(\d+)/);
      if (m) { if (!f) { f = { from: '', to: '', hunks: [] }; files.push(f); } h = { a: +m[1], b: +m[2], old: [], neu: [] }; f.hunks.push(h); continue; }
      if (!h || l.startsWith('\\')) continue;
      const c = l[0], t = l.slice(1);
      if (c === ' ' || c === undefined) { h.old.push(t); h.neu.push(t); } else if (c === '-') h.old.push(t); else if (c === '+') h.neu.push(t);
    }
    return files;
  }
  function lcsOps(a, b) {
    const n = a.length, m = b.length;
    if (n * m > 4e6) return [...a.map(t => ['-', t]), ...b.map(t => ['+', t])];
    const dp = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
    for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) dp[i][j] = a[i] === b[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    const ops = []; let i = 0, j = 0;
    while (i < n && j < m) { if (a[i] === b[j]) { ops.push([' ', a[i]]); i++; j++; } else if (dp[i + 1][j] >= dp[i][j + 1]) ops.push(['-', a[i++]]); else ops.push(['+', b[j++]]); }
    while (i < n) ops.push(['-', a[i++]]); while (j < m) ops.push(['+', b[j++]]);
    return ops;
  }
  function diffStat(unified) { let add = 0, del = 0; for (const f of hunks(unified)) for (const h of f.hunks) for (const [o] of lcsOps(h.old, h.neu)) { if (o === '+') add++; else if (o === '-') del++; } return { add, del }; }
  function diffView(unified, ctx) {
    ctx = ctx == null ? 2 : ctx;
    const box = el('div', { class: 'hx-diff' });
    for (const f of hunks(unified)) for (const h of f.hunks) {
      const ops = lcsOps(h.old, h.neu); let a = h.a, b = h.b;
      const rows = ops.map(([o, t]) => { const r = { o, t, a: o === '+' ? '' : a, b: o === '-' ? '' : b }; if (o !== '+') a++; if (o !== '-') b++; return r; });
      const near = rows.map((r, i) => r.o !== ' ' || rows.slice(Math.max(0, i - ctx), i + ctx + 1).some(x => x.o !== ' '));
      let html = '', skipped = 0;
      const fold = () => { if (skipped) html += '<div class="r fold"><span class="g"></span><span class="g"></span><span class="t">⋯ ' + skipped + ' unchanged line' + (skipped === 1 ? '' : 's') + '</span></div>'; skipped = 0; };
      rows.forEach((r, i) => {
        if (!near[i]) { skipped++; return; }
        fold();
        html += '<div class="r ' + (r.o === '+' ? 'add' : r.o === '-' ? 'del' : '') + '"><span class="g">' + r.a + '</span><span class="g">' + r.b + '</span><span class="t"><span class="o">' + (r.o === ' ' ? ' ' : r.o === '-' ? '−' : '+') + '</span>' + (highlight(r.t) || ' ') + '</span></div>';
      });
      fold();
      box.insertAdjacentHTML('beforeend', html);
      pairMarks(box);
    }
    return box;
  }
  // a removed line followed by its added twin: mark only the span that changed (common prefix/suffix stay plain)
  function pairMarks(box) {
    for (const del of box.querySelectorAll('.r.del')) {
      const add = del.nextElementSibling; if (!add || !add.classList.contains('add') || del.previousElementSibling?.classList.contains('del') || add.nextElementSibling?.classList.contains('add')) continue;
      const [a, b] = [del, add].map(r => r.querySelector('.t').textContent.slice(1));
      let p = 0; while (p < a.length && a[p] === b[p]) p++;
      let q = 0; while (q < a.length - p && q < b.length - p && a[a.length - 1 - q] === b[b.length - 1 - q]) q++;
      if (p + q < 4) continue;
      for (const [r, t] of [[del, a], [add, b]]) r.querySelector('.t').innerHTML = '<span class="o">' + (r === del ? '−' : '+') + '</span>' + highlight(t.slice(0, p)) + '<span class="chg">' + highlight(t.slice(p, t.length - q)) + '</span>' + highlight(t.slice(t.length - q));
    }
  }
  function codeView(lines, start) {
    const box = el('div', { class: 'hx-src' });
    box.innerHTML = lines.map((t, i) => '<div class="r"><span class="g">' + (start + i) + '</span><span class="t">' + (highlight(t) || ' ') + '</span></div>').join('');
    return box;
  }
  // test/compiler output: colour the verdict lines the way a person scans them
  function termOut(text) {
    const box = el('div', { class: 'hx-out' });
    box.innerHTML = text.replace(/^\n+|\n+$/g, '').split('\n').map(l => {
      const c = /\.\.\. ok$|^test result: ok|^\s*Finished|^ok\b/.test(l) ? 'ok' : /FAILED|panicked|^error|^\s*error\[|failures:/.test(l) ? 'bad' : /^warning|^note:/.test(l) ? 'warn' : /^running \d+|^\s*Compiling/.test(l) ? 'dim' : '';
      return '<div class="' + c + '">' + (esc(l).replace(/\b(ok)$/, '<b>$1</b>').replace(/\b(FAILED)\b/g, '<b>$1</b>') || ' ') + '</div>';
    }).join('');
    return box;
  }

  // ---------- profiles: the machine-global config, ~/.bboxy450-harness/profiles/<name>.md + default-profile ----------
  // A profile record is what a face may see: never a key, only where the key comes from and whether it resolved.
  // { name, provider, model, endpoint, auth: { kind: keychain|env|command|login|none, ref, ok }, context_tokens, default, note }
  const PROVIDERS = {
    openrouter: { label: 'OpenRouter', mark: 'OR', color: '#7C6CFF', endpoint: 'https://openrouter.ai/api/v1', auth: { kind: 'env', ref: 'OPENROUTER_API_KEY' }, model: 'anthropic/claude-sonnet-5-5' },
    anthropic: { label: 'Anthropic API', mark: 'A', color: '#D97757', endpoint: 'https://api.anthropic.com/v1', auth: { kind: 'keychain', ref: 'bboxy450-harness-anthropic' }, model: 'claude-opus-5-5' },
    'anthropic-subscription': { label: 'Anthropic subscription', mark: 'A', color: '#D97757', endpoint: '', auth: { kind: 'login', ref: 'Claude Code login' }, model: 'claude-opus-5-5',
      unsupported: 'Claude Pro/Max sign-in belongs to Anthropic’s own apps. The harness speaks API keys; a subscription profile needs Claude Code as its backend, which is not built yet.' },
    openai: { label: 'OpenAI', mark: 'O', color: '#10A37F', endpoint: 'https://api.openai.com/v1', auth: { kind: 'env', ref: 'OPENAI_API_KEY' }, model: 'gpt-5.1' },
    local: { label: 'Local', mark: 'L', color: '#9AA3C7', endpoint: 'http://localhost:11434/v1', auth: { kind: 'none' }, model: 'qwen3-coder' },
  };
  // An ACP-agent profile (kind acp-agent, provider acp-agent) runs another agent behind the harness over ACP; its tile
  // is the agent's, picked by the first word of agent.command. The agent authenticates itself: the harness holds no key.
  PROVIDERS['acp-agent'] = { label: 'ACP agent', mark: '⌁', color: '#2EE6A8', endpoint: '', auth: { kind: 'none' }, model: '', agent: 'claude-agent-acp' };
  const AGENTS = {
    'claude-agent-acp': { label: 'Claude Agent', mark: 'C', color: '#D97757' },
    'codex-acp': { label: 'Codex', mark: 'Cx', color: '#10A37F' },
    opencode: { label: 'opencode', mark: 'oc', color: '#5B6178' },
    'pi-acp': { label: 'pi', mark: 'π', color: '#FF4FA3' },
    omp: { label: 'oh-my-pi', mark: 'ω', color: '#B07BFF' },
    goose: { label: 'Goose', mark: 'G', color: '#C98A1B' },
  };
  const AUTH_SAY = { keychain: 'Keychain', env: 'env', command: 'command', login: 'login', none: 'no key' };
  const isAgent = p => p.kind === 'acp-agent' || p.provider === 'acp-agent';
  const agentOf = cmd => { const c = (Array.isArray(cmd) ? cmd[0] : String(cmd || '').split(/\s/)[0]) || ''; return AGENTS[c] || { label: c || 'agent', mark: (c[0] || '·').toUpperCase(), color: '#2EE6A8' }; };
  const provOf = p => isAgent(p) ? agentOf(p.agent_command) : PROVIDERS[p.provider] || { label: p.provider || 'custom', mark: '?', color: '#9AA3C7' };
  const provMark = (p, cls) => { const P = provOf(p); return el('span', { class: 'hx-pm' + (isAgent(p) ? ' agent' : '') + (cls ? ' ' + cls : ''), style: '--pc:' + P.color }, P.mark); };
  // "acp:claude-agent-acp · claude-haiku-5-5" (profile_changed.model of an ACP session) → { agent, model }
  const acpModel = m => { const x = /^acp:([^·]+?)\s*(?:·\s*(.*))?$/.exec(m || ''); return x ? { agent: x[1].trim(), model: x[2] || '' } : null; };
  // the frontmatter a new profile writes; the key itself never enters the file
  function profileYaml(f) {
    const a = f.auth || {};
    if (f.provider === 'acp-agent') return ['---', 'name: ' + f.name, 'provider: acp-agent', 'agent:', '  command: [' + (f.agent || 'claude-agent-acp').split(/\s+/).join(', ') + ']', f.model ? 'model: ' + f.model : null, 'on_unattended: deny', '---'].filter(Boolean).join('\n');
    return ['---', 'name: ' + f.name, 'model: ' + f.model, 'endpoint: ' + (f.endpoint || '""'), 'provider: ' + f.provider,
      a.kind === 'keychain' ? 'api_key: "!security find-generic-password -s ' + a.ref + ' -w"' : a.kind === 'command' ? 'api_key: "!' + a.ref + '"' : null,
      a.kind === 'env' ? 'api_key_env: ' + a.ref : null, 'on_unattended: deny', 'auto_compact: true', '---'].filter(Boolean).join('\n');
  }
  function profileMenu(st, opts, close) {
    const list = opts.profiles || [], cur = st.profile;
    const box = el('div', { class: 'hx-pmenu', role: 'menu', onclick: e => e.stopPropagation() });
    let q = '';
    const search = el('input', { class: 'hx-pq', placeholder: 'Switch profile…', oninput: e => { q = e.target.value.toLowerCase(); draw(); }, onkeydown: e => { if (e.key === 'Escape') close(); if (e.key === 'Enter') { const r = rowsEl.querySelector('.hx-prow:not(.off)'); if (r) r.click(); } } });
    const rowsEl = el('div', { class: 'hx-prows' });
    function row(p) {
      const agent = isAgent(p), P = provOf(p), off = !!P.unsupported || (p.auth && p.auth.ok === false), on = p.name === cur;
      const a = p.auth || {}, cmd = (p.agent_command || []).join(' ');
      // model rows: model id + provider; agent rows: the command the harness spawns + the model it asks the agent for
      const sub = agent ? el('small', {}, el('span', { class: 'p' }, '$ '), cmd || 'agent', p.model ? ' · ' + p.model : '') : el('small', {}, p.model, el('span', { class: 'pv' }, ' · ' + P.label));
      const auth = agent ? el('span', { class: 'hx-auth na', title: (p.agent_env_names || []).length ? 'agent env: ' + p.agent_env_names.join(', ') : 'the agent signs itself in' }, (p.agent_env_names || []).length ? 'env ' + p.agent_env_names.length : 'own login')
        : el('span', { class: 'hx-auth ' + (P.unsupported ? 'na' : a.ok === false ? 'bad' : 'ok') }, P.unsupported ? 'not supported' : (a.ok === false ? 'missing ' : '') + AUTH_SAY[a.kind || 'none']);
      return el('button', { type: 'button', class: 'hx-prow' + (on ? ' on' : '') + (off ? ' off' : ''), title: P.unsupported || (a.ok === false ? AUTH_SAY[a.kind] + ' ' + (a.ref || '') + ' not found' : agent ? P.label + ' over ACP' : p.endpoint || ''),
        onclick: () => { if (off || on) return; close(); opts.onOp && opts.onOp({ type: 'set_profile', profile: p.name }, p); } },
        provMark(p),
        el('span', { class: 'hx-pn' }, el('b', {}, p.name, p.default ? el('span', { class: 'hx-def', title: 'default-profile' }, 'default') : null), sub),
        el('span', { class: 'hx-sp' }),
        p.context_tokens ? el('span', { class: 'hx-ctx' }, kfmt(p.context_tokens)) : null,
        auth,
        el('span', { class: 'hx-pck' }, on ? icon('check') : null));
    }
    function draw() {
      rowsEl.replaceChildren();
      const hit = p => !q || (p.name + ' ' + p.model + ' ' + provOf(p).label + ' ' + (p.agent_command || []).join(' ')).toLowerCase().includes(q);
      if (cur && !list.some(p => p.name === cur)) rowsEl.append(el('div', { class: 'hx-prow on off' }, el('span', { class: 'hx-pm', style: '--pc:#9AA3C7' }, '·'), el('span', { class: 'hx-pn' }, el('b', {}, cur), el('small', {}, (st.model || '') + ' · not in profiles/')), el('span', { class: 'hx-sp' }), el('span', { class: 'hx-pck' }, icon('check'))));
      // two kinds, agents first: an ACP agent is a whole other harness, a model is a brain for this one
      const agents = list.filter(p => isAgent(p) && hit(p)), models = list.filter(p => !isAgent(p) && hit(p));
      const order = Object.keys(PROVIDERS), rank = p => { const i = order.indexOf(p.provider); return i < 0 ? order.length : i; };
      models.sort((x, y) => rank(x) - rank(y));
      if (agents.length) rowsEl.append(el('div', { class: 'hx-pgrp' }, icon('plug'), 'ACP agents', el('span', { class: 'n' }, agents.length)), ...agents.map(row));
      if (models.length) rowsEl.append(el('div', { class: 'hx-pgrp' }, icon('think'), 'Models', el('span', { class: 'n' }, models.length)), ...models.map(row));
      if (!agents.length && !models.length) rowsEl.append(el('div', { class: 'hx-pnone' }, 'No profile matches “' + q + '”'));
    }
    draw();
    box.append(search, rowsEl, el('div', { class: 'hx-pfoot' },
      el('button', { type: 'button', class: 'hx-padd', onclick: () => { box.replaceWith(newProfile(opts, close)); } }, '+ New profile'),
      el('span', { class: 'hx-sp' }), el('code', { title: 'machine-global: every seat and face reads it' }, '~/.bboxy450-harness/profiles')));
    requestAnimationFrame(() => search.focus());
    return box;
  }
  function newProfile(opts, close) {
    const f = { provider: 'openrouter', name: '', model: '', endpoint: '', agent: '', auth: null };
    const box = el('div', { class: 'hx-pmenu hx-pnew', onclick: e => e.stopPropagation() });
    const pre = el('pre', { class: 'hx-yaml' });
    const tabs = el('div', { class: 'hx-ptabs' });
    const field = (label, key, ph) => el('label', { class: 'hx-pf' }, el('span', {}, label), el('input', { value: f[key], placeholder: ph || '', oninput: e => { f[key] = e.target.value; paint(); } }));
    const fields = el('div', { class: 'hx-pfs' });
    function paint() {
      const P = PROVIDERS[f.provider];
      const ff = { ...f, name: f.name || f.provider, model: f.model || P.model, endpoint: f.endpoint || P.endpoint, auth: P.auth, agent: f.agent || P.agent };
      if (f.provider === 'acp-agent') { ff.kind = 'acp-agent'; ff.agent_command = ff.agent.split(/\s+/); ff.name = f.name || ff.agent_command[0].replace(/-acp$/, '') + '-acp'; }
      tabs.replaceChildren(...Object.entries(PROVIDERS).map(([k, X]) => el('button', { type: 'button', class: 'hx-pt' + (k === f.provider ? ' on' : ''), style: '--pc:' + X.color, onclick: () => { f.provider = k; f.endpoint = ''; f.model = ''; f.agent = ''; build(); } }, X.label)));
      pre.textContent = P.unsupported ? P.unsupported : '# ~/.bboxy450-harness/profiles/' + ff.name + '.md\n' + profileYaml(ff);
      pre.classList.toggle('na', !!P.unsupported);
      save.disabled = !!P.unsupported;
      return ff;
    }
    const save = el('button', { type: 'button', class: 'pri', onclick: () => { const ff = paint(); close(); opts.onOp && opts.onOp({ type: 'create_profile', profile: ff }, ff); } }, 'Save & switch');
    function build() {
      const P = PROVIDERS[f.provider];
      const acp = f.provider === 'acp-agent';
      fields.replaceChildren(field('Name', 'name', acp ? 'claude-acp' : f.provider), acp ? field('Agent', 'agent', P.agent) : null, field('Model', 'model', acp ? 'the agent\u2019s own default' : P.model), P.unsupported || acp ? null : field('Endpoint', 'endpoint', P.endpoint),
        P.auth.kind === 'none' || P.unsupported ? null : el('div', { class: 'hx-pf' }, el('span', {}, 'Key'), el('span', { class: 'hx-keysrc' }, icon('shield'), AUTH_SAY[P.auth.kind] + ' ', el('code', {}, P.auth.ref), el('small', {}, ' · read when the profile loads, never written here'))));
      paint();
    }
    build();
    box.append(el('div', { class: 'hx-pnh' }, el('b', {}, 'New profile'), el('span', { class: 'hx-sp' }), el('button', { type: 'button', class: 'hx-px', onclick: close }, '×')), tabs, fields, pre, el('div', { class: 'hx-pact' }, el('span', { class: 'hx-sp' }), el('button', { type: 'button', onclick: close }, 'Cancel'), save));
    return box;
  }

  // ---------- the fold ----------
  function fresh() { return { meta: null, model: '', profile: '', items: new Map(), blocks: [], usage: { in: 0, out: 0, cached: 0, reasoning: 0, cost: 0 }, turns: new Map(), closed: false, phase: null }; }
  function view(opts) {
    opts = opts || {};
    let s = fresh();
    const nodes = new Map();
    const head = el('div', { class: 'hx-head' }), feed = el('div', { class: 'hx-feed' }), scroller = el('div', { class: 'hx-scroll' }, feed);
    const comp = composer(opts, { profiles: () => opts.profiles || [], openProfiles: () => { if (!menu) toggleMenu(); } });
    const node = el('div', { class: 'hx' + (opts.compact ? ' compact' : '') }, head, scroller, comp);
    let stick = true;
    scroller.addEventListener('scroll', () => { stick = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 40; });
    // follow the tail with a critically damped glide (~180ms), never a jump; a new target retargets mid-glide
    let glide = 0, vel = 0;
    const follow = () => {
      if (!stick || glide) return;
      if (matchMedia('(prefers-reduced-motion: reduce)').matches) { scroller.scrollTop = scroller.scrollHeight; return; }
      let t0 = performance.now();
      const step = now => {
        const dt = Math.min(0.05, (now - t0) / 1000), w = 2 * Math.PI / 0.18; t0 = now;
        const gap = scroller.scrollHeight - scroller.clientHeight - scroller.scrollTop;
        vel += (w * w * gap - 2 * w * vel) * dt; scroller.scrollTop += vel * dt;
        if (!stick || Math.abs(gap) < 0.5 && Math.abs(vel) < 4) { if (stick) scroller.scrollTop = scroller.scrollHeight; glide = 0; vel = 0; return; }
        glide = requestAnimationFrame(step);
      };
      glide = requestAnimationFrame(step);
    };
    // the empty session: the mark, one question, three ways in; it leaves on the first block
    function empty() {
      const was = feed.querySelector(':scope > .hx-empty');
      if (s.blocks.length) { if (was) was.remove(); return; }
      const acp = acpModel(s.model), who = acp ? (s.profile || acp.agent) + (acp.model ? ' · ' + acp.model : '') : s.model || s.profile;
      const key = who + '|' + cwd();
      if (was && was.dataset.key === key) return;
      if (was) { was.querySelector('.hx-es')?.remove(); if (who || cwd()) was.querySelector('.hx-et').after(el('div', { class: 'hx-es' }, who || '', who && cwd() ? ' in ' : '', cwd() ? '~/' + cwd().split('/').pop() : '')); was.dataset.key = key; return; }
      const chip = (label, o) => el('button', { type: 'button', class: 'hx-echip', onclick: e => { e.stopPropagation(); comp.hx.set(Object.assign({ focus: true }, o)); } }, label);
      feed.append(el('div', { class: 'hx-empty', 'data-key': key },
        el('span', { class: 'hx-emark' }, el('img', { src: 'bboxy.png', alt: '' })),
        el('div', { class: 'hx-et' }, 'What are we building?'),
        who || cwd() ? el('div', { class: 'hx-es' }, who || '', who && cwd() ? ' in ' : '', cwd() ? '~/' + cwd().split('/').pop() : '') : null,
        el('div', { class: 'hx-echips' }, chip('/ commands', { text: '/', menu: 'slash', sel: 0 }), chip('@ a file', { text: '@' }), chip('Describe a change', { text: '' }))));
    }

    function item(id) { return s.items.get(id); }
    function toolOf(it) { return it && it.parent ? item(it.parent) : null; }
    function apply(env) {
      const e = env.event, ts = env.ts_ms != null ? env.ts_ms : Date.now();
      switch (e.type) {
        case 'session_started': s.meta = e.meta; s.profile = e.meta.profile || ''; break;
        case 'profile_changed': {
          const was = s.profile; s.profile = e.profile; s.model = e.model;
          if (was && was !== e.profile && s.blocks.length) { const k = 'sw:' + s.blocks.length; s.switches = s.switches || {}; s.switches[k] = { from: was, to: e.profile, model: e.model }; s.blocks.push({ id: k }); mount(k); }
          break;
        }
        case 'turn_started': s.turns.set(e.turn, { t0: ts, rounds: 0, tools: 0, edits: 0 }); s.turn = e.turn; break;
        case 'item_started': {
          const it = { id: e.item, kind: e.kind, turn: e.turn, parent: e.parent, head: e.head, status: 'running', body: null, text: '', live: '', t0: ts, t1: null };
          s.items.set(it.id, it);
          const t = s.turns.get(e.turn);
          if (t && e.kind === 'reasoning') t.rounds++;
          if (t && e.kind === 'tool_call') t.tools++;
          const parent = toolOf(it);
          if (parent && (e.kind === 'terminal' || e.kind === 'file_change')) { parent[e.kind === 'terminal' ? 'terminal' : 'change'] = it; touch(parent.id); break; }
          if (e.kind === 'approval' || e.kind === 'usage') break;   // folded into a tool card / the meter at completion
          s.blocks.push({ id: it.id }); mount(it.id);
          break;
        }
        case 'item_delta': {
          const it = item(e.item); if (!it) break;
          const d = e.delta;
          if (d.type === 'text' || d.type === 'reasoning') it.text += d.append;
          else if (d.type === 'output') it.live += d.append;
          else if (d.type === 'tool_input') it.input = (it.input || '') + d.append;
          touch(it.parent && (it.kind === 'terminal' || it.kind === 'file_change') ? it.parent : it.id);
          break;
        }
        case 'item_snapshot': { const it = item(e.item); if (it) { it.body = e.partial; touch(it.id); } break; }
        case 'item_completed': {
          const it = item(e.item) || { id: e.item, kind: e.body.kind, t0: ts };
          s.items.set(it.id, it);
          it.status = e.status.status; it.reason = e.status.reason; it.body = e.body; it.t1 = ts;
          const t = s.turns.get(it.turn);
          if (e.body.kind === 'usage') { const u = e.body.usage; s.usage.in += u.input_tokens; s.usage.out += u.output_tokens; s.usage.cached += u.cached_input_tokens || 0; s.usage.reasoning += u.reasoning_tokens || 0; s.usage.cost += u.cost_micros_usd || 0; break; }
          if (e.body.kind === 'approval') {
            const tool = item(e.body.subject.tool_call);
            if (tool) { tool.approval = it; touch(tool.id); } else { s.blocks.push({ id: it.id }); mount(it.id); }
            break;
          }
          if (e.body.kind === 'file_change' && t) t.edits++;
          if (e.body.kind === 'agent_message' && t) {
            const k = lineKey(e.body.markdown); t.said = t.said || new Set();
            if (k && !t.loop && t.said.has(k)) { startLoop(t, it); touch(it.id); drawLoop(t.loop); break; }
            if (t.loop && t.loop.items.includes(it.id)) { touch(it.id); drawLoop(t.loop); break; }
            t.said.add(k);
          }
          const parent = toolOf(it);
          if (parent && (it.kind === 'terminal' || it.kind === 'file_change')) touch(parent.id); else touch(it.id);
          break;
        }
        case 'turn_ended': {
          const t = s.turns.get(e.turn) || {}; t.t1 = ts; t.reason = e.reason; t.usage = e.usage;
          s.blocks.push({ end: e.turn }); mount('end:' + e.turn); s.turn = null;
          break;
        }
        case 'status': s.phase = e.phase; break;
        case 'session_closed': s.closed = true; break;
      }
      drawHead();
      follow();
    }

    // ---------- blocks ----------
    function mount(id) {
      const it0 = item(id), lt = it0 && s.turns.get(it0.turn), L = lt && lt.loop;
      if (L && /^(agent_message|reasoning|tool_call|notice)$/.test(it0.kind)) { const n = build(id); if (!n) return; nodes.set(id, n); L.items.push(id); L.body.append(n); drawLoop(L); return; }
      const n = build(id); if (!n) return;
      n.classList.add('hx-in'); nodes.set(id, n);
      // consecutive tool calls started in one round sit on a shared rail: they ran together
      const last = feed.lastElementChild;
      if (n.classList.contains('hx-tool') && last && (last.classList.contains('hx-tool') || last.classList.contains('hx-group'))) {
        const grp = last.classList.contains('hx-group') ? last : (() => { const g = el('div', { class: 'hx-group' }); last.replaceWith(g); g.append(last); return g; })();
        grp.append(n); grp.dataset.n = grp.children.length;
      } else feed.append(n);
    }
    function touch(id) { const old = nodes.get(id); if (!old) return; const n = build(id); if (!n) return; nodes.set(id, n); old.replaceWith(n); }
    function build(id) {
      if (id.startsWith('end:')) return turnEnd(id.slice(4));
      if (id.startsWith('sw:')) { const w = s.switches[id], p = (opts.profiles || []).find(x => x.name === w.to) || { provider: '' }; return el('div', { class: 'hx-switch' }, el('i'), provMark(p), el('span', {}, 'Switched to ', el('b', {}, w.to), ' · ', w.model), el('i')); }
      const it = item(id); if (!it) return null;
      switch (it.kind) {
        case 'user_message': return userMsg(it);
        case 'agent_message': return agentMsg(it);
        case 'reasoning': return thought(it);
        case 'tool_call': return tool(it);
        case 'terminal': return standaloneTerm(it);
        case 'file_change': return el('div', { class: 'hx-tool done open' }, el('div', { class: 'hx-th' }, el('span', { class: 'hx-ki edit' }, icon('edit')), el('span', { class: 'hx-verb' }, 'Changed'), el('span', { class: 'hx-tgt' }, short(it.body && it.body.path))), it.body ? diffView(it.body.unified) : null);
        case 'approval': return approvalCard(it);
        case 'notice': return el('div', { class: 'hx-notice ' + ((it.body && it.body.level) || 'info') }, el('i'), it.body ? it.body.text : '');
        case 'plan': return planCard(it);
        default: return null;
      }
    }
    const cwd = () => (s.meta && s.meta.cwd) || '';
    const short = p => { if (!p) return ''; const c = cwd(); return c && p.startsWith(c + '/') ? p.slice(c.length + 1) : p.replace(/^\/(?:Users|home)\/[^/]+/, '~'); };

    function userMsg(it) {
      const text = it.body ? it.body.blocks.filter(b => b.type === 'text').map(b => b.text).join('\n') : '';
      return el('div', { class: 'hx-user' }, el('div', { class: 'hx-who' }, 'you'), el('div', { class: 'hx-ubody', html: inline(text) }));
    }
    // every prose body (agent text, reasoning, markdown tool output) goes through the one renderer, as bbt_doc does natively
    function mdNode(md, cls) {
      const figs = [], n = el('div', { class: cls, html: markdown(md, figs) });
      // Mermaid draws natively in the lab's figure card (Drawing / Source), as bbt_doc does natively
      for (const p of n.querySelectorAll('.hx-mmd')) {
        const src = figs[+p.dataset.i];
        p.replaceWith(window.C && C.diagramFigure ? C.diagramFigure(src) : el('pre', { class: 'hx-code', html: src.split('\n').map(highlight).join('\n') }));
      }
      return n;
    }
    // tool output is markdown when it comes from a .md file or carries markdown structure (a fence, or two headings/lists/table rows)
    const MD_LINE = /^(#{1,6} |\s*[-*+] |\s*\d+\. |\|.*\|\s*$|> )/;
    const looksMd = t => /^```/m.test(t) || t.split('\n').filter(l => MD_LINE.test(l)).length >= 2;
    function agentMsg(it) {
      const md = it.body ? it.body.markdown : it.text, live = !it.body;
      const n = mdNode(md, 'hx-msg' + (live ? ' live' : ''));
      if (live) (n.lastElementChild || n).append(el('span', { class: 'hx-caret' }));
      return n;
    }
    function thought(it) {
      const text = (it.body ? it.body.summary || it.body.raw || '' : it.text).trim(), live = !it.body;
      const n = el('div', { class: 'hx-think' + (live ? ' live' : '') + (it.open ? ' open' : '') });
      const took = (it.t1 || 0) - (it.t0 || 0);
      const label = live ? el('span', { class: 'hx-shimmer' }, 'Thinking') : took >= 1000 ? el('span', {}, 'Thought for ', el('b', {}, ms(took))) : null;
      const gist = text.replace(/\*\*|__|`/g, '').split(/(?<=[.!?])\s|\n/)[0];
      n.append(el('button', { class: 'hx-tk', type: 'button', onclick: () => { it.open = !it.open; touch(it.id); } }, icon('think', 'spin-slow'), label, live ? null : el('span', { class: 'hx-gist' }, gist), live ? null : icon('chev', 'chev')));
      if (live) n.append(el('div', { class: 'hx-tbody' }, text, el('span', { class: 'hx-caret' })));
      else if (it.open) n.append(mdNode(text, 'hx-tbody hx-msg'));
      return n;
    }

    function describe(it) {
      const b = it.body || {}, h = it.head || {}, name = b.name || h.name || '?', kind = b.tool_kind || h.tool_kind || 'other';
      const a = parse(b.input_raw || it.input || '{}');
      // ACP agents fence their output (```console … ```) or leave a placeholder for output only their own terminal saw
      const outText = (b.output || []).filter(o => o.type === 'text' || o.type === 'error').map(o => o.text).join('\n')
        .replace(/^```[\w-]*\n([\s\S]*?)\n?```$/, '$1').replace(/^\[terminal [^\]]+\]$/, '').replace(/^\(no output\)$/, '');
      const mcp = name.match(/^mcp__(.+?)__(.+)$/);
      if (mcp) return { kind: 'other', verb: mcp[2].replace(/_/g, ' '), target: mcp[1], a, outText, chip: 'mcp' };
      switch (name) {
        case 'read': { const lines = outText ? outText.split('\n').filter(Boolean).length : 0; return { kind, verb: 'Read', target: short(a.path), meta: lines ? lines + ' lines' : '', a, outText }; }
        case 'grep': case 'glob': { const hits = outText ? outText.split('\n').filter(Boolean).length : 0; return { kind, verb: name === 'grep' ? 'Search' : 'Find', target: a.pattern, where: a.path && short(a.path), meta: it.body ? hits + (hits === 1 ? ' hit' : ' hits') : '', a, outText, mono: true }; }
        case 'bash': return { kind: 'execute', verb: 'Run', target: a.command, a, outText, mono: true };
        case 'exec': return { kind: 'execute', verb: 'Run', target: [a.program, ...(a.args || [])].join(' '), a, outText, mono: true };
        case 'edit': case 'write': case 'apply_patch': return { kind: 'edit', verb: name === 'write' ? 'Write' : 'Edit', target: short(a.path || (a.edits && a.edits[0] && a.edits[0].path)), a, outText };
        case 'Terminal': return { kind: 'execute', verb: 'Run', target: a.command || '', a, outText, mono: true, note: a.description };
        default:
          // an agent tool named by its command line ("ls -la") is a Run of that line; a phrase ("List files") is the verb itself
          if (kind === 'execute' && !a.command && /^[\w./-]+(\s+[-\w./]+)+$/.test(name)) return { kind, verb: 'Run', target: name, a, outText, mono: true, quiet: !outText };
          return { kind, verb: name, target: a.command || a.path || '', a, outText, mono: !!a.command };
      }
    }
    function approvalBadge(ap) {
      if (!ap || !ap.body) return null;
      const d = ap.body.decision, by = ap.body.by || {};
      const who = by.by === 'rule' ? 'rule ' + by.id : by.by === 'face' ? by.name : by.by === 'unattended' ? 'unattended' : by.by || '';
      if (!d) return el('span', { class: 'hx-ap ask' }, icon('shield'), 'needs approval');
      if (d.decision === 'deny') return el('span', { class: 'hx-ap deny', title: (d.reason || 'denied') + (who ? ' · ' + who : '') }, icon('shieldx'), by.by === 'unattended' ? 'denied · no approver' : 'denied');
      if (by.by === 'rule') return null;   // tight: a rule allowing a call is the normal case, not news
      return el('span', { class: 'hx-ap ok', title: 'allowed by ' + who }, icon('shield'), d.decision === 'allow_always' ? 'always' : 'allowed');
    }
    function tool(it) {
      const D = describe(it), b = it.body, term = it.terminal, ch = it.change;
      const denied = it.approval && it.approval.body && it.approval.body.decision && it.approval.body.decision.decision === 'deny';
      const asking = it.approval && !(it.approval.body && it.approval.body.decision);
      const st = it.status === 'running' ? (asking ? 'ask' : 'run') : denied ? 'deny' : it.status === 'completed' ? 'done' : it.status === 'interrupted' || it.status === 'cancelled' ? 'stop' : 'fail';
      const exit = term && term.body ? term.body.exit : null;
      let meta = D.meta || '';
      if (D.kind === 'execute' && exit) meta = 'exit ' + exit;   // tight: success says nothing, the check mark already did
      const stat = ch && ch.body ? diffStat(ch.body.unified) : D.kind === 'edit' && D.outText.startsWith('---') ? diffStat(D.outText) : null;
      const outLines = ((term && term.live) || D.outText || '').split('\n').filter(Boolean).length;
      const autoOpen = st === 'ask' || st === 'fail' || st === 'run' || (D.kind === 'execute' && outLines <= 6) || D.kind === 'edit' || (D.verb === 'Search' && D.outText.split('\n').filter(Boolean).length <= 6);
      const open = it.open == null ? autoOpen : it.open;
      const n = el('div', { class: 'hx-tool ' + st + (open ? ' open' : ''), 'data-kind': D.kind });
      const th = el('button', { class: 'hx-th', type: 'button', onclick: () => { it.open = !open; touch(it.id); } },
        el('span', { class: 'hx-ki ' + D.kind }, icon(D.kind)),
        el('span', { class: 'hx-verb' }, D.verb),
        el('span', { class: 'hx-tgt' + (D.mono ? ' mono' : '') }, D.target || ''),
        D.where ? el('span', { class: 'hx-where' }, 'in ', D.where) : null,
        D.chip ? el('span', { class: 'hx-chip' }, D.chip) : null,
        el('span', { class: 'hx-sp' }),
        stat ? el('span', { class: 'hx-stat' }, el('b', { class: 'add' }, '+' + stat.add), el('b', { class: 'del' }, '−' + stat.del)) : null,
        meta ? el('span', { class: 'hx-meta' + (exit ? ' bad' : exit === 0 ? ' good' : '') }, meta) : null,
        approvalBadge(it.approval),
        el('span', { class: 'hx-dur' }, it.t1 && it.t1 - it.t0 >= 1000 ? ms(it.t1 - it.t0) : ''),
        el('span', { class: 'hx-st' }, st === 'run' ? el('i', { class: 'hx-dots' }, el('i'), el('i'), el('i')) : st === 'ask' ? icon('shield') : icon(st === 'done' ? 'check' : st === 'stop' ? 'stop' : 'x')));
      n.append(th);
      // a rebuild must not replay entrance motion: the status pops once per status, the body opens once
      if (it._popped === st) th.querySelector('.hx-st').classList.add('noanim'); it._popped = st;
      if (!open) { it._opened = false; return n; }
      const body = el('div', { class: 'hx-tb' });
      if (st === 'ask') { const ap = it.approval.body || {}, sub = ap.subject || {};
        // what exactly is being allowed, verbatim: the command or the call, never only its effect class
        if (sub.command || sub.summary) body.append(el('div', { class: 'hx-asubj' }, el('span', { class: 'p' }, sub.command ? '$' : '›'), sub.command || sub.summary, (sub.paths || []).length ? el('span', { class: 'hx-where' }, ' · ' + sub.paths.map(short).join(', ')) : null));
        body.append(el('div', { class: 'hx-askbar' }, el('span', {}, 'Allow ', el('b', {}, (ap.subject && ap.subject.effect) || 'this'), '?'), el('span', { class: 'hx-sp' }), ...((ap.options || ['allow', 'allow_always', 'deny']).map((o, i) => el('button', { type: 'button', class: i === 0 ? 'pri' : o === 'deny' ? 'no' : '' }, { allow: 'Allow', allow_always: 'Always', deny: 'Deny' }[o] || o, el('span', { class: 'k' }, String(i + 1))))))); }
      else if (st === 'deny') body.append(el('div', { class: 'hx-err' }, (it.approval.body.decision.reason || it.reason || 'denied')));
      else if (D.kind === 'execute') {
        const out = term && term.live ? term.live : D.outText;
        body.append(el('div', { class: 'hx-term' }, out ? termOut(out) : st === 'run' ? el('div', { class: 'hx-out dim' }, el('span', { class: 'hx-caret' })) : el('div', { class: 'hx-out dim' }, D.quiet ? 'output stayed in the agent\u2019s own terminal' : 'no output')));
      } else if (D.kind === 'edit') {
        const u = ch && ch.body ? ch.body.unified : D.outText.startsWith('---') ? D.outText : '';
        if (u) body.append(diffView(u)); else if (D.outText) body.append(el('div', { class: it.status === 'failed' ? 'hx-err' : 'hx-out' }, D.outText));
      } else if (D.verb === 'Read' && D.outText && /\.(md|markdown|mdx)$/i.test(D.a.path || D.target || '')) {
        body.append(mdNode(D.outText.split('\n').map(l => l.replace(/^\d+: ?/, '')).join('\n'), 'hx-msg hx-mdout'));
      } else if (D.verb === 'Read' && D.outText) {
        const rows = D.outText.split('\n').filter(Boolean).map(l => l.replace(/^\d+: ?/, ''));
        const all = it.all, cut = all ? rows : rows.slice(0, 14);
        body.append(codeView(cut, +(D.outText.match(/^(\d+):/) || [0, 1])[1]));
        if (!all && rows.length > cut.length) body.append(el('button', { class: 'hx-more', type: 'button', onclick: () => { it.all = true; touch(it.id); } }, 'Show all ' + rows.length + ' lines'));
      } else if ((D.verb === 'Search' || D.verb === 'Find') && D.outText) {
        const re = D.verb === 'Search' ? (() => { try { return new RegExp('(' + D.a.pattern + ')', 'g'); } catch { return null; } })() : null;
        body.append(el('div', { class: 'hx-hits', html: D.outText.split('\n').filter(Boolean).map(l => {
          const m = l.match(/^(.*?):(\d+):(.*)$/); if (!m) return '<div class="h"><span class="f">' + esc(short(l)) + '</span></div>';
          return '<div class="h"><span class="f">' + esc(short(m[1])) + '<span class="ln">:' + m[2] + '</span></span><span class="x">' + (re ? esc(m[3].trim()).replace(re, '<mark>$1</mark>') : esc(m[3].trim())) + '</span></div>';
        }).join('') }));
      } else if (D.outText && it.status !== 'failed' && looksMd(D.outText)) body.append(mdNode(D.outText, 'hx-msg hx-mdout'));
      else if (D.outText) body.append(el('div', { class: it.status === 'failed' ? 'hx-err' : 'hx-out' }, D.outText.length > 1600 ? D.outText.slice(0, 1600) + '…' : D.outText));
      else if (it.status === 'failed') body.append(el('div', { class: 'hx-err' }, it.reason || 'failed'));
      if (body.childNodes.length) { if (it._opened) body.classList.add('noanim'); it._opened = true; n.append(body); }
      return n;
    }
    function standaloneTerm(it) {
      const b = it.body || {}, cmd = (b.command || (it.head && it.head.command) || '').replace(/^bash -c /, '');
      return el('div', { class: 'hx-tool open ' + (it.status === 'running' ? 'run' : it.status === 'completed' ? 'done' : 'fail'), 'data-kind': 'execute' },
        el('div', { class: 'hx-th' }, el('span', { class: 'hx-ki execute' }, icon('execute')), el('span', { class: 'hx-verb' }, 'Run'), el('span', { class: 'hx-tgt mono' }, cmd)),
        el('div', { class: 'hx-tb' }, el('div', { class: 'hx-term' }, el('div', { class: 'hx-cmd' }, el('span', { class: 'p' }, '$'), cmd), it.live ? termOut(it.live) : null)));
    }
    function approvalCard(it) {
      const b = it.body, sub = b.subject;
      const n = el('div', { class: 'hx-approve' + (b.decision ? ' resolved' : '') }, el('div', { class: 'hx-aq' }, icon('shield'), el('span', {}, 'Allow ', el('b', {}, sub.effect), '?'), el('code', {}, sub.command || sub.summary)));
      if (!b.decision) n.append(el('div', { class: 'hx-abtn' }, ...b.options.map((o, i) => el('button', { type: 'button', class: i === 0 ? 'pri' : '' }, { allow: 'Allow', allow_always: 'Always allow', deny: 'Deny' }[o] || o, el('span', { class: 'k' }, String(i + 1))))));
      else n.append(approvalBadge(it));
      return n;
    }
    function planCard(it) {
      const entries = (it.body && it.body.entries) || [];
      return el('div', { class: 'hx-plan' }, el('div', { class: 'hx-ph' }, icon('plan'), 'Plan', el('span', { class: 'hx-sp' }), entries.filter(x => x.status === 'completed').length + '/' + entries.length),
        ...entries.map(x => el('div', { class: 'pe ' + x.status }, el('i'), x.text)));
    }
    // The turn receipt. end_turn is a quiet green line; a stop the user asked for (interrupted, cancelled) is neutral;
    // anything else is red, says what happened in words, and offers the two ways out: try again, or another profile.
    const STOP_SAY = { max_tokens: 'Hit the output limit', max_rounds: 'Stopped at the round limit', interrupted: 'Interrupted', cancelled: 'Cancelled', unknown: 'Stopped' };
    const CLASS_SAY = { transient: 'Connection dropped', rate_limited: 'Rate limited', auth: 'Not signed in', bad_request: 'Request refused', malformed: 'Unreadable reply', mid_stream: 'Reply cut off', config: 'Profile misconfigured', store: 'Could not save the session', tool: 'Tool failed', internal: 'Harness error', cancelled: 'Cancelled' };
    function turnEnd(turn) {
      const t = s.turns.get(turn) || {}, R = t.reason || {}, r = R.reason || 'end_turn', u = t.usage || {};
      const ok = r === 'end_turn', soft = r === 'interrupted' || r === 'cancelled';
      const title = ok ? 'Done' : r === 'error' ? CLASS_SAY[R.class] || 'Failed' : STOP_SAY[r] || r.replace(/_/g, ' ');
      const L = t.loop, plural = (n, w) => n + ' ' + w + (n === 1 ? '' : 's');
      const n = el('div', { class: 'hx-end ' + (ok ? 'ok' : soft ? 'soft' : 'bad') },
        el('span', { class: 'hx-endmark' }, icon(ok ? 'check' : soft ? 'stop' : 'x')),
        el('b', {}, title),
        (t.t1 || 0) - (t.t0 || 0) >= 1000 || !ok ? el('span', {}, ms((t.t1 || 0) - (t.t0 || 0))) : null,
        t.tools ? el('span', {}, plural(t.tools, 'tool')) : null, t.edits ? el('span', {}, plural(t.edits, 'edit')) : null,
        u.input_tokens ? el('span', {}, kfmt(u.input_tokens) + ' in · ' + kfmt(u.output_tokens) + ' out') : null,
        u.cost_micros_usd ? el('span', {}, '$' + (u.cost_micros_usd / 1e6).toFixed(u.cost_micros_usd < 1e4 ? 4 : 2)) : null,
        L ? el('span', { class: 'warn' }, icon('loop'), plural(L.items.filter(id => (item(id) || {}).kind === 'agent_message').length, 'repeated reply') + ' folded') : null);
      if (!ok && !soft) n.append(el('span', { class: 'why' }, R.message || 'The turn ended without an answer.'),
        el('span', { class: 'hx-eact' }, el('button', { type: 'button', class: 'pri', onclick: () => opts.onOp && opts.onOp({ type: 'retry', turn }) }, icon('retry'), 'Retry'),
          el('button', { type: 'button', onclick: e => { e.stopPropagation(); toggleMenu(); } }, 'Switch profile')));
      return n;
    }

    // ---------- runaway loop: an agent answering the same thing again in one turn ----------
    // The second reply whose first line matches an earlier one in the turn opens a loop card; everything the agent does
    // after that in the turn folds into it. The first answer stays in the transcript; the repeats stay one click away.
    const lineKey = md => String(md || '').trim().split('\n')[0].toLowerCase().replace(/[^a-z ]+/g, '').replace(/\s+/g, ' ').trim().slice(0, 48);
    function startLoop(t, it) {
      const L = { id: 'loop:' + it.turn, items: [], open: false };
      L.head = el('button', { type: 'button', class: 'hx-lh', onclick: () => { L.open = !L.open; L.node.classList.toggle('open', L.open); } });
      L.body = el('div', { class: 'hx-lb' });
      L.node = el('div', { class: 'hx-loop hx-in' }, L.head, L.body);
      t.loop = L; nodes.set(L.id, L.node);
      const old = nodes.get(it.id); if (old) old.replaceWith(L.node); else feed.append(L.node);
      L.items.push(it.id); if (old) L.body.append(old);
      drawLoop(L);
    }
    function drawLoop(L) {
      const c = k => L.items.filter(id => (item(id) || {}).kind === k).length;
      const bits = [[c('agent_message'), 'reply', 'replies'], [c('reasoning'), 'thought', 'thoughts'], [c('tool_call'), 'tool call', 'tool calls']].filter(x => x[0]).map(x => x[0] + ' ' + (x[0] === 1 ? x[1] : x[2]));
      L.head.replaceChildren(el('span', { class: 'hx-ki loop' }, icon('loop')), el('b', {}, 'The agent kept repeating itself'), el('span', { class: 'hx-gist' }, bits.join(' · ')), el('span', { class: 'hx-sp' }), el('span', { class: 'hx-chip' }, 'folded'), icon('chev', 'chev'));
    }

    // ---------- header: identity, live phase, meter ----------
    function phaseNow() {
      if (s.closed) return ['idle', 'Session closed'];
      for (const it of [...s.items.values()].reverse()) {
        if (it.status !== 'running') continue;
        if (it.kind === 'reasoning') return ['think', 'Thinking'];
        if (it.kind === 'agent_message') return ['write', 'Writing'];
        if (it.kind === 'tool_call') { const D = describe(it); return it.approval && !(it.approval.body && it.approval.body.decision) ? ['ask', 'Waiting for you · ' + D.verb] : ['tool', D.verb + ' ' + (D.target || '')]; }
      }
      return s.turn ? ['think', 'Working'] : ['idle', 'Ready'];
    }
    function drawHead() {
      const [ph, say] = phaseNow(); node.dataset.phase = ph; comp.hx.sync(ph);
      empty();
      const acp = acpModel(s.model);
      if (opts.onPhase) opts.onPhase(ph);
      const u = s.usage;
      head.replaceChildren(...[
        el('span', { class: 'hx-mark' }, el('img', { src: 'bboxy.png', alt: '' })),
        el('span', { class: 'hx-name' }, 'bboxy'),
        s.model || s.profile ? el('button', { type: 'button', class: 'hx-model' + (menu ? ' on' : '') + (acp ? ' acp' : ''), title: 'Switch profile', onclick: e => { e.stopPropagation(); toggleMenu(); } },
          ...(acp ? [provMark({ kind: 'acp-agent', agent_command: acp.agent.split(/\s+/) }, 'sm'), el('span', { class: 'hx-mt' }, s.profile || acp.agent, acp.model ? el('span', { class: 'dim' }, ' · ' + acp.model) : null), el('span', { class: 'hx-kind' }, 'ACP')]
            : [el('i'), el('span', { class: 'hx-mt' }, s.profile && s.profile !== s.model ? s.profile + ' · ' + s.model : s.model || s.profile)]), icon('chev', 'down')) : null,
        cwd() ? el('span', { class: 'hx-cwd' }, icon('branch'), cwd().split('/').pop()) : null,
        el('span', { class: 'hx-sp' }),
        el('span', { class: 'hx-phase ' + ph }, el('i'), say.length > 48 ? say.slice(0, 47) + '…' : say),
        u.in ? el('span', { class: 'hx-meter', title: kfmt(u.in) + ' input · ' + kfmt(u.cached) + ' cached · ' + kfmt(u.out) + ' output' + (u.cost ? ' · $' + (u.cost / 1e6).toFixed(4) : '') }, el('span', { class: 'bar' }, el('i', { style: 'width:' + Math.min(100, u.in / 2000) + '%' })), kfmt(u.in + u.out)) : null].filter(Boolean));
    }
    let menu = null;
    function toggleMenu(force) {
      if (menu || force === false) { if (menu) menu.remove(); menu = null; drawHead(); return; }
      menu = profileMenu(s, opts, () => toggleMenu(false)); node.append(menu); drawHead();
    }
    node.addEventListener('click', () => { if (menu) toggleMenu(false); });
    function reset() { s = fresh(); nodes.clear(); feed.replaceChildren(); if (menu) toggleMenu(false); drawHead(); }
    drawHead();
    return { node, apply, reset, setProfiles(list) { opts.profiles = list; }, openProfiles() { if (!menu) toggleMenu(); }, compose(o) { comp.hx.set(o); }, get state() { return s; } };
  }

  // ---------- composer ----------
  // One rounded box. Its look follows two inputs: the session phase (idle, running, waiting on an approval) and the
  // local edit state (focused, has text). The "/" menu and the queued follow-ups hang above it, anchored to its top edge.
  //   comp.hx.set({ focus, text, queued, menu: 'slash' | null, sel })   comp.hx.sync(phase)
  const COMMANDS = [
    { cmd: '/interrupt', say: 'Stop the running turn', key: 'esc', icon: 'stop' },
    { cmd: '/stop', say: 'Close this session', icon: 'x' },
    { cmd: '/profiles', say: 'Open the profile switcher', icon: 'plug' },
  ];
  const PH = { idle: 'Ask bboxy, @ a file, / a command', focus: 'Ask bboxy, @ a file, / a command', typing: '', run: 'Queue a follow-up…', ask: 'Answer the approval above' };
  const HINT = { idle: '', focus: '', typing: '⏎ send · ⇧⏎ newline', run: 'esc interrupt', ask: '1 allow · 2 always · 3 deny' };
  const tokens = t => esc(t).replace(/^(\/[a-z]+)/, '<span class="tok cmd">$1</span>').replace(/(^|\s)(@[\w./-]+)/g, '$1<span class="tok at">$2</span>');
  function slashChoices(query, profiles) {
    return [...COMMANDS, ...profiles.map(p => ({ cmd: '/profile ' + p.name, say: isAgent(p) ? (p.agent_command || []).join(' ') + (p.model ? ' · ' + p.model : '') : p.model, profile: p }))]
      .filter(c => c.cmd.slice(1).startsWith(query));
  }
  function slashMenu(query, sel, profiles, choose) {
    const hits = slashChoices(query, profiles); if (!hits.length) return null;
    const row = (c, i) => el('button', { type: 'button', class: 'hx-crow' + (i === sel ? ' sel' : ''), onmousedown: e => { e.preventDefault(); choose(c); } },
      c.profile ? provMark(c.profile, 'sm') : el('span', { class: 'hx-ct' }, icon(c.icon)),
      el('span', { class: 'hx-cc' }, el('b', {}, c.cmd.slice(0, query.length + 1)), c.cmd.slice(query.length + 1)),
      el('span', { class: 'hx-cd' }, c.say || ''),
      c.key ? el('kbd', {}, c.key) : c.profile && isAgent(c.profile) ? el('span', { class: 'hx-kind' }, 'ACP') : null);
    const cmds = hits.filter(c => !c.profile), profs = hits.filter(c => c.profile);
    let k = 0;
    // at most eight rows show; the list scrolls and keeps the selected row in view
    const rows = el('div', { class: 'hx-crows' },
      cmds.length ? el('div', { class: 'hx-pgrp' }, 'Commands') : null, ...cmds.map(c => row(c, k++)),
      profs.length ? el('div', { class: 'hx-pgrp' }, 'Switch profile') : null, ...profs.map(c => row(c, k++)));
    requestAnimationFrame(() => { const r = rows.querySelector('.sel'); if (r && r.scrollIntoView) r.scrollIntoView({ block: 'nearest' }); });
    return el('div', { class: 'hx-cmenu', role: 'listbox' }, rows,
      el('div', { class: 'hx-cfoot' }, el('kbd', {}, '↑'), el('kbd', {}, '↓'), ' select', el('kbd', {}, '⏎'), ' run', el('kbd', {}, 'esc'), ' close', el('span', { class: 'hx-sp' }), hits.length + (hits.length === 1 ? ' match' : ' matches')));
  }
  function composer(opts, api) {
    const st = { focus: false, text: '', queued: [], menu: null, sel: 0, phase: 'idle' };
    const input = el('div', { class: 'hx-input', contenteditable: opts.interactive ? 'true' : 'false', spellcheck: 'false' });
    const hint = el('span', { class: 'hx-hint' }), send = el('button', { type: 'button', class: 'hx-send' }), qchip = el('span', { class: 'hx-chip' });
    const box = el('div', { class: 'hx-box' }, el('i', { class: 'hx-sweep' }), input,
      el('div', { class: 'hx-tools' }, el('span', { class: 'hx-chip' }, '@ context'), qchip, el('span', { class: 'hx-sp' }), hint, send));
    const menuHost = el('div', { class: 'hx-cmenu-host' }), queue = el('div', { class: 'hx-queue' });
    // the alive rim: the same shader the term's panes wear (glow.js, spec rim-glow), live off box.dataset.rimState; the top sweep is only the fallback
    const rimWrap = el('div', { class: 'glow-wrap hx-rimwrap' }, box);
    if (window.Glow && window.C) { box.classList.add('rimmed'); window.Glow.attach(rimWrap, { key: 'composer' }, { box, live: true, block: true }); }
    const node = el('div', { class: 'hx-comp' }, menuHost, queue, rimWrap);
    const query = () => /^\/[^\n]*$/.test(st.text) ? st.text.slice(1) : null;
    const choices = () => query() == null ? [] : slashChoices(query(), api.profiles());
    function choose(c) {
      st.text = ''; st.menu = null; input.textContent = '';
      if (c.profile) opts.onOp && opts.onOp({ type: 'set_profile', profile: c.profile.name }, c.profile);
      else if (c.cmd === '/profiles') api.openProfiles();
      paint();
    }
    function paint() {
      const state = st.phase === 'ask' ? 'ask' : st.phase !== 'idle' ? 'run' : st.text ? 'typing' : st.focus ? 'focus' : 'idle';
      node.dataset.state = state; box.dataset.focus = st.focus ? 'true' : 'false';
      box.dataset.rimState = state === 'run' ? 'active' : state === 'ask' ? 'waiting' : 'idle'; box.classList.toggle('focused', st.focus);
      input.dataset.ph = PH[state];
      hint.textContent = st.text && state === 'run' ? '⏎ queue · esc interrupt' : HINT[state];
      const stop = state === 'run' && !st.text;
      send.className = 'hx-send' + (st.text ? ' ready' : '') + (stop ? ' stop' : '');
      send.title = stop ? 'Interrupt (esc)' : state === 'run' ? 'Queue' : 'Send';
      send.replaceChildren(icon(stop ? 'stop' : 'send'));
      qchip.textContent = st.queued.length ? 'queue · ' + st.queued.length : 'queue'; qchip.classList.toggle('on', !!st.queued.length);
      queue.replaceChildren(...st.queued.map((t, i) => el('div', { class: 'hx-qi' }, icon('clock'), el('span', { class: 't' }, t), el('span', { class: 'w' }, i ? 'after #' + i : 'after this turn'),
        el('button', { type: 'button', class: 'x', title: 'Drop', onclick: () => { st.queued.splice(i, 1); paint(); } }, icon('x')))));
      const m = st.menu === 'slash' && query() != null ? slashMenu(query(), st.sel, api.profiles(), choose) : null;
      menuHost.replaceChildren(...(m ? [m] : []));
    }
    input.addEventListener('focus', () => { st.focus = true; if (window.Glow) window.Glow.swell(rimWrap); paint(); });
    input.addEventListener('blur', () => { st.focus = false; st.menu = null; paint(); });
    input.addEventListener('input', () => { st.text = input.textContent; st.menu = query() != null ? 'slash' : null; st.sel = 0; paint(); });
    input.addEventListener('keydown', e => {
      const hits = choices();
      if (st.menu && hits.length) {
        if (e.key === 'ArrowDown' || e.key === 'ArrowUp') { e.preventDefault(); st.sel = (st.sel + (e.key === 'ArrowDown' ? 1 : hits.length - 1)) % hits.length; paint(); return; }
        if (e.key === 'Enter') { e.preventDefault(); choose(hits[st.sel]); return; }
        if (e.key === 'Escape') { e.preventDefault(); st.menu = null; paint(); return; }
      }
      if (e.key === 'Enter' && !e.shiftKey && st.text.trim()) {
        e.preventDefault();
        if (st.phase !== 'idle') st.queued.push(st.text.trim()); else opts.onOp && opts.onOp({ type: 'send', text: st.text.trim() });
        st.text = ''; input.textContent = ''; paint();
      }
    });
    node.hx = {
      set(o) { Object.assign(st, o); if ('text' in o) input.innerHTML = tokens(st.text); if (o.menu) st.menu = o.menu; paint(); },
      sync(phase) { if (phase === st.phase) return; st.phase = phase; paint(); },
    };
    paint();
    return node;
  }

  // ---------- replay ----------
  // The session log keeps completed items; replay re-streams their text at reading pace and holds each tool for a plausible
  // run time, so a stored session plays like a live one. The fold sees the same envelope shapes a live gateway sends.
  function play(v, envs, opts) {
    opts = opts || {};
    const speed = opts.speed || 1; let timer = null, i = 0, stopped = false, clock = 0;
    const HOLD = { read: 380, search: 520, execute: 1500, edit: 700, other: 600 };
    const queue = [], done = new Map();
    for (const env of envs) if (env.event.type === 'item_completed') done.set(env.event.item, env.event.body);
    for (const env of envs) {
      const e = env.event;
      if (e.type === 'item_started' && (e.kind === 'reasoning' || e.kind === 'agent_message')) {
        // a live gateway sends these deltas between start and completion; the log keeps only the completed text
        const b = done.get(e.item) || {}, why = e.kind === 'reasoning', text = why ? b.summary || '' : b.markdown || '';
        queue.push([why ? 120 : 160, env]);
        const parts = text.match(/\S+\s*/g) || [];
        for (let k = 0; k < parts.length; k += 2) queue.push([why ? 16 : 24, { ...env, event: { type: 'item_delta', item: e.item, delta: { type: why ? 'reasoning' : 'text', append: parts.slice(k, k + 2).join('') } } }]);
      } else if (e.type === 'item_completed' && (e.body.kind === 'reasoning' || e.body.kind === 'agent_message')) queue.push([60, env]);
      else if (e.type === 'item_completed' && e.body.kind === 'tool_call') queue.push([HOLD[e.body.tool_kind] || 600, env]);
      else if (e.type === 'item_completed' && e.body.kind === 'approval') queue.push([e.body.decision ? 160 : 900, env]);
      else if (e.type === 'item_started' && e.kind === 'tool_call') {
        const b = done.get(e.item) || {};
        queue.push([160, env]);
        if (b.input_raw) queue.push([40, { ...env, event: { type: 'item_delta', item: e.item, delta: { type: 'tool_input', append: b.input_raw } } }]);
      }
      else if (e.type === 'item_started') queue.push([e.kind === 'user_message' ? 500 : 90, env]);
      else if (e.type === 'turn_ended') queue.push([300, env]);
      else queue.push([0, env]);
    }
    function step() {
      if (stopped) return;
      if (i >= queue.length) { if (opts.onEnd) opts.onEnd(); if (!opts.loop) return; timer = setTimeout(() => { v.reset(); i = 0; clock = 0; step(); }, (opts.rest || 7000) / speed); return; }
      const [wait, env] = queue[i];
      timer = setTimeout(() => { clock += wait; v.apply({ ...env, ts_ms: clock }); i++; step(); }, wait / speed);
    }
    step();
    return { stop() { stopped = true; clearTimeout(timer); } };
  }

  // demo profiles for the lab and the public showcase: the shapes of a real profiles/ directory, no machine's actual config
  const DEMO_PROFILES = [
    // ACP agents: the shapes harness_profiles reports on a real machine (kind, agent command, env names; never values)
    { name: 'claude-acp', kind: 'acp-agent', provider: 'acp-agent', model: 'claude-haiku-5-5', agent_command: ['claude-agent-acp'], agent_env_names: [] },
    { name: 'codex-acp', kind: 'acp-agent', provider: 'acp-agent', model: '', agent_command: ['codex-acp'], agent_env_names: [] },
    { name: 'opencode-acp', kind: 'acp-agent', provider: 'acp-agent', model: '', agent_command: ['opencode', 'acp'], agent_env_names: [] },
    { name: 'pi-acp', kind: 'acp-agent', provider: 'acp-agent', model: 'claude-haiku-5-5', agent_command: ['pi-acp'], agent_env_names: ['ANTHROPIC_API_KEY'] },
    { name: 'omp-acp', kind: 'acp-agent', provider: 'acp-agent', model: '', agent_command: ['omp', 'acp'], agent_env_names: [] },
    { name: 'goose-acp', kind: 'acp-agent', provider: 'acp-agent', model: '', agent_command: ['goose', 'acp'], agent_env_names: [] },
    { name: 'sonnet', kind: 'model', provider: 'openrouter', model: 'anthropic/claude-sonnet-5-5', endpoint: 'https://openrouter.ai/api/v1', auth: { kind: 'env', ref: 'OPENROUTER_API_KEY', ok: true }, context_tokens: 1000000, default: true },
    { name: 'nemotron', provider: 'openrouter', model: 'nvidia/nemotron-3.5-super-vl:free', endpoint: 'https://openrouter.ai/api/v1', auth: { kind: 'env', ref: 'OPENROUTER_API_KEY', ok: true }, context_tokens: 262144 },
    { name: 'opus', provider: 'anthropic', model: 'claude-opus-5-5', endpoint: 'https://api.anthropic.com/v1', auth: { kind: 'keychain', ref: 'bboxy450-harness-anthropic', ok: true }, context_tokens: 1000000 },
    { name: 'haiku', provider: 'anthropic', model: 'claude-haiku-5-5', endpoint: 'https://api.anthropic.com/v1', auth: { kind: 'keychain', ref: 'bboxy450-harness-anthropic', ok: true }, context_tokens: 200000 },
    { name: 'max', provider: 'anthropic-subscription', model: 'claude-opus-5-5', endpoint: '', auth: { kind: 'login', ref: 'Claude Code login' } },
    { name: 'gpt', provider: 'openai', model: 'gpt-5.1', endpoint: 'https://api.openai.com/v1', auth: { kind: 'env', ref: 'OPENAI_API_KEY', ok: false } },
    { name: 'qwen-local', provider: 'local', model: 'qwen3-coder', endpoint: 'http://localhost:11434/v1', auth: { kind: 'none', ok: true }, context_tokens: 131072 },
  ];
  // a face without a gateway (lab, site) answers its own ops the way the harness would: SetProfile → ProfileChanged
  function demoOps(getView, list) {
    list = list || DEMO_PROFILES;
    return (op, p) => {
      const v = getView();
      if (op.type === 'create_profile' && !list.some(x => x.name === p.name)) list.push({ ...p, auth: { ...p.auth, ok: p.auth.kind !== 'env' } });
      if (op.type !== 'set_profile' && op.type !== 'create_profile') return;
      const model = isAgent(p) ? 'acp:' + (p.agent_command || ['agent'])[0] + (p.model ? ' · ' + p.model : '') : p.model;
      v.apply({ v: { major: 0, minor: 4 }, session: '', epoch: 1, seq: null, at: 0, ts_ms: Date.now(), cause: null, event: { type: 'profile_changed', profile: p.name, model } });
    };
  }

  window.Hx = { view, play, markdown, diffView, highlight, PROVIDERS, AGENTS, DEMO_PROFILES, demoOps, profileYaml, composer, slashMenu };
})();
