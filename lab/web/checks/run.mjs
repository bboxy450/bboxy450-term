// Lab checks: every lab/web page × density through typescan.js (type at true size) and uascan.js (no browser-default chrome),
// plus page exceptions. Self-tests on bad.html first: each check must fire there, or the checks themselves are broken.
// Exit 0 clean · 1 findings · 2 self-test failed. Isolated headless Chrome profile; never attaches to the operator's browser.
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { readFileSync, readdirSync, existsSync, mkdtempSync, rmSync } from 'node:fs';
import { join, dirname, extname } from 'node:path';
import { tmpdir } from 'node:os';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url)), web = dirname(here);
const SCANS = { type: readFileSync(join(here, 'typescan.js'), 'utf8'), ua: readFileSync(join(here, 'uascan.js'), 'utf8') };
const MUST_FIRE = ['type-below', 'svg-scaled', 'ua-select', 'ua-textarea', 'ua-input', 'ua-summary', 'ua-button', 'ua-focus', 'ua-scrollbar'];
const only = process.argv.slice(2).filter(a => a.endsWith('.html'));
const pages = only.length ? only : readdirSync(web).filter(f => f.endsWith('.html')).sort();

const MIME = { '.html': 'text/html', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.png': 'image/png', '.svg': 'image/svg+xml', '.json': 'application/json', '.woff2': 'font/woff2' };
const server = createServer((req, res) => { const p = join(web, decodeURIComponent(new URL(req.url, 'http://x').pathname));
  if (!p.startsWith(web) || !existsSync(p)) { res.writeHead(404); return res.end(); } res.writeHead(200, { 'content-type': MIME[extname(p)] || 'application/octet-stream' }); res.end(readFileSync(p)); });
await new Promise(r => server.listen(0, '127.0.0.1', r));
const base = 'http://127.0.0.1:' + server.address().port + '/';

const bin = (process.env.CHROME ? [process.env.CHROME] : ['/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome', '/usr/bin/chromium', '/usr/bin/chromium-browser']).find(existsSync);
if (!bin) { console.error('lab-check: no Chrome found (set CHROME)'); server.close(); process.exit(2); }
const sleep = ms => new Promise(r => setTimeout(r, ms));
const prof = mkdtempSync(join(tmpdir(), 'lab-check-'));
const chrome = spawn(bin, ['--headless=new', '--disable-gpu', '--hide-scrollbars', '--remote-debugging-port=0', '--user-data-dir=' + prof, '--window-size=1280,900', 'about:blank'], { stdio: ['ignore', 'ignore', 'pipe'] });
const wsUrl = await new Promise((ok, bad) => { let s = ''; chrome.stderr.on('data', d => { s += d; const m = s.match(/DevTools listening on (ws:\S+)/); if (m) ok(m[1]); }); setTimeout(() => bad(new Error('Chrome did not start')), 15000); });
const done = async code => { const gone = new Promise(r => chrome.once('exit', r)); try { chrome.kill(); } catch {} await Promise.race([gone, sleep(3000)]); server.close();
  try { rmSync(prof, { recursive: true, force: true, maxRetries: 5, retryDelay: 100 }); } catch {} process.exit(code); };

const browser = new WebSocket(wsUrl); await new Promise(r => browser.onopen = r);
let id = 0; const pend = new Map(), events = [];
browser.onmessage = m => { const d = JSON.parse(m.data); if (d.id && pend.has(d.id)) { pend.get(d.id)(d); pend.delete(d.id); } else if (d.method) events.push(d); };
const send = (method, params = {}, sessionId) => new Promise(r => { const i = ++id; pend.set(i, r); browser.send(JSON.stringify({ id: i, method, params, ...(sessionId ? { sessionId } : {}) })); });
const { result: { targetId } } = await send('Target.createTarget', { url: 'about:blank' });
const { result: { sessionId } } = await send('Target.attachToTarget', { targetId, flatten: true });
const S = (m, p) => send(m, p, sessionId);
await S('Page.enable'); await S('Runtime.enable');
async function scan(url) {
  events.length = 0; await S('Page.navigate', { url });
  for (let t = 0; t < 100 && !events.some(e => e.method === 'Page.loadEventFired'); t++) await sleep(50);
  await sleep(350);
  const errs = events.filter(e => e.method === 'Runtime.exceptionThrown').map(e => ({ check: 'exception', what: e.params.exceptionDetails.exception?.description?.split('\n')[0] || e.params.exceptionDetails.text, where: 'page' }));
  const out = [...errs];
  for (const k in SCANS) { const r = await S('Runtime.evaluate', { expression: SCANS[k], returnByValue: true });
    if (r.result.exceptionDetails) out.push({ check: 'scan-error', what: k + ': ' + (r.result.exceptionDetails.exception?.description || '').split('\n')[0], where: 'page' }); else out.push(...r.result.result.value); }
  return out;
}

// self-test: the bad page must trip every check
const bad = await scan(base + 'checks/bad.html'), fired = new Set(bad.map(f => f.check)), silent = MUST_FIRE.filter(c => !fired.has(c));
if (silent.length) { console.error('lab-check SELF-TEST FAILED: checks/bad.html did not trip ' + silent.join(', ')); await done(2); }
console.log('lab-check: self-test ok (' + MUST_FIRE.length + ' checks fire on checks/bad.html)');

await S('Page.navigate', { url: base + 'index.html' }); await sleep(600);
const dens = (await S('Runtime.evaluate', { expression: 'Object.keys(TOKENS.density)', returnByValue: true })).result.result.value;
let n = 0;
for (const page of pages) for (const d of dens) {
  for (const f of await scan(base + page + '?density=' + d)) { n++; console.log(`${page} [${d}] ${f.check}: ${f.what} · ${f.where}`); }
}
console.log(`lab-check: ${pages.length} pages × ${dens.length} densities · ${n} finding${n === 1 ? '' : 's'}`);
await done(n ? 1 : 0);
