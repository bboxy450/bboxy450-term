// Isolated headless Chrome profile only. Never attaches to the operator's browser.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {spawn,execFileSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const root=process.cwd(),out=path.resolve(process.argv[2]||'target/parity');
const manifest=JSON.parse(await fs.readFile(process.argv[3]||path.join(out,'requested.json'),'utf8'));
const candidates=[process.env.CHROME,'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome','/usr/bin/google-chrome','/usr/bin/chromium','/usr/bin/chromium-browser'].filter(Boolean);
let binary;for(const p of candidates){try{await fs.access(p);binary=p;break;}catch{}}
await fs.mkdir(out,{recursive:true});
if(!binary){await fs.writeFile(path.join(out,'capture.json'),JSON.stringify({status:'unproven',reason:'local Chromium absent; no matching committed matrix references',cases:manifest.length},null,2));await fs.writeFile(path.join(out,'cases.json'),JSON.stringify(manifest,null,2));console.log('Chromium unavailable; parity will mark missing references.');process.exit(0);}
const profile=await fs.mkdtemp(path.join(os.tmpdir(),'bboxybook-chrome-'));
const browser=spawn(binary,['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check','--disable-extensions','--hide-scrollbars','--enable-unsafe-swiftshader','about:blank'],{stdio:['ignore','ignore','pipe']});
let stderr='';browser.stderr.on('data',b=>{stderr=(stderr+b).slice(-8000);});
const pause=ms=>new Promise(r=>setTimeout(r,ms));
class CDP{
  constructor(ws){this.ws=ws;this.id=0;this.pending=new Map();this.errors=[];ws.addEventListener('message',ev=>{const m=JSON.parse(ev.data);if(m.id){const p=this.pending.get(m.id);if(p){this.pending.delete(m.id);clearTimeout(p.timer);m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result);}}else if(m.method==='Runtime.exceptionThrown'){this.errors.push(m.params.exceptionDetails);}});}
  call(method,params={}){return new Promise((resolve,reject)=>{const id=++this.id;const timer=setTimeout(()=>{this.pending.delete(id);reject(Error(`CDP timeout: ${method}`));},15000);this.pending.set(id,{resolve,reject,timer});this.ws.send(JSON.stringify({id,method,params}));});}
  async eval(expression){const r=await this.call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw Error(r.exceptionDetails.text+': '+(r.exceptionDetails.exception?.description||''));return r.result.value;}
}
let ws;const failures=[];const captured=[];
try{
  let port;for(let i=0;i<120;i++){try{port=Number((await fs.readFile(path.join(profile,'DevToolsActivePort'),'utf8')).split('\n')[0]);break;}catch{await pause(50);}}
  if(!port)throw Error(`Chrome did not start: ${stderr}`);
  const version=await(await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  ws=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);await new Promise((r,j)=>{ws.addEventListener('open',r,{once:true});ws.addEventListener('error',j,{once:true});});
  const cdp=new CDP(ws);await cdp.call('Page.enable');await cdp.call('Runtime.enable');
  const adapter=await fs.readFile(path.join(root,'crates/bboxybook/web-fixtures.js'),'utf8');
  for(let i=0;i<manifest.length;i++){
    const c={...manifest[i]};const relative=`${c.component}/${c.slug}-${c.density}-${c.theme}.png`;cdp.errors=[];
    try{
      await cdp.call('Emulation.setDeviceMetricsOverride',{width:c.component.startsWith('showcase')?1440:1280,height:960,deviceScaleFactor:1,mobile:false});
      await cdp.call('Emulation.setEmulatedMedia',{features:[{name:'prefers-color-scheme',value:c.theme}]});
      const page=c.page==='@component'?'seat-row.html':c.page;const url=new URL(pathToFileURL(path.join(root,'lab/web',page.split('?')[0])));if(page.includes('?'))url.search=page.split('?')[1];
      url.searchParams.set('state',c.page==='@component'?'idle':c.state);url.searchParams.set('density',c.density);url.searchParams.set('theme',c.theme);
      await cdp.call('Page.navigate',{url:url.href});
      let ready=false;for(let j=0;j<80;j++){try{ready=await cdp.eval(`document.readyState==='complete' && !!window.C && !!document.querySelector(${JSON.stringify(c.selector)})`);}catch{}if(ready)break;await pause(25);}
      if(!ready)throw Error('lab page never became ready');
      await cdp.eval(`Lab.prefs.theme=${JSON.stringify(c.theme)};Lab.prefs.density=${JSON.stringify(c.density)};Lab.applyTokens();document.documentElement.style.setProperty('scroll-behavior','auto');`);
      if(c.page==='@component'){await cdp.eval(`(${adapter})(${JSON.stringify(c)})`);}
      await cdp.eval(`document.fonts.ready`);
      await cdp.eval(`(()=>{const s=document.createElement('style');s.textContent='*,*::before,*::after{animation-play-state:paused!important;transition:none!important;caret-color:transparent!important}';document.head.append(s);})()`);
      const rect=await cdp.eval(`(()=>{const n=document.querySelector(${JSON.stringify(c.selector)});const r=n.getBoundingClientRect();return {x:r.x+scrollX,y:r.y+scrollY,width:Math.ceil(r.width),height:Math.ceil(r.height)};})()`);
      if(!rect.width||!rect.height||rect.width>8192||rect.height>8192)throw Error(`unsupported capture bounds ${JSON.stringify(rect)}`);
      if(cdp.errors.length)throw Error(`script exceptions: ${JSON.stringify(cdp.errors)}`);
      const shot=await cdp.call('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{...rect,scale:1}});
      const dest=path.join(out,'web',relative);await fs.mkdir(path.dirname(dest),{recursive:true});await fs.writeFile(dest,Buffer.from(shot.data,'base64'));
      c.width=rect.width;c.height=rect.height;captured.push(c);
    }catch(error){failures.push({case:relative,error:String(error)});captured.push(c);console.error(relative,String(error));}
    if((i+1)%100===0)console.log(`captured ${i+1}/${manifest.length}`);
  }
  const hashes={};for(const name of ['components.js','lab.css','lab.js','tokens.js','showcase.js','showcase.html'])hashes[name]=createHash('sha256').update(await fs.readFile(path.join(root,'lab/web',name))).digest('hex');
  await fs.writeFile(path.join(out,'cases.json'),JSON.stringify(captured,null,2));
  await fs.writeFile(path.join(out,'capture.json'),JSON.stringify({status:failures.length?'partial':'captured',revision:execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim(),browser:version.Browser,profile:'isolated ephemeral profile',deviceScaleFactor:1,freeze:'CSS animation playback paused; no transcript normalization; original web page content retained',webHashes:hashes,cases:manifest.length,failures},null,2));
  console.log(`web: ${captured.length-failures.length}/${manifest.length} captured; ${failures.length} failures`);
}finally{if(ws)ws.close();browser.kill('SIGTERM');await Promise.race([new Promise(r=>browser.once('exit',r)),pause(2000)]);if(browser.exitCode===null)browser.kill('SIGKILL');await fs.rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:100});}
if(failures.length)process.exitCode=1;
