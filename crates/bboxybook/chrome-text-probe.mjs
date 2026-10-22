// Raster-only fixture: explicit shared baselines isolate layout from coverage.
// Isolated ephemeral Chrome, never an operator browser. No external resources.
// node crates/bboxybook/chrome-text-probe.mjs OUT
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import vm from 'node:vm';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';

const out = path.resolve(process.argv[2] || 'target/bboxybook/chrome-text');
await fs.mkdir(out, {recursive: true});
const source = await fs.readFile('lab/web/tokens.js', 'utf8');
const context = {window: {}};
vm.runInNewContext(source, context, {timeout: 1000});
const tokens = context.window.TOKENS;
const raw = [], palette = [], phases = [];
for (const mono of [false, true]) for (const weight of [400, 500, 600, 700]) {
  for (const size of [11, 12.5, 16]) for (const dark of [false, true]) {
    for (const phase of [0, .25, .5, .75]) {
      raw.push({mono, weight, size, dark, phase, fg: dark ? '#ffffff' : '#000000',
        bg: dark ? '#000000' : '#ffffff', text: 'Hamburgefonts 0123 AV fi', id: raw.length});
    }
  }
}
for (const mono of [false, true]) for (const weight of [400, 600]) {
  for (const dark of [false, true]) for (const color of ['fg', 'fg_dim', 'accent']) {
    for (const phase of [0, .25, .5, .75]) {
      const theme = tokens.color[dark ? 'dark' : 'light'];
      palette.push({mono, weight, size: 13, dark, phase, color, fg: theme[color],
        bg: theme.bg, text: 'Vesper Greywind 0123 AV', id: palette.length});
    }
  }
}
// Non-quarter origins detect premature rounding of a whole shaped line.
for (const mono of [false, true]) for (const weight of [400, 500, 600, 700]) {
  for (const dark of [false, true]) for (const phase of [.0625, .1875, .375, .9]) {
    phases.push({mono, weight, size: 13, dark, phase,
      fg: dark ? '#ffffff' : '#000000', bg: dark ? '#000000' : '#ffffff',
      text: 'Hamburgefonts 0123 AV fi', id: phases.length});
  }
}
const italic = raw.map(c => ({...c, italic: true}));
const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'bboxy-raster-'));
const browser = spawn('/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', [
  '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
  '--no-first-run', '--disable-extensions', '--hide-scrollbars', 'about:blank'
], {stdio: ['ignore', 'ignore', 'pipe']});
browser.stderr.on('data', () => {});
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
let ws;
try {
  let port;
  for (let i = 0; i < 120; i++) {
    try {port = +(await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]; break;}
    catch {await pause(50);}
  }
  if (!port) throw Error('Isolated Chrome did not start');
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  ws = new WebSocket(tabs.find(t => t.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, {once: true});
    ws.addEventListener('error', reject, {once: true});
  });
  let id = 0;
  const pending = new Map(), errors = [];
  ws.addEventListener('message', event => {
    const m = JSON.parse(event.data);
    if (m.id) {
      const p = pending.get(m.id);
      if (!p) return;
      pending.delete(m.id);
      clearTimeout(p.timer);
      m.error ? p.reject(Error(JSON.stringify(m.error))) : p.resolve(m.result);
    } else if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails);
  });
  const call = (method, params = {}) => new Promise((resolve, reject) => {
    const current = ++id;
    const timer = setTimeout(() => {pending.delete(current); reject(Error(`CDP timeout: ${method}`));}, 15000);
    pending.set(current, {resolve, reject, timer});
    ws.send(JSON.stringify({id: current, method, params}));
  });
  const evaluate = async expression => {
    const r = await call('Runtime.evaluate', {expression, returnByValue: true, awaitPromise: true});
    if (r.exceptionDetails) throw Error(JSON.stringify(r.exceptionDetails));
    return r.result.value;
  };
  await call('Page.enable');
  await call('Runtime.enable');
  await call('DOM.enable');
  await call('CSS.enable');
  const platformFonts = [];
  for (const [prefix, cases] of [['', raw], ['palette-', palette], ['phase-', phases], ['italic-', italic]]) {
    const height = Math.ceil(cases.length / 4) * 48;
    for (const scale of [1, 2]) {
      await call('Emulation.setDeviceMetricsOverride', {width: 1360, height,
        deviceScaleFactor: scale, mobile: false});
      await call('Page.navigate', {url: 'about:blank'});
      await evaluate(`(() => {
        document.body.innerHTML = '';
        document.head.innerHTML = '<style>body{margin:0;background:#777}.cell{position:absolute;width:340px;height:48px;overflow:hidden}.text{position:absolute;top:8px;white-space:pre;line-height:32px;-webkit-font-smoothing:antialiased}.base{display:inline-block;width:0;height:0;vertical-align:baseline}</style>';
        const cases = ${JSON.stringify(cases)}, fonts = ${JSON.stringify(tokens.font)};
        for (const c of cases) {
          const cell = document.createElement('div');
          cell.className = 'cell';
          cell.style.cssText = 'left:' + ((c.id % 4) * 340) + 'px;top:' + (Math.floor(c.id / 4) * 48) + 'px;background:' + c.bg;
          const text = document.createElement('span');
          text.className = 'text'; text.id = 's' + c.id;
          text.style.cssText = 'left:' + (8 + c.phase) + 'px;color:' + c.fg + ';font-family:' + (c.mono ? fonts.mono : fonts.chrome) + ';font-size:' + c.size + 'px;font-weight:' + c.weight + ';font-style:' + (c.italic ? 'italic' : 'normal');
          text.append(document.createTextNode(c.text));
          const baseline = document.createElement('i');
          baseline.className = 'base'; text.append(baseline);
          cell.append(text); document.body.append(cell);
        }
      })()`);
      await evaluate('document.fonts.ready');
      const measurements = await evaluate(`(${JSON.stringify(cases)}).map(c => {
        const text = document.querySelector('#s' + c.id), r = text.getBoundingClientRect();
        return {...c, x: r.x, baseline: text.lastElementChild.getBoundingClientRect().y,
          cell: {x: (c.id % 4) * 340, y: Math.floor(c.id / 4) * 48, w: 340, h: 48},
          font: getComputedStyle(text).font};
      })`);
      const shot = await call('Page.captureScreenshot', {format: 'png', captureBeyondViewport: true,
        clip: {x: 0, y: 0, width: 1360, height, scale: 1}});
      await fs.writeFile(path.join(out, `${prefix}web-${scale}x.png`), Buffer.from(shot.data, 'base64'));
      await fs.writeFile(path.join(out, `${prefix}cases-${scale}x.json`), JSON.stringify(measurements, null, 2));
      if (!prefix || prefix === 'italic-') {
        const doc = await call('DOM.getDocument');
        for (const index of [0, 24, 48, 72, 96, 120, 144, 168]) {
          const node = await call('DOM.querySelector', {nodeId: doc.root.nodeId, selector: '#s' + index});
          platformFonts.push({scale, kind: prefix || 'raw', id: index,
            ...await call('CSS.getPlatformFontsForNode', {nodeId: node.nodeId})});
        }
      }
    }
  }
  await fs.writeFile(path.join(out, 'chrome.json'), JSON.stringify({browser: version.Browser,
    fonts: platformFonts, token_sha256: createHash('sha256').update(source).digest('hex'),
    font_families: tokens.font, profile: 'isolated ephemeral', errors}, null, 2));
  if (errors.length) throw Error(`${errors.length} browser exceptions`);
  console.log('Captured raw 192 + palette 96 + fractional-origin 64 + italic 192 cases at 1x and 2x; zero exceptions');
} finally {
  if (ws) ws.close();
  browser.kill('SIGTERM');
  await Promise.race([new Promise(resolve => browser.once('exit', resolve)), pause(2000)]);
  if (browser.exitCode === null) browser.kill('SIGKILL');
  await fs.rm(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 100});
}
