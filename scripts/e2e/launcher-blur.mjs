// Independent web filter over the app's actual pre-launcher bitmap. Own Chrome profile only.
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {fileURLToPath, pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {waitForDevToolsPort} from '../../crates/bboxybook/capture-startup.mjs';

const inputPath = path.resolve(process.argv[2]);
const input = JSON.parse(await fs.readFile(inputPath, 'utf8'));
const output = path.dirname(inputPath);
const page = fileURLToPath(new URL('../../lab/web/launcher.html', import.meta.url));
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const before = await fs.readFile(input.before);
const after = await fs.readFile(input.after);
for (const png of [before, after]) {
  if (png.subarray(0, 8).toString('hex') !== '89504e470d0a1a0a') throw Error('capture is not PNG');
}
const [width, height] = [before.readUInt32BE(16), before.readUInt32BE(20)];
if (width !== after.readUInt32BE(16) || height !== after.readUInt32BE(20)) throw Error('capture sizes differ');
const dpr = width / input.size[0];
if (Math.abs(height / input.size[1] - dpr) > .001) throw Error('capture has inconsistent device scale');
const candidates = process.env.CHROME ? [process.env.CHROME] : [
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  '/usr/bin/google-chrome', '/usr/bin/chromium', '/usr/bin/chromium-browser',
];
let executable;
for (const name of candidates) { try { await fs.access(name); executable = name; break; } catch {} }
if (!executable) throw Error('Chrome is required for the independent approved blur reference');
const profile = await fs.mkdtemp(path.join(os.tmpdir(), 'l1-chrome-'));
const browser = spawn(executable, ['--headless=new', '--remote-debugging-port=0',
  `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check',
  '--disable-extensions', '--disable-background-networking', '--disable-sync',
  '--password-store=basic', '--use-mock-keychain', '--hide-scrollbars',
  '--enable-unsafe-swiftshader', 'about:blank'], {stdio: ['ignore', 'ignore', 'pipe']});
let stderr = '', ws;
browser.stderr.on('data', value => { stderr = (stderr + value).slice(-8000); });
process.once('SIGTERM', () => { browser.kill('SIGTERM'); process.exitCode = 143; });
await fs.writeFile(path.join(output, 'l1-browser.json'), JSON.stringify({pid: browser.pid, profile}) + '\n');

class CDP {
  constructor(socket) {
    this.socket = socket; this.id = 0; this.pending = new Map();
    socket.addEventListener('message', event => {
      const message = JSON.parse(event.data), item = this.pending.get(message.id);
      if (!item) return;
      this.pending.delete(message.id); clearTimeout(item.timer);
      message.error ? item.reject(Error(JSON.stringify(message.error))) : item.resolve(message.result);
    });
  }
  call(method, params = {}) {
    return new Promise((resolve, reject) => {
      const id = ++this.id;
      const timer = setTimeout(() => { this.pending.delete(id); reject(Error(`CDP timeout: ${method}`)); }, 15000);
      this.pending.set(id, {resolve, reject, timer});
      this.socket.send(JSON.stringify({id, method, params}));
    });
  }
  async eval(expression) {
    const reply = await this.call('Runtime.evaluate', {expression, awaitPromise: true, returnByValue: true});
    if (reply.exceptionDetails) throw Error(reply.exceptionDetails.exception?.description || reply.exceptionDetails.text);
    return reply.result.value;
  }
}

try {
  const port = await waitForDevToolsPort(profile, () => stderr, 30000);
  const version = await (await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  ws = new WebSocket(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, {once: true}); ws.addEventListener('error', reject, {once: true});
  });
  const cdp = new CDP(ws);
  await cdp.call('Page.enable'); await cdp.call('Runtime.enable');
  await cdp.call('Emulation.setDeviceMetricsOverride', {
    width: Math.round(input.size[0]), height: Math.round(input.size[1]), deviceScaleFactor: dpr, mobile: false,
  });
  const url = pathToFileURL(page);
  url.searchParams.set('state', 'root'); url.searchParams.set('theme', input.theme); url.searchParams.set('density', input.density);
  await cdp.call('Page.navigate', {url: url.href});
  let ready = false;
  const deadline = performance.now() + 10000;
  while (performance.now() < deadline) {
    ready = await cdp.eval(`location.href===${JSON.stringify(url.href)} && document.readyState==='complete' && !!window.Lab && !!document.querySelector('.spot-back')`);
    if (ready) break;
    await pause(25);
  }
  if (!ready) throw Error('approved launcher lab did not become ready');
  const material = await cdp.eval(`(async()=>{
    Lab.applyTokens(); await document.fonts.ready;
    const image=new Image();image.src='data:image/png;base64,${before.toString('base64')}';await image.decode();
    image.style.cssText='position:fixed;inset:0;width:100%;height:100%;';
    const back=document.createElement('div');back.className='spot-back';
    document.body.replaceChildren(image,back);document.body.style.overflow='hidden';
    await new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)));
    const style=getComputedStyle(back);
    return {background:style.backgroundColor,filter:style.backdropFilter,material:STYLE.material};
  })()`);
  if (!material.filter.includes('blur(14px)')) throw Error(`unexpected approved filter: ${material.filter}`);
  const expected = await cdp.call('Page.captureScreenshot', {format: 'png', captureBeyondViewport: false});
  await fs.writeFile(path.join(output, 'l1-web-blur.png'), Buffer.from(expected.data, 'base64'));
  await cdp.eval(`document.querySelector('.spot-back').style.backdropFilter='none';new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))`);
  const negative = await cdp.call('Page.captureScreenshot', {format: 'png', captureBeyondViewport: false});
  await fs.writeFile(path.join(output, 'l1-web-no-blur.png'), Buffer.from(negative.data, 'base64'));
  await cdp.eval(`document.querySelector('.spot-back').style.backdropFilter='blur(${2 * material.material.backdrop_blur_px}px) saturate(1.15)';new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r)))`);
  const excessive = await cdp.call('Page.captureScreenshot', {format: 'png', captureBeyondViewport: false});
  await fs.writeFile(path.join(output, 'l1-web-double-blur.png'), Buffer.from(excessive.data, 'base64'));
  const result = await cdp.eval(`(async()=>{
    const buffers=await Promise.all(${JSON.stringify([after.toString('base64'), expected.data, negative.data, excessive.data])}.map(async encoded=>{
      const image=new Image();image.src='data:image/png;base64,'+encoded;await image.decode();
      if(image.width!==${width}||image.height!==${height})throw Error('reference pixel dimensions differ');
      const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
      const context=canvas.getContext('2d');context.drawImage(image,0,0);return context.getImageData(0,0,image.width,image.height).data;
    }));
    const viewport=${JSON.stringify(input.size)},picker=${JSON.stringify(input.picker)},scale=${dpr};
    const counts=[0,0,0];let pixels=0;
    // 3 blur radii inside the window, outside a 140px guard for the panel shadow.
    // Geometry comes from the actual produced frame, never hand-picked image crops.
    for(let y=0;y<${height};y++)for(let x=0;x<${width};x++){
      const lx=(x+.5)/scale,ly=(y+.5)/scale;
      if(lx<42||ly<42||lx>=viewport[0]-42||ly>=viewport[1]-42)continue;
      if(lx>=picker[0]-140&&lx<=picker[0]+picker[2]+140&&ly>=picker[1]-140&&ly<=picker[1]+picker[3]+140)continue;
      pixels++;const at=(y*${width}+x)*4;
      for(let variant=0;variant<3;variant++){
        let changed=false;for(let channel=0;channel<4;channel++)changed ||= Math.abs(buffers[0][at+channel]-buffers[variant+1][at+channel])>8;
        if(changed)counts[variant]++;
      }
    }
    if(!pixels)throw Error('no observable backdrop pixels outside the panel/shadow');
    return {pixels,changed:counts[0],changed_fraction:counts[0]/pixels,no_blur_changed:counts[1],no_blur_fraction:counts[1]/pixels,
      double_blur_changed:counts[2],double_blur_fraction:counts[2]/pixels,
      sample_region:{window_interior:[42,42,viewport[0]-84,viewport[1]-84],
        excludes_picker_shadow:[picker[0]-140,picker[1]-140,picker[2]+280,picker[3]+280],
        rule:'pixel centre inside window_interior and outside excludes_picker_shadow'},
      channel_tolerance:8,changed_budget:.001,approved_blur_pass:counts[0]/pixels<=.001,
      no_blur_rejected:counts[1]/pixels>.001,double_blur_rejected:counts[2]/pixels>.001};
  })()`);
  const hashes = {};
  for (const name of ['launcher.html', 'lab.css', 'lab.js', 'tokens.js']) {
    hashes[name] = createHash('sha256').update(await fs.readFile(path.join(path.dirname(page), name))).digest('hex');
  }
  const report = {...result, material, reference: page, hashes, browser: version.Browser, device_scale_factor: dpr};
  await fs.writeFile(path.join(output, 'l1-blur-report.json'), JSON.stringify(report, null, 2) + '\n');
  console.log(JSON.stringify(report));
  if (!result.approved_blur_pass || !result.no_blur_rejected || !result.double_blur_rejected) process.exitCode = 1;
} finally {
  if (ws) ws.close();
  browser.kill('SIGTERM');
  await Promise.race([new Promise(resolve => browser.once('exit', resolve)), pause(2000)]);
  if (browser.exitCode === null && browser.signalCode === null) browser.kill('SIGKILL');
  await fs.rm(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 100});
}
