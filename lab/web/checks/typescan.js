// Type at true size (renderer.md §4.4b): no text below the micro step, no SVG text drawn at a viewBox scale other than 1.
// Evaluated in the page; returns [{check, what, where}]. Effective size = font-size × every ancestor's transform scale × zoom.
(() => {
  const MIN = window.STYLE.type.micro, out = [], seen = new Set();
  const add = (check, what, where) => { const k = check + what + where; if (!seen.has(k)) { seen.add(k); out.push({ check, what, where }); } };
  const name = e => e.tagName.toLowerCase() + ((e.getAttribute('class') || '').split(' ')[0] ? '.' + e.getAttribute('class').split(' ')[0] : '');
  const scaleOf = e => { let k = 1; for (let a = e; a && a.nodeType === 1; a = a.parentElement) { const cs = getComputedStyle(a); if (cs.transform && cs.transform !== 'none') { const m = new DOMMatrixReadOnly(cs.transform); k *= Math.hypot(m.a, m.b); } const z = parseFloat(cs.zoom); if (z && z !== 1) k *= z; } return k; };
  for (const svg of document.querySelectorAll('svg')) {
    const texts = svg.querySelectorAll('text'), vb = svg.viewBox && svg.viewBox.baseVal, r = svg.getBoundingClientRect();
    if (!texts.length || !r.width) continue;
    const k = vb && vb.width ? r.width / vb.width / scaleOf(svg) : 1;
    if (Math.abs(k - 1) > 0.005) add('svg-scaled', 'viewBox drawn at ' + k.toFixed(3) + 'x', name(svg));
    for (const t of texts) { const fs = parseFloat(getComputedStyle(t).fontSize) * k * scaleOf(svg); if (fs < MIN - 0.01) add('type-below', '"' + t.textContent.slice(0, 20) + '" at ' + fs.toFixed(2) + 'px', name(svg)); }
  }
  const w = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
  while (w.nextNode()) { const n = w.currentNode, p = n.parentElement; if (!p || !n.textContent.trim() || p.closest('svg')) continue;
    const cs = getComputedStyle(p), f = parseFloat(cs.fontSize); if (!f || cs.visibility === 'hidden' || !p.getClientRects().length) continue;
    const fs = f * scaleOf(p); if (fs < MIN - 0.01) add('type-below', '"' + n.textContent.trim().slice(0, 20) + '" at ' + fs.toFixed(2) + 'px', name(p)); }
  return out;
})()
