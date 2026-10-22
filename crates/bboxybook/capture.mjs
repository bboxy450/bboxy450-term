// Isolated headless Chrome profile only. Never attaches to the operator's browser.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {spawn,execFileSync} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import vm from 'node:vm';
const root=process.cwd(),out=path.resolve(process.argv[2]||'target/parity');
const manifest=JSON.parse(await fs.readFile(process.argv[3]||path.join(out,'requested.json'),'utf8'));
// Baseline components use the exact review tag. New built design entries are
// supplemental HEAD previews and are identified separately in capture.json.
const head=execFileSync('git',['rev-parse',process.env.BBOXYBOOK_REFERENCE_HEAD||'HEAD'],{encoding:'utf8'}).trim();
const baseline=path.join(out,'reference-v2'),supplemental=path.join(out,'reference-head');
for(const [rev,destRoot] of [['ux-prototype-v2',baseline],[head,supplemental]]){
  for(const file of execFileSync('git',['ls-tree','-r','--name-only',rev,'--','lab/web'],{encoding:'utf8'}).trim().split('\n')){
    if(!file.startsWith('lab/web/')||file.includes('..'))throw Error('invalid reference path');
    const dest=path.join(destRoot,file);await fs.mkdir(path.dirname(dest),{recursive:true});await fs.writeFile(dest,execFileSync('git',['show',`${rev}:${file}`],{maxBuffer:16*1024*1024}));
  }
}
// fixtures.js is data-only JS. Evaluate the frozen, committed file in an empty
// context and export JSON; no secret/user/browser state enters a fixture.
const fixtureSource=await fs.readFile(path.join(supplemental,'lab/web/fixtures.js'),'utf8');
const fixtureContext={window:{}};vm.runInNewContext(fixtureSource,fixtureContext,{timeout:1000});
const sharedFixtures=JSON.parse(JSON.stringify(fixtureContext.window.FIX));
await fs.writeFile(path.join(out,'fixtures.json'),JSON.stringify(sharedFixtures,null,2));
const candidates=process.env.CHROME?[process.env.CHROME]:['/Applications/Google Chrome.app/Contents/MacOS/Google Chrome','/usr/bin/google-chrome','/usr/bin/chromium','/usr/bin/chromium-browser'];
let binary;for(const p of candidates){try{await fs.access(p);binary=p;break;}catch{}}
await fs.mkdir(out,{recursive:true});
if(!binary){
  let matched=0;const cases=[];const tracked=new Set(execFileSync('git',['ls-tree','-r','--name-only','HEAD','--','crates/bboxybook/references'],{encoding:'utf8'}).trim().split('\n'));
  for(const original of manifest){const c={...original};const relative=`${c.component}/${c.slug}-${c.density}-${c.theme}.png`;const file=path.join(root,'crates/bboxybook/references',relative);
    try{const object='crates/bboxybook/references/'+relative;if(!tracked.has(object))throw Error('no committed reference');const png=execFileSync('git',['show','HEAD:'+object],{maxBuffer:64*1024*1024});if(png.subarray(1,4).toString()!=='PNG')throw Error('invalid reference PNG');c.width=png.readUInt32BE(16);c.height=png.readUInt32BE(20);const dest=path.join(out,'web',relative);await fs.mkdir(path.dirname(dest),{recursive:true});await fs.writeFile(dest,png);matched++;}catch{}
    cases.push(c);
  }
  await fs.writeFile(path.join(out,'capture.json'),JSON.stringify({status:matched===manifest.length?'committed-references':matched?'partial':'unproven',reason:'local Chromium absent',matched,missing:manifest.length-matched,cases:manifest.length},null,2));
  await fs.writeFile(path.join(out,'cases.json'),JSON.stringify(cases,null,2));console.log(`Chromium unavailable; ${matched} committed references, ${manifest.length-matched} missing.`);process.exit(0);
}
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
  const captureInputs=await fs.readFile(path.join(root,'crates/bboxybook/capture-inputs.js'),'utf8');
  for(let i=0;i<manifest.length;i++){
    const c={...manifest[i]};const seatRowLayer=c.fixture?.layer==='seat-row';const atomLayer=c.fixture?.layer==='atoms';const componentLayer=c.fixture?.layer==='components';const surfaceLayer=c.fixture?.layer==='floating-surfaces';const surfaceKind=c.fixture?.surface_kind;const relative=`${c.component}/${c.slug}-${c.density}-${c.theme}.png`;cdp.errors=[];
    try{
      await cdp.call('Emulation.setDeviceMetricsOverride',{width:c.component.startsWith('showcase')?1440:1280,height:c.component.startsWith('showcase')?868:960,deviceScaleFactor:1,mobile:false});
      await cdp.call('Emulation.setEmulatedMedia',{features:[{name:'prefers-color-scheme',value:c.theme}]});
      const page=c.page==='@component'?'seat-row.html':c.page;const reference=(seatRowLayer&&c.fixture?.reference!=='baseline')||surfaceLayer||componentLayer||atomLayer||['materials','style'].includes(c.component)||c.page==='@component'||(c.component==='settings_tab'&&c.state==='new harness')?supplemental:baseline;const url=new URL(pathToFileURL(path.join(reference,'lab/web',page.split('?')[0])));if(page.includes('?'))url.search=page.split('?')[1];
      url.searchParams.set('state',c.page==='@component'?'idle':c.state);url.searchParams.set('density',c.density);url.searchParams.set('theme',c.theme);
      const init=await cdp.call('Page.addScriptToEvaluateOnNewDocument',{source:`window.bboxySeatRowFixture=${JSON.stringify(seatRowLayer)};window.bboxySurfaceFixture=${JSON.stringify(surfaceLayer?surfaceKind:null)};window.bboxyComponentFixture=${JSON.stringify(componentLayer)};window.bboxyAtomFixture=${JSON.stringify(atomLayer)};window.bboxyLayerFixture=${JSON.stringify(['materials','style'].includes(c.component))};window.FIX=${JSON.stringify(sharedFixtures)};${captureInputs}\ntry{localStorage.setItem('lab.theme',${JSON.stringify(c.theme)});localStorage.setItem('lab.density',${JSON.stringify(c.density)});localStorage.setItem('showcase.theme',${JSON.stringify(c.theme)});}catch{}`});
      await cdp.call('Page.navigate',{url:url.href});await cdp.call('Page.removeScriptToEvaluateOnNewDocument',{identifier:init.identifier});
      let ready=false;for(let j=0;j<80;j++){try{ready=await cdp.eval(`document.readyState==='complete' && !!window.C && !!document.querySelector(${JSON.stringify(c.selector)})`);}catch{}if(ready)break;await pause(25);}
      if(!ready)throw Error('lab page never became ready');
      if(c.page!=='@component'&&c.selector!=='body'){const states=await cdp.eval(`Array.from(document.querySelectorAll('.state-strip button'),n=>n.textContent)`);if(!states.includes(c.state))throw Error('reference page does not support state: '+c.state);}
      await cdp.eval(`Lab.prefs.theme=${JSON.stringify(c.theme)};Lab.prefs.density=${JSON.stringify(c.density)};Lab.applyTokens();document.documentElement.style.setProperty('scroll-behavior','auto');`);
      if(c.page==='@component'){await cdp.eval(`(${adapter})(${JSON.stringify(c)})`);}
      if(c.checker)await cdp.eval(`document.querySelector('.card')?.classList.add('checker')`);
      await cdp.eval(`document.fonts.ready`);
      if(['materials','style'].includes(c.component))await cdp.eval(`bboxyFreezeLayer()`);
      if(seatRowLayer)await cdp.eval(`(()=>{document.getAnimations().forEach(a=>{a.pause();a.effect.updateTiming({delay:0});a.currentTime=0});const s=document.createElement('style');s.textContent='.row-menu{visibility:hidden!important}.seat-row{animation-delay:0ms!important}';document.head.append(s);})()`);
      if(atomLayer)await cdp.eval(`document.getAnimations().forEach(a=>{a.pause();a.currentTime=0})`);
      if(surfaceLayer)await cdp.eval(`document.getAnimations().forEach(a=>{a.pause();a.currentTime=1000})`);
      await cdp.eval(`(()=>{const s=document.createElement('style');s.textContent='*,*::before,*::after{animation-play-state:paused!important;transition:none!important;caret-color:transparent!important}';document.head.append(s);})()`);
      const rect=await cdp.eval(`(()=>{const n=document.querySelector(${JSON.stringify(c.selector)});const r=n.getBoundingClientRect();return {x:r.x+scrollX,y:r.y+scrollY,width:Math.ceil(r.width),height:Math.ceil(r.height)};})()`);
      if(!rect.width||!rect.height||rect.width>8192||rect.height>8192)throw Error(`unsupported capture bounds ${JSON.stringify(rect)}`);
      if(cdp.errors.length)throw Error(`script exceptions: ${JSON.stringify(cdp.errors)}`);
      c.fixture=await cdp.eval(`bboxyCaptureInputs(${JSON.stringify(c.selector)})`);
      if(surfaceLayer){
        const context=await cdp.call('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{...rect,scale:1}});
        const file=path.join(out,'context',relative);await fs.mkdir(path.dirname(file),{recursive:true});await fs.writeFile(file,Buffer.from(context.data,'base64'));
        await cdp.eval(`(()=>{const s=document.createElement('style');s.textContent='[data-bboxy-surface-panel] > *{visibility:hidden!important}';document.head.append(s);})()`);
      }
      const shot=await cdp.call('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{...rect,scale:1}});
      const dest=path.join(out,'web',relative);await fs.mkdir(path.dirname(dest),{recursive:true});await fs.writeFile(dest,Buffer.from(shot.data,'base64'));
      if(seatRowLayer||surfaceLayer||componentLayer||atomLayer||['materials','style'].includes(c.component)){
        const full=await cdp.eval(`({width:Math.ceil(document.documentElement.clientWidth),height:Math.ceil(Math.max(document.documentElement.scrollHeight,innerHeight))})`);
        await cdp.eval(`(()=>{const s=document.createElement('style');s.textContent=${JSON.stringify(seatRowLayer?'[data-bboxy-seat-row]{visibility:hidden!important}':surfaceLayer?'[data-bboxy-surface-host]{visibility:hidden!important}':componentLayer?'[data-bboxy-capture-demo]{visibility:hidden!important}':atomLayer?'.glyph,.hicon{visibility:hidden!important}':'.ui-sq,.ui-box,.ui-scope .pane-wrap{visibility:hidden!important}')};document.head.append(s);})()`);
        const backdrop=await cdp.call('Page.captureScreenshot',{format:'png',captureBeyondViewport:true,clip:{x:0,y:0,...full,scale:1}});
        const file=path.join(out,'backdrops',relative);await fs.mkdir(path.dirname(file),{recursive:true});await fs.writeFile(file,Buffer.from(backdrop.data,'base64'));
        c.fixture.backdrop={path:file,...full,crop:rect};
      }
      c.width=rect.width;c.height=rect.height;captured.push(c);
    }catch(error){failures.push({case:relative,error:String(error)});captured.push(c);console.error(relative,String(error));}
    if((i+1)%100===0)console.log(`captured ${i+1}/${manifest.length}`);
  }
  const hashes={};for(const name of ['components.js','lab.css','lab.js','tokens.js','showcase.js','showcase.html','fixtures.js','ui.js','materials.html','style.html','components.html','launcher.html','documents.html'])hashes[name]=createHash('sha256').update(await fs.readFile(path.join(supplemental,'lab/web',name))).digest('hex');
  await fs.writeFile(path.join(out,'cases.json'),JSON.stringify(captured,null,2));
  await fs.writeFile(path.join(out,'capture.json'),JSON.stringify({status:failures.length?'partial':'captured',baseline:execFileSync('git',['rev-parse','ux-prototype-v2^{commit}'],{encoding:'utf8'}).trim(),supplementalReference:'Layer fixtures, @component entries and settings_tab/new harness use frozen committed HEAD lab components',revision:head,browser:version.Browser,profile:'isolated ephemeral profile',deviceScaleFactor:1,floatingSurfaceScope:manifest.some(c=>c.fixture?.layer==='floating-surfaces')?'Container surfaces only: module children hidden, full context retained in context/, external content slot height recorded':null,freeze:'CSS animation playback paused; floating surfaces fixed at final animation state; module content hidden only for explicitly marked surface pairs; L2/L3 canvas rAF clock fixed at 450ms with seeded counter-hash Math.random; L5 glyph CSS pulse fixed at 0ms; no transcript normalization; original web content retained',supplementalWebHashes:hashes,cases:manifest.length,failures},null,2));
  console.log(`web: ${captured.length-failures.length}/${manifest.length} captured; ${failures.length} failures`);
}finally{if(ws)ws.close();browser.kill('SIGTERM');await Promise.race([new Promise(r=>browser.once('exit',r)),pause(2000)]);if(browser.exitCode===null)browser.kill('SIGKILL');await fs.rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:100});}
if(failures.length)process.exitCode=1;
