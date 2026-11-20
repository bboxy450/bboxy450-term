// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Adapted from bboxy450-term crates/bboxybook/capture.mjs: isolated CDP transport
// and exact-child custody. https://github.com/bboxy450/bboxy450-term
// Changes: trusted local SVGs, shared font bytes, bounded cameras and DOM geometry.
import fs from 'node:fs/promises';
import path from 'node:path';
import {spawn} from 'node:child_process';
import {createHash} from 'node:crypto';
import {waitForDevToolsPort} from '../bboxybook/capture-startup.mjs';

const request=JSON.parse(await fs.readFile(process.argv[2],'utf8'));
const out=path.resolve(request.out),fontDir=path.resolve(request.font_directory);
await fs.mkdir(out,{recursive:true});
const hash=b=>createHash('sha256').update(b).digest('hex');
const captureSourceSha=hash(await fs.readFile(new URL(import.meta.url)));
const manifestBytes=await fs.readFile(path.join(fontDir,'manifest.json'));
const fontSet='sha256:'+hash(manifestBytes),fonts=JSON.parse(manifestBytes),faces=[];
for(const f of fonts.faces){const bytes=await fs.readFile(path.join(fontDir,f.file));if(hash(bytes)!==f.sha256)throw Error('font byte mismatch');faces.push({...f,data:bytes.toString('base64')});}
const profile=await fs.mkdtemp(path.join(out,'profile-'));
const binary=process.env.CHROME||'/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const browser=spawn(binary,['--headless=new','--remote-debugging-port=0',`--user-data-dir=${profile}`,'--no-first-run','--no-default-browser-check','--disable-extensions','--hide-scrollbars','--force-color-profile=srgb','--disable-background-networking','about:blank'],{stdio:['ignore','ignore','pipe']});
await fs.writeFile(path.join(out,'owned-process.json'),JSON.stringify({pid:browser.pid,profile,binary}));
let stderr='',ws,exited=false;
browser.stderr.on('data',b=>{stderr=(stderr+b).slice(-4000);});
const exitPromise=new Promise(resolve=>browser.once('exit',()=>{exited=true;resolve();}));
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
const runs=[],fileHashes={};
try{
  const port=await waitForDevToolsPort(profile,()=>stderr);
  const version=await(await fetch(`http://127.0.0.1:${port}/json/version`)).json();
  const tabs=await(await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  ws=new WebSocket(tabs.find(t=>t.type==='page').webSocketDebuggerUrl);
  await new Promise((resolve,reject)=>{ws.addEventListener('open',resolve,{once:true});ws.addEventListener('error',reject,{once:true});});
  let next=0;const pending=new Map(),exceptions=[];
  ws.addEventListener('message',event=>{const m=JSON.parse(event.data);if(m.id){const p=pending.get(m.id);if(p){pending.delete(m.id);clearTimeout(p.timer);m.error?p.reject(Error(JSON.stringify(m.error))):p.resolve(m.result);}}else if(m.method==='Runtime.exceptionThrown')exceptions.push(m.params);});
  const call=(method,params={})=>new Promise((resolve,reject)=>{const id=++next,timer=setTimeout(()=>{pending.delete(id);reject(Error(`CDP timeout ${method}`));},15000);pending.set(id,{resolve,reject,timer});ws.send(JSON.stringify({id,method,params}));});
  const evaluate=async expression=>{const r=await call('Runtime.evaluate',{expression,awaitPromise:true,returnByValue:true});if(r.exceptionDetails)throw Error(JSON.stringify(r.exceptionDetails));return r.result.value;};
  for(const domain of ['Page','Runtime','DOM','CSS','Network'])await call(domain+'.enable');
  await call('Network.setBlockedURLs',{urls:['http://*','https://*','file://*']});
  const frame=(await call('Page.getFrameTree')).frameTree.frame.id;
  for(const c of request.cases){
    const source=await fs.readFile(c.svg,'utf8');
    if(hash(Buffer.from(source))!==c.sha256)throw Error('source changed '+c.id);
    for(const view of c.views){
      const [x,y,w,h]=view.camera,s=view.scale;
      if(!(w>0&&h>0&&s>0&&w*s<=4096&&h*s<=4096&&w*h*s*s<=16777216))throw Error('capture bound');
      await call('Emulation.setDeviceMetricsOverride',{width:Math.ceil(w),height:Math.ceil(h),deviceScaleFactor:s,mobile:false});
      await call('Page.setDocumentContent',{frameId:frame,html:'<!doctype html><html><head><style>html,body{margin:0;background:white;overflow:hidden}svg{position:absolute;overflow:visible}[data-bboxy-hidden]{visibility:hidden!important}</style></head><body></body></html>'});
      await evaluate(`(async()=>{const faces=${JSON.stringify(faces)},family=${JSON.stringify(fonts.family)};for(const f of faces){const face=new FontFace(family,'url(data:font/ttf;base64,'+f.data+')',{weight:String(f.weight),style:'normal'});await face.load();document.fonts.add(face);}await document.fonts.ready;document.body.insertAdjacentHTML('beforeend',${JSON.stringify(source.replace(/<\?xml[\s\S]*?\?>|<!DOCTYPE[\s\S]*?>/g,''))});const svg=document.querySelector('svg');svg.style.width=${c.viewBox[2]}+'px';svg.style.height=${c.viewBox[3]}+'px';svg.style.left='0';svg.style.top='0';svg.style.transformOrigin='0 0';const intrinsic=svg.getScreenCTM();if(intrinsic.b||intrinsic.c||!(intrinsic.a>0&&intrinsic.d>0))throw Error('unexpected root transform');svg.style.transform='matrix('+[1/intrinsic.a,0,0,1/intrinsic.d,-intrinsic.e/intrinsic.a+${-x},-intrinsic.f/intrinsic.d+${-y}].join(',')+')';await document.fonts.ready;if(svg.querySelector('text')&&svg.getAttribute('data-bboxy-font-set')!==${JSON.stringify(fontSet)})throw Error('font-set mismatch');})()`);
      await evaluate(`(async()=>{document.querySelector('svg').getBBox();await document.fonts.ready;await new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve)));})()`);
      const geometry=await evaluate(`(()=>{
        const scale=${s},svg=document.querySelector('svg');
        const loc=n=>{let p=[];while(n&&n!==svg){const tag=n.localName;const i=Array.from(n.parentElement.children).filter(c=>c.localName===tag).indexOf(n)+1;p.unshift(tag+'['+i+']');n=n.parentElement;}return '/svg[1]/'+p.join('/');};
        const point=(p,m)=>{const q=new DOMPoint(p.x,p.y).matrixTransform(m);return [q.x*scale,q.y*scale];};
        const box=(b,m)=>{const ps=[{x:b.x,y:b.y},{x:b.x+b.width,y:b.y},{x:b.x,y:b.y+b.height},{x:b.x+b.width,y:b.y+b.height}].map(p=>point(p,m));const xs=ps.map(p=>p[0]),ys=ps.map(p=>p[1]);return [Math.min(...xs),Math.min(...ys),Math.max(...xs)-Math.min(...xs),Math.max(...ys)-Math.min(...ys)];};
        const texts=Array.from(svg.querySelectorAll('text,tspan')).filter(n=>n.getNumberOfChars()>0&&!n.querySelector('tspan')).map(n=>{const m=n.getScreenCTM(),b=n.getBBox(),chars=[];for(let i=0;i<n.getNumberOfChars();i++)chars.push({start:point(n.getStartPositionOfChar(i),m),end:point(n.getEndPositionOfChar(i),m),box:box(n.getExtentOfChar(i),m),rotation:n.getRotationOfChar(i)});const style=getComputedStyle(n),anchor={x:n.x.baseVal.numberOfItems?n.x.baseVal.getItem(0).value:0,y:n.y.baseVal.numberOfItems?n.y.baseVal.getItem(0).value:0},transforms=[];for(let parent=n;parent;parent=parent.parentElement){transforms.unshift(parent.getAttribute('transform')||'');if(parent===svg)break;}return {attributes:Object.fromEntries(Array.from(n.attributes,a=>[a.name,a.value])),transforms,local_anchor:[anchor.x,anchor.y],baseline_anchor:point(anchor,m),anchor_mode:style.textAnchor,advance_axis_scale:Math.hypot(m.a,m.b)*scale,source:loc(n),text:n.textContent,bounds:box(b,m),local_bounds:[b.x,b.y,b.width,b.height],matrix:[m.a,m.b,m.c,m.d,m.e,m.f],advance:Math.hypot(chars.at(-1).end[0]-chars[0].start[0],chars.at(-1).end[1]-chars[0].start[1]),local_advance:n.getComputedTextLength(),font:style.fontFamily,weight:style.fontWeight,font_size:style.fontSize,chars};});
        const shapes=Array.from(svg.querySelectorAll('path,rect,line,circle,ellipse,polyline,polygon,use,image')).filter(n=>!n.closest('defs')&&getComputedStyle(n).display!=='none').map(n=>{const m=n.getScreenCTM(),b=n.getBBox(),target=n.localName==='use'?svg.querySelector(n.getAttribute('href')||n.getAttributeNS('http://www.w3.org/1999/xlink','href')):n;let endpoints=null;if(typeof target?.getTotalLength==='function'){const offset=p=>({x:p.x+(n.localName==='use'?n.x.baseVal.value:0),y:p.y+(n.localName==='use'?n.y.baseVal.value:0)});endpoints=[point(offset(target.getPointAtLength(0)),m),point(offset(target.getPointAtLength(target.getTotalLength())),m)];}return {source:loc(n),tag:n.localName,id:n.id,bounds:box(b,m),matrix:[m.a,m.b,m.c,m.d,m.e,m.f],endpoints};});
        const rm=svg.getScreenCTM();return {texts,shapes,root_matrix:[rm.a,rm.b,rm.c,rm.d,rm.e,rm.f],font_ready:document.fonts.status,device_scale:scale,viewport:[innerWidth,innerHeight]};
      })()`);
      const platformFonts=[],platformFontNodes=[];
      const doc=await call('DOM.getDocument');const nodes=await call('DOM.querySelectorAll',{nodeId:doc.root.nodeId,selector:'svg text:not(:has(tspan)),svg tspan:not(:has(tspan))'});
      if(nodes.nodeIds.length||c.exporter){
        for(const [index,nodeId] of nodes.nodeIds.entries()){const f=await call('CSS.getPlatformFontsForNode',{nodeId}),expected=fonts.faces.find(face=>String(face.weight)===geometry.texts[index]?.weight);platformFontNodes.push({nodeId,fonts:f.fonts,expected_postscript_name:expected?.postscript_name||null});platformFonts.push(...f.fonts);}
        if(!platformFonts.length||platformFontNodes.some(n=>!n.fonts.length||!n.fonts.some(f=>f.glyphCount>0)||n.fonts.some(f=>f.postScriptName!==n.expected_postscript_name))||!platformFonts.some(f=>f.glyphCount>0)||platformFonts.some(f=>!f.isCustomFont||!fonts.faces.some(face=>face.postscript_name===f.postScriptName))){await fs.writeFile(path.join(out,'font-refusal.json'),JSON.stringify({case:c.id,view,source_sha256:c.sha256,geometry,platformFonts,platformFontNodes,diagnostics:await evaluate(`({faces:Array.from(document.fonts,f=>({family:f.family,weight:f.weight,status:f.status})),unquoted:CSS.supports('font-family',${JSON.stringify(fonts.family)}),quoted:CSS.supports('font-family',JSON.stringify(${JSON.stringify(fonts.family)})),text:Array.from(document.querySelectorAll('svg text'),n=>({attribute:n.getAttribute('font-family'),computed:getComputedStyle(n).fontFamily,weight:getComputedStyle(n).fontWeight}))})`)},null,2));throw Error('missing or unapproved platform face '+JSON.stringify(platformFonts));}
      }
      const prefix=path.join(out,c.id+'-'+view.id),captures={};
      const layer=async mode=>{await evaluate(`(()=>{const svg=document.querySelector('svg');svg.querySelectorAll('[data-bboxy-hidden]').forEach(n=>n.removeAttribute('data-bboxy-hidden'));const mode=${JSON.stringify(mode)};for(const n of svg.querySelectorAll('text,path,rect,line,circle,ellipse,polyline,polygon,use,image')){if(n.closest('defs'))continue;if((mode==='shapes'&&['text','image'].includes(n.localName))||(mode==='image'&&n.localName!=='image')||(mode==='without-image'&&n.localName==='image')||(mode==='without-gradient'&&getComputedStyle(n).fill.includes('url(')))n.setAttribute('data-bboxy-hidden','');}})()`);};
      const shot=async name=>{const data=await call('Page.captureScreenshot',{format:'png',captureBeyondViewport:false,clip:{x:0,y:0,width:w,height:h,scale:1}});const file=prefix+'-'+name+'.png';await fs.writeFile(file,Buffer.from(data.data,'base64'));captures[name]={file,sha256:hash(Buffer.from(data.data,'base64'))};};
      await shot('complete');await layer('shapes');await shot('shapes');
      if(source.includes('<linearGradient')){await layer('without-gradient');await shot('without-gradient');}
      if(source.includes('<image')){await layer('without-image');await shot('without-image');}
      if(source.includes('<image')){await layer('image');await shot('image-auto');await evaluate(`document.querySelectorAll('svg image').forEach(n=>n.style.imageRendering='pixelated')`);await shot('image-nearest');}
      if(exceptions.length)throw Error('reference script exception '+JSON.stringify(exceptions));
      const result={id:c.id,view,source_sha256:c.sha256,font_set:fontSet,browser:version.Browser,geometry,platformFonts,platformFontNodes,captures};
      const geometryBytes=JSON.stringify(result,null,2);await fs.writeFile(prefix+'-geometry.json',geometryBytes);fileHashes[prefix+'-geometry.json']=hash(Buffer.from(geometryBytes));for(const capture of Object.values(captures))fileHashes[capture.file]=capture.sha256;runs.push(result);
    }
  }
  if(hash(await fs.readFile(new URL(import.meta.url)))!==captureSourceSha)throw Error('capture program changed during run');
  await fs.writeFile(path.join(out,'chrome.json'),JSON.stringify({browser:version,source_sha256:captureSourceSha,font_set:fontSet,file_hashes:fileHashes,runs},null,2));
  console.log(JSON.stringify({chrome:version.Browser,runs:runs.length,font_set:fontSet}));
}finally{
  if(ws)ws.close();
  if(!exited)browser.kill('SIGTERM');
  await Promise.race([exitPromise,pause(2000)]);
  if(!exited){browser.kill('SIGKILL');await Promise.race([exitPromise,pause(10000)]);}
  if(!exited)throw Error('owned Chrome not reaped; retained its profile');
  await fs.rm(profile,{recursive:true,force:true,maxRetries:5,retryDelay:100});
  await fs.writeFile(path.join(out,'owned-process.json'),JSON.stringify({pid:browser.pid,reaped:true,profile_removed:true,binary}));
}
