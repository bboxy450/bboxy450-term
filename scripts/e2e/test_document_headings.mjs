// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxybook/capture.mjs's isolated headless Chrome/CDP fixture.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import {spawn} from 'node:child_process';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {waitForDevToolsPort} from '../../crates/bboxybook/capture-startup.mjs';

const root=process.cwd(),out=path.resolve(process.argv[2]);
await fs.mkdir(out,{recursive:true});
const candidates=process.env.CHROME?[process.env.CHROME]:['/Applications/Google Chrome.app/Contents/MacOS/Google Chrome','/usr/bin/google-chrome','/usr/bin/chromium'];
let binary;
for(const candidate of candidates){try{await fs.access(candidate);binary=candidate;break;}catch{}}
assert(binary,'private headless Chrome is required');
const profile=await fs.mkdtemp(path.join(os.tmpdir(),'lumen-document-heading-'));
const browser=spawn(binary,['--headless=new','--disable-gpu','--no-first-run','--no-default-browser-check','--disable-extensions',`--user-data-dir=${profile}`,'--remote-debugging-port=0','about:blank'],{stdio:['ignore','ignore','pipe']});
let stderr='',socket;
browser.stderr.on('data',data=>{stderr=(stderr+data).slice(-8000);});
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
try{
  const port=await waitForDevToolsPort(profile,()=>stderr,15_000);
  const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  socket=new WebSocket(tabs.find(tab=>tab.type==='page').webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{socket.addEventListener('open',resolve,{once:true});socket.addEventListener('error',reject,{once:true});});
  let next=0;const pending=new Map();
  socket.onmessage=event=>{const response=JSON.parse(event.data),entry=pending.get(response.id);if(entry){pending.delete(response.id);clearTimeout(entry.timer);response.error?entry.reject(Error(JSON.stringify(response.error))):entry.resolve(response.result);}};
  const call=(method,params={})=>new Promise((resolve,reject)=>{const id=++next,timer=setTimeout(()=>{pending.delete(id);reject(Error(`CDP timeout: ${method}`));},15_000);pending.set(id,{resolve,reject,timer});socket.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const value=await call('Runtime.evaluate',{expression,returnByValue:true,awaitPromise:true});assert(!value.exceptionDetails,JSON.stringify(value.exceptionDetails));return value.result.value;};
  await call('Page.enable');await call('Runtime.enable');
  await call('Page.navigate',{url:pathToFileURL(path.join(root,'lab/web/documents.html')).href+'?state=spec'});
  const deadline=performance.now()+10_000;
  while(!await evaluate('document.readyState==="complete" && !!window.C && !!window.Lab')){
    assert(performance.now()<deadline,'reference document did not load');await pause(25);
  }
  await evaluate('document.fonts.ready.then(()=>true)');
  const style=JSON.parse(await fs.readFile(path.join(root,'design/style.json'),'utf8')).type;
  const tokens=['doc_title','heading','subtitle','doc_heading','body','section'];
  const expected=tokens.map(token=>style[token]),checks=[];
  for(const width of [640,1280]){
    await call('Emulation.setDeviceMetricsOverride',{width,height:900,deviceScaleFactor:1,mobile:false});
    const sample=await evaluate(`(()=>{
      document.getElementById('heading-probe')?.remove();
      const stage=document.createElement('div');stage.id='heading-probe';
      stage.append(C.mdLite([1,2,3,4,5,6].map(n=>'#'.repeat(n)+' Heading'+n).join('\\n')));
      document.body.append(stage);
      const heads=Array.from(stage.querySelector('.md').children,n=>{const s=getComputedStyle(n),r=n.getBoundingClientRect();return {tag:n.tagName,text:n.textContent,font:parseFloat(s.fontSize),weight:s.fontWeight,line:parseFloat(s.lineHeight),height:r.height};});
      const title=C.bodyAfterTitle('# Same title\\n\\n## Keep second','Same title');
      const different=C.bodyAfterTitle('# Different title\\n\\n## Keep second','Same title');
      const fence=C.mdLite('\\\`\\\`\\\`text\\n# Inside fence\\n\\\`\\\`\\\`\\n###### Outside fence');
      return {heads,title,different,fenceTags:Array.from(fence.children,n=>n.tagName),fenceText:fence.querySelector('pre')?.textContent};
    })()`);
    assert.deepEqual(sample.heads.map(head=>head.tag),['H1','H2','H3','H4','H5','H6']);
    assert.deepEqual(sample.heads.map(head=>head.font),expected);
    assert(sample.heads.every(head=>head.weight==='700'&&head.line>=head.font&&head.height>=head.font));
    assert(!sample.title.includes('# Same title')&&sample.title.includes('## Keep second'));
    assert(sample.different.includes('# Different title'));
    assert.deepEqual(sample.fenceTags,['PRE','H6']);assert.equal(sample.fenceText,'# Inside fence');
    checks.push({width,...sample});
  }
  const hashes={};for(const name of ['lab/web/lab.css','lab/web/components.js','design/style.json'])hashes[name]=createHash('sha256').update(await fs.readFile(path.join(root,name))).digest('hex');
  await fs.writeFile(path.join(out,'verdict.json'),JSON.stringify({pass:true,checks,source_sha256:hashes},null,2)+'\n');
  console.log(JSON.stringify({pass:true,widths:checks.map(check=>check.width),native_style_tokens:tokens,expected,evidence:out}));
}finally{
  socket?.close();const gone=new Promise(resolve=>browser.once('exit',resolve));browser.kill();await Promise.race([gone,pause(3000)]);
  await fs.rm(profile,{recursive:true,force:true,maxRetries:3,retryDelay:100});
}
