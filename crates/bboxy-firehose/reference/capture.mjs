// Adapted from bboxybook/capture.mjs: isolated Chrome profile, local files, CDP.
// Never connects to the operator's browser. No CDN or network assets.
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {spawn, execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '../../..');
const binary = path.join(root, 'target/release/bboxy-firehose');
const out = process.env.TERMINAL_OUT ? path.resolve(process.env.TERMINAL_OUT) : path.join(here, 'evidence');
const fixedGrid = process.env.TERMINAL_GRID === '1';
const chrome = process.env.CHROME || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const seed = '42', scale = Number(process.env.SCALE || 1), light = process.env.THEME === 'light';
if (![1, 2].includes(scale)) throw Error('SCALE must be 1 or 2');
const variant = `${light ? 'light' : 'dark'}-${scale}x`;
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const vendor = {};
for (const file of ['xterm.js', 'xterm.css', 'LICENSE', 'addon-unicode11.js', 'unicode11-LICENSE']) {
  vendor[file] = sha256(await fs.readFile(path.join(here, 'vendor', file)));
}
const sources = JSON.parse(await fs.readFile(path.join(here, 'vendor/provenance.json'), 'utf8'));
if (JSON.stringify(vendor) !== JSON.stringify(sources.sha256)) throw Error('vendored xterm digest mismatch');
const core = JSON.parse(execFileSync('cargo', ['run', '--quiet', '-p', 'bboxy-firehose', '--example', 'screen_dump'], {cwd: root, maxBuffer: 8 * 1024 * 1024}));
const names = execFileSync(binary, ['--list'], {encoding: 'utf8'}).trim().split('\n').map(line => line.split('\t')[0]);
await fs.mkdir(out, {recursive: true});
const profile = await fs.mkdtemp(path.join(here, '.chrome-'));
const browser = spawn(chrome, ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`,
  '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--hide-scrollbars',
  '--allow-file-access-from-files', 'about:blank'], {stdio: ['ignore', 'ignore', 'pipe']});
let stderr = '';
browser.stderr.on('data', b => { stderr = (stderr + b).slice(-4000); });
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
class CDP {
  constructor(ws) {
    this.ws = ws; this.id = 0; this.pending = new Map();
    ws.addEventListener('message', event => {
      const m = JSON.parse(event.data), pending = this.pending.get(m.id);
      if (!pending) return;
      this.pending.delete(m.id); clearTimeout(pending.timer);
      m.error ? pending.reject(Error(JSON.stringify(m.error))) : pending.resolve(m.result);
    });
  }
  call(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      const timer = setTimeout(() => { this.pending.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 15000);
      this.pending.set(id, {resolve, reject, timer});
      this.ws.send(JSON.stringify({id, method, params}));
    });
  }
  async eval(expression) {
    const result = await this.call('Runtime.evaluate', {expression, awaitPromise: true, returnByValue: true});
    if (result.exceptionDetails) throw Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  }
}
let ws;
const results = [];
try {
  let port;
  for (let i = 0; i < 100; i++) {
    try {
      const candidate = Number((await fs.readFile(path.join(profile, 'DevToolsActivePort'), 'utf8')).split('\n')[0]);
      if (Number.isInteger(candidate) && candidate > 0 && candidate <= 65535) { port = candidate; break; }
    } catch {}
    await pause(50);
  }
  if (!port) throw Error(`Chrome did not start: ${stderr}`);
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  ws = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => { ws.addEventListener('open', resolve, {once: true}); ws.addEventListener('error', reject, {once: true}); });
  const cdp = new CDP(ws);
  await cdp.call('Page.enable');
  await cdp.call('Runtime.enable');
  await cdp.call('Emulation.setDeviceMetricsOverride', {width: 1000, height: 700, deviceScaleFactor: scale, mobile: false});
  await cdp.call('Network.enable');
  // Local file reference only, so no request can leak data to a remote origin.
  await cdp.call('Network.setBlockedURLs', {urls: ['http://*', 'https://*']});
  await cdp.call('Page.navigate', {url: pathToFileURL(path.join(here, 'index.html')).href});
  let ready = false;
  for (let i = 0; i < 100; i++) {
    ready = await cdp.eval('typeof openFixture === "function" && typeof Terminal === "function"');
    if (ready) break;
    await pause(25);
  }
  if (!ready) throw Error('local xterm reference did not load');
  const capture = async (name, grid) => {
    const shot = await cdp.call('Page.captureScreenshot', {format: 'png', captureBeyondViewport: true,
      clip: {x: 0, y: 0, width: grid.width, height: grid.height, scale: 1}});
    const bytes = Buffer.from(shot.data, 'base64'), filename = `${name}-${variant}.png`;
    await fs.writeFile(path.join(out, filename), bytes);
    return {file: filename, sha256: sha256(bytes), bytes: bytes.length};
  };
  for (const name of names) {
    const bytes = execFileSync(binary, ['--scene', name, '--seed', seed, '--rate', 'max']);
    await cdp.call('Emulation.setFocusEmulationEnabled', {enabled: name.startsWith('cursor-')});
    await cdp.eval(`openFixture(${light},${fixedGrid},${name.startsWith('cursor-')})`);
    const endAlt = name === 'alternate' ? bytes.lastIndexOf(Buffer.from('\x1b[?1049l')) : -1;
    const halfProgress = name === 'progress' ? bytes.indexOf(Buffer.from(' 50%')) + 4 : -1;
    const split = endAlt > 0 ? endAlt : halfProgress > 4 ? halfProgress : -1;
    const intermediate = [];
    if (split > 0) {
      await cdp.eval(`feedFixture(${JSON.stringify(bytes.subarray(0, split).toString('base64'))})`);
      intermediate.push(await capture(`${name}-intermediate`, await cdp.eval('fixtureGrid()')));
      await cdp.eval(`feedFixture(${JSON.stringify(bytes.subarray(split).toString('base64'))})`);
    } else {
      await cdp.eval(`feedFixture(${JSON.stringify(bytes.toString('base64'))})`);
    }
    const grid = await cdp.eval('fixtureGrid()');
    const expected = core.find(scene => scene.name === name), differences = [];
    for (let i = 0; i < expected.cells.length; i++) {
      if (JSON.stringify(expected.cells[i]) !== JSON.stringify(grid.cells[i])) {
        differences.push({row: Math.floor(i / 80), col: i % 80, core: expected.cells[i], xterm: grid.cells[i]});
      }
    }
    const cursorMatch = JSON.stringify(expected.cursor) === JSON.stringify(grid.cursor);
    const cursorStyleMatch = expected.cursorStyle === grid.cursorStyle;
    const steadyCursorMatch = !name.startsWith('cursor-') || (expected.cursorBlink === false && grid.cursorBlink === false);
    const cursorFocusMatch = grid.focused === name.startsWith('cursor-');
    const result = {name, seed, inputBytes: bytes.length, inputSha256: sha256(bytes),
      ...(await capture(name, grid)), intermediate, cursor: grid.cursor, cursorMatch,
      cursorStyle: grid.cursorStyle, cursorStyleMatch, steadyCursorMatch,
      focused: grid.focused, cursorFocusMatch,
      differingCells: differences.length, firstDifferences: differences.slice(0, 12),
      buffer: grid.buffer, lines: grid.lines};
    results.push(result);
    console.log(`${name}: captured; core/xterm differing cells=${differences.length}, cursor=${cursorMatch && cursorStyleMatch && steadyCursorMatch ? 'match' : 'DIFFERENT'}`);
  }
  const report = {xterm: '6.0.0', unicode: '11 (addon-unicode11 0.9.0)', vendor, browser: version.Browser, seed, font: 'Menlo', fontSize: 13,
    cols: 80, rows: 24, scale, theme: light ? 'light' : 'dark', profile: 'isolated ephemeral',
    reference: 'xterm.js DOM renderer; iTerm2/Ghostty comparison remains unproven',
    semanticSchema: '[chars,width,fg,bg,bold,dim,italic,inverse,underline,strike]',
    semanticExactScenes: results.filter(r => r.differingCells === 0 && r.cursorMatch && r.cursorStyleMatch && r.steadyCursorMatch).length,
    captureReadyScenes: results.filter(r => r.cursorFocusMatch).length,
    scenes: results};
  await fs.writeFile(path.join(out, `report-${variant}.json`), JSON.stringify(report, null, 2) + '\n');
  console.log(`${results.length}/${names.length} scenes captured; ${report.semanticExactScenes}/${names.length} semantic viewport matches; ${report.captureReadyScenes}/${names.length} focus states match; variant=${variant}`);
  if (process.argv.includes('--strict') && (report.semanticExactScenes !== names.length || report.captureReadyScenes !== names.length)) process.exitCode = 1;
} finally {
  ws?.close(); browser.kill('SIGTERM');
  await Promise.race([new Promise(resolve => browser.once('exit', resolve)), pause(2000)]);
  if (browser.exitCode === null) browser.kill('SIGKILL');
  await fs.rm(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 100});
}
