// No browser-default chrome: every control is drawn by us. Evaluated in the page; returns [{check, what, where}].
(() => {
  const out = [], seen = new Set();
  const add = (check, what, e) => { const where = e.tagName.toLowerCase() + ((typeof e.className === 'string' && e.className.split(' ')[0]) ? '.' + e.className.split(' ')[0] : ''); const k = check + what + where; if (!seen.has(k)) { seen.add(k); out.push({ check, what, where }); } };
  for (const e of document.querySelectorAll('select, textarea, input, button, summary, progress, meter')) {
    if (!e.getClientRects().length) continue;
    const cs = getComputedStyle(e), ap = cs.appearance;
    if (e.tagName === 'SELECT' && ap !== 'none') add('ua-select', 'native arrow (appearance ' + ap + ')', e);
    if (e.tagName === 'TEXTAREA' && cs.resize !== 'none') add('ua-textarea', 'resize grip (' + cs.resize + ')', e);
    if (e.tagName === 'INPUT' && ['checkbox', 'radio', 'range', 'number', 'color', 'date', 'file'].includes(e.type) && ap !== 'none' && ap !== 'textfield') add('ua-input', e.type + ' (appearance ' + ap + ')', e);
    if (e.tagName === 'SUMMARY' && cs.display === 'list-item') add('ua-summary', 'disclosure marker', e);
    if ((e.tagName === 'PROGRESS' || e.tagName === 'METER') && ap !== 'none') add('ua-input', e.tagName.toLowerCase() + ' (appearance ' + ap + ')', e);
    if (e.tagName === 'BUTTON' && ap !== 'none' && /outset/.test(cs.borderStyle)) add('ua-button', 'outset UA button', e);
  }
  for (const e of document.querySelectorAll('input, textarea, select, button, summary, [tabindex], a[href]')) {
    if (!e.getClientRects().length) continue; e.focus({ preventScroll: true, focusVisible: true }); if (document.activeElement !== e) continue;
    if (getComputedStyle(e).outlineStyle === 'auto') add('ua-focus', 'UA focus ring', e); e.blur();
  }
  for (const e of document.querySelectorAll('*')) { const cs = getComputedStyle(e); if (!/(auto|scroll)/.test(cs.overflowX + cs.overflowY)) continue;
    if ((e.scrollHeight > e.clientHeight + 1 || e.scrollWidth > e.clientWidth + 1) && cs.scrollbarWidth === 'auto' && cs.scrollbarColor === 'auto') add('ua-scrollbar', 'UA scrollbar', e); }
  return out;
})()
