// Launch-defaults reference flow in an owned headless browser/profile only.
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {spawn} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {waitForDevToolsPort} from './capture-startup.mjs';
const out=path.resolve(process.argv[2]||'target/launch-preview');
await fs.mkdir(out,{recursive:true});
const profile=await fs.mkdtemp(path.join(os.tmpdir(),'bboxy-launch-preview-'));
const chrome=process.env.CHROME||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const child=spawn(chrome,['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check','--disable-extensions','--hide-scrollbars','about:blank'],{stdio:['ignore','ignore','pipe']});
let stderr='',ws; child.stderr.on('data',b=>stderr=(stderr+b).slice(-8000));
try {
 const port=await waitForDevToolsPort(profile,()=>stderr);
 const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
 ws=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);
 await new Promise((resolve,reject)=>{ws.addEventListener('open',resolve,{once:true});ws.addEventListener('error',reject,{once:true});});
 let id=0;const pending=new Map();
 ws.addEventListener('message',ev=>{const m=JSON.parse(ev.data);if(m.id){const p=pending.get(m.id);if(p){pending.delete(m.id);clearTimeout(p.timer);m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result);}}});
 const call=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;const timer=setTimeout(()=>{pending.delete(n);reject(Error(method+' timeout'));},15000);pending.set(n,{resolve,reject,timer});ws.send(JSON.stringify({id:n,method,params}));});
 const evaluate=async expression=>{const result=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(result.exceptionDetails)throw Error(result.exceptionDetails.exception?.description||result.exceptionDetails.text);return result.result.value;};
 await call('Page.enable'); await call('Emulation.setDeviceMetricsOverride',{width:1246,height:900,deviceScaleFactor:1,mobile:false});
 await call('Page.navigate',{url:pathToFileURL(path.resolve('lab/web/showcase.html')).href});
 await evaluate(`new Promise((resolve,reject)=>{let n=0;const probe=()=>document.querySelector('.pane')?resolve():++n>200?reject(Error('preview did not load')):setTimeout(probe,25);probe();})`);
 const state=await evaluate(`(() => {
 const key=(target,key,meta=false)=>target.dispatchEvent(new KeyboardEvent('keydown',{key,metaKey:meta,bubbles:true}));
 document.querySelector('.pane').click(); key(window,'d',true);
 const input=document.querySelector('.spot-back input'); if(!input)throw Error('Cmd+D did not open');
 input.value='claude';input.dispatchEvent(new Event('input',{bubbles:true}));key(input,'ArrowRight');key(input,'ArrowRight');
 const architect=[...document.querySelectorAll('.spot-back .opt')].find(r=>r.querySelector('.lbl')?.textContent==='Architect');
 if(!architect)throw Error('Role lacks Architect');architect.click();
 const rows=[...document.querySelectorAll('.spot-back .opt')].map(r=>r.textContent);
 if(!rows[0].includes('architect'))throw Error('Architect was not taken');
 return {rows,runs:document.querySelector('.cmdline').textContent};})()`);
 const png=await call('Page.captureScreenshot',{format:'png'});await fs.writeFile(path.join(out,'parameters.png'),Buffer.from(png.data,'base64'));
 await fs.writeFile(path.join(out,'parameters.json'),JSON.stringify(state,null,2)+'\n');
 await evaluate(`document.querySelector('.spot-back input').dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));if(document.querySelector('.spot-back'))throw Error('Enter did not launch');`);
 console.log(JSON.stringify({flow:'Cmd+D > claude > Right > Right > Architect > Enter',...state}));
} finally {
 if(ws)ws.close();child.kill('SIGTERM');
 await Promise.race([new Promise(r=>child.once('exit',r)),new Promise(r=>setTimeout(r,2000))]);
 if(child.exitCode===null)child.kill('SIGKILL');
 await fs.rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:100});
}
