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
  function markdown(src) {
    const out = []; let list = null, code = null;
    const flush = () => { if (list) { out.push('<' + list.tag + '>' + list.items.map(x => '<li>' + inline(x) + '</li>').join('') + '</' + list.tag + '>'); list = null; } };
    for (const line of src.split('\n')) {
      if (line.startsWith('```')) { if (code) { out.push('<pre class="hx-code">' + code.map(highlight).join('\n') + '</pre>'); code = null; } else { flush(); code = []; } continue; }
      if (code) { code.push(line); continue; }
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

  // ---------- the fold ----------
  function fresh() { return { meta: null, model: '', profile: '', items: new Map(), blocks: [], usage: { in: 0, out: 0, cached: 0, reasoning: 0, cost: 0 }, turns: new Map(), closed: false, phase: null }; }
  function view(opts) {
    opts = opts || {};
    let s = fresh();
    const nodes = new Map();
    const head = el('div', { class: 'hx-head' }), feed = el('div', { class: 'hx-feed' }), scroller = el('div', { class: 'hx-scroll' }, feed);
    const comp = composer(opts);
    const node = el('div', { class: 'hx' + (opts.compact ? ' compact' : '') }, head, scroller, comp);
    let stick = true;
    scroller.addEventListener('scroll', () => { stick = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 40; });
    const follow = () => { if (stick) scroller.scrollTop = scroller.scrollHeight; };

    function item(id) { return s.items.get(id); }
    function toolOf(it) { return it && it.parent ? item(it.parent) : null; }
    function apply(env) {
      const e = env.event, ts = env.ts_ms != null ? env.ts_ms : Date.now();
      switch (e.type) {
        case 'session_started': s.meta = e.meta; s.profile = e.meta.profile || ''; break;
        case 'profile_changed': s.profile = e.profile; s.model = e.model; break;
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
    function agentMsg(it) {
      const md = it.body ? it.body.markdown : it.text, live = !it.body;
      const n = el('div', { class: 'hx-msg' + (live ? ' live' : ''), html: markdown(md) });
      if (live) (n.lastElementChild || n).append(el('span', { class: 'hx-caret' }));
      return n;
    }
    function thought(it) {
      const text = it.body ? it.body.summary || it.body.raw || '' : it.text, live = !it.body;
      const n = el('div', { class: 'hx-think' + (live ? ' live' : '') + (it.open ? ' open' : '') });
      const took = (it.t1 || 0) - (it.t0 || 0);
      const label = live ? el('span', { class: 'hx-shimmer' }, 'Thinking') : took >= 100 ? el('span', {}, 'Thought for ', el('b', {}, ms(took))) : el('b', {}, 'Thought');
      const gist = text.split(/(?<=[.!?])\s/)[0];
      n.append(el('button', { class: 'hx-tk', type: 'button', onclick: () => { it.open = !it.open; touch(it.id); } }, icon('think', 'spin-slow'), label, live ? null : el('span', { class: 'hx-gist' }, gist), live ? null : icon('chev', 'chev')));
      if (live || it.open) n.append(el('div', { class: 'hx-tbody' }, text, live ? el('span', { class: 'hx-caret' }) : null));
      return n;
    }

    function describe(it) {
      const b = it.body || {}, h = it.head || {}, name = b.name || h.name || '?', kind = b.tool_kind || h.tool_kind || 'other';
      const a = parse(b.input_raw || it.input || '{}');
      const outText = (b.output || []).filter(o => o.type === 'text' || o.type === 'error').map(o => o.text).join('\n');
      const mcp = name.match(/^mcp__(.+?)__(.+)$/);
      if (mcp) return { kind: 'other', verb: mcp[2].replace(/_/g, ' '), target: mcp[1], a, outText, chip: 'mcp' };
      switch (name) {
        case 'read': { const lines = outText ? outText.split('\n').filter(Boolean).length : 0; return { kind, verb: 'Read', target: short(a.path), meta: lines ? lines + ' lines' : '', a, outText }; }
        case 'grep': case 'glob': { const hits = outText ? outText.split('\n').filter(Boolean).length : 0; return { kind, verb: name === 'grep' ? 'Search' : 'Find', target: a.pattern, where: a.path && short(a.path), meta: it.body ? hits + (hits === 1 ? ' hit' : ' hits') : '', a, outText, mono: true }; }
        case 'bash': return { kind: 'execute', verb: 'Run', target: a.command, a, outText, mono: true };
        case 'exec': return { kind: 'execute', verb: 'Run', target: [a.program, ...(a.args || [])].join(' '), a, outText, mono: true };
        case 'edit': case 'write': case 'apply_patch': return { kind: 'edit', verb: name === 'write' ? 'Write' : 'Edit', target: short(a.path || (a.edits && a.edits[0] && a.edits[0].path)), a, outText };
        default: return { kind, verb: name, target: '', a, outText };
      }
    }
    function approvalBadge(ap) {
      if (!ap || !ap.body) return null;
      const d = ap.body.decision, by = ap.body.by || {};
      const who = by.by === 'rule' ? 'rule ' + by.id : by.by === 'face' ? by.name : by.by === 'unattended' ? 'unattended' : by.by || '';
      if (!d) return el('span', { class: 'hx-ap ask' }, icon('shield'), 'needs approval');
      if (d.decision === 'deny') return el('span', { class: 'hx-ap deny', title: (d.reason || 'denied') + (who ? ' · ' + who : '') }, icon('shieldx'), by.by === 'unattended' ? 'denied · no approver' : 'denied');
      return el('span', { class: 'hx-ap ok', title: 'allowed by ' + who }, icon('shield'), by.by === 'rule' ? 'auto' : d.decision === 'allow_always' ? 'always' : 'allowed');
    }
    function tool(it) {
      const D = describe(it), b = it.body, term = it.terminal, ch = it.change;
      const denied = it.approval && it.approval.body && it.approval.body.decision && it.approval.body.decision.decision === 'deny';
      const asking = it.approval && !(it.approval.body && it.approval.body.decision);
      const st = it.status === 'running' ? (asking ? 'ask' : 'run') : denied ? 'deny' : it.status === 'completed' ? 'done' : 'fail';
      const exit = term && term.body ? term.body.exit : null;
      let meta = D.meta || '';
      if (D.kind === 'execute' && exit != null) meta = 'exit ' + exit;
      const stat = ch && ch.body ? diffStat(ch.body.unified) : D.kind === 'edit' && D.outText.startsWith('---') ? diffStat(D.outText) : null;
      const autoOpen = st === 'ask' || st === 'fail' || D.kind === 'execute' || D.kind === 'edit' || (D.verb === 'Search' && D.outText.split('\n').filter(Boolean).length <= 6);
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
        el('span', { class: 'hx-dur' }, it.t1 ? ms(it.t1 - it.t0) : ''),
        el('span', { class: 'hx-st' }, st === 'run' ? el('i', { class: 'hx-dots' }, el('i'), el('i'), el('i')) : st === 'ask' ? icon('shield') : icon(st === 'done' ? 'check' : 'x')));
      n.append(th);
      // a rebuild must not replay entrance motion: the status pops once per status, the body opens once
      if (it._popped === st) th.querySelector('.hx-st').classList.add('noanim'); it._popped = st;
      if (!open) { it._opened = false; return n; }
      const body = el('div', { class: 'hx-tb' });
      if (st === 'ask') { const ap = it.approval.body || {}; body.append(el('div', { class: 'hx-askbar' }, el('span', {}, 'Allow ', el('b', {}, (ap.subject && ap.subject.effect) || 'this'), '?'), el('span', { class: 'hx-sp' }), ...((ap.options || ['allow', 'allow_always', 'deny']).map((o, i) => el('button', { type: 'button', class: i === 0 ? 'pri' : o === 'deny' ? 'no' : '' }, { allow: 'Allow', allow_always: 'Always', deny: 'Deny' }[o] || o, el('span', { class: 'k' }, String(i + 1))))))); }
      else if (st === 'deny') body.append(el('div', { class: 'hx-err' }, (it.approval.body.decision.reason || it.reason || 'denied')));
      else if (D.kind === 'execute') {
        const out = term && term.live ? term.live : D.outText;
        body.append(el('div', { class: 'hx-term' }, el('div', { class: 'hx-cmd' }, el('span', { class: 'p' }, '$'), D.target), out ? termOut(out) : st === 'run' ? el('div', { class: 'hx-out dim' }, el('span', { class: 'hx-caret' })) : null));
      } else if (D.kind === 'edit') {
        const u = ch && ch.body ? ch.body.unified : D.outText.startsWith('---') ? D.outText : '';
        if (u) body.append(diffView(u)); else if (D.outText) body.append(el('div', { class: it.status === 'failed' ? 'hx-err' : 'hx-out' }, D.outText));
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
      } else if (D.outText) body.append(el('div', { class: it.status === 'failed' ? 'hx-err' : 'hx-out' }, D.outText.length > 1600 ? D.outText.slice(0, 1600) + '…' : D.outText));
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
    function turnEnd(turn) {
      const t = s.turns.get(turn) || {}, r = (t.reason && t.reason.reason) || 'end_turn', u = t.usage || {};
      const ok = r === 'end_turn';
      return el('div', { class: 'hx-end ' + (ok ? 'ok' : 'bad') },
        el('span', { class: 'hx-endmark' }, icon(ok ? 'check' : 'x')),
        el('b', {}, ok ? 'Done' : r.replace(/_/g, ' ')),
        el('span', {}, ms((t.t1 || 0) - (t.t0 || 0))),
        el('span', {}, t.rounds + ' rounds'), el('span', {}, t.tools + ' tools'), t.edits ? el('span', {}, t.edits + ' edit' + (t.edits === 1 ? '' : 's')) : null,
        u.input_tokens ? el('span', {}, kfmt(u.input_tokens) + ' in · ' + kfmt(u.output_tokens) + ' out') : null,
        t.reason && t.reason.message ? el('span', { class: 'why' }, t.reason.message) : null);
    }

    // ---------- header: identity, live phase, meter ----------
    function phaseNow() {
      if (s.closed) return ['idle', 'Session closed'];
      for (const it of [...s.items.values()].reverse()) {
        if (it.status !== 'running') continue;
        if (it.kind === 'reasoning') return ['think', 'Thinking'];
        if (it.kind === 'agent_message') return ['write', 'Writing'];
        if (it.kind === 'tool_call') { const D = describe(it); return ['tool', D.verb + ' ' + (D.target || '')]; }
      }
      return s.turn ? ['think', 'Working'] : ['idle', 'Ready'];
    }
    function drawHead() {
      const [ph, say] = phaseNow(); node.dataset.phase = ph;
      if (opts.onPhase) opts.onPhase(ph);
      const u = s.usage;
      head.replaceChildren(...[
        el('span', { class: 'hx-mark' }, el('img', { src: 'bboxy.png', alt: '' })),
        el('span', { class: 'hx-name' }, 'bboxy'),
        s.model || s.profile ? el('span', { class: 'hx-model' }, el('i'), s.model || s.profile) : null,
        cwd() ? el('span', { class: 'hx-cwd' }, icon('branch'), cwd().split('/').pop()) : null,
        el('span', { class: 'hx-sp' }),
        el('span', { class: 'hx-phase ' + ph }, el('i'), say.length > 48 ? say.slice(0, 47) + '…' : say),
        u.in ? el('span', { class: 'hx-meter', title: kfmt(u.in) + ' input · ' + kfmt(u.cached) + ' cached · ' + kfmt(u.out) + ' output' + (u.cost ? ' · $' + (u.cost / 1e6).toFixed(4) : '') }, el('span', { class: 'bar' }, el('i', { style: 'width:' + Math.min(100, u.in / 2000) + '%' })), kfmt(u.in + u.out)) : null].filter(Boolean));
    }
    function reset() { s = fresh(); nodes.clear(); feed.replaceChildren(); drawHead(); }
    drawHead();
    return { node, apply, reset, get state() { return s; } };
  }

  function composer(opts) {
    const input = el('div', { class: 'hx-input', contenteditable: opts.interactive ? 'true' : 'false', 'data-ph': 'Ask bboxy, @ a file, / a command' });
    return el('div', { class: 'hx-comp' }, el('div', { class: 'hx-box' }, input,
      el('div', { class: 'hx-tools' }, el('span', { class: 'hx-chip' }, '@ context'), el('span', { class: 'hx-chip' }, 'queue'), el('span', { class: 'hx-sp' }), el('span', { class: 'hx-hint' }, 'esc interrupt'), el('span', { class: 'hx-send' }, icon('send')))));
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

  window.Hx = { view, play, markdown, diffView, highlight };
})();
