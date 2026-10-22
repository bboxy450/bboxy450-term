// Capture component inputs, not DOM internals. Both renderers receive the same
// data and outer fixture rectangles; native components still lay out/draw their
// own contents. Keep references until capture so live seat state is current.
(() => {
  const calls = [];
  const materials = [];
  const styles = [];
  const glows = [];
  const uiNodes = new WeakMap();
  if (window.bboxyLayerFixture) {
    let randomIndex=12345;
    const random=()=>{let x=randomIndex++>>>0;x=Math.imul(x^(x>>>16),0x7feb352d);x=Math.imul(x^(x>>>15),0x846ca68b);x=(x^(x>>>16))>>>0;return (x>>>8)/16777216;};
    Math.random=random;
    let frames=[],nextFrame=1;
    window.requestAnimationFrame=fn=>{const id=nextFrame++;frames.push({id,fn});return id;};
    window.cancelAnimationFrame=id=>{frames=frames.filter(f=>f.id!==id);};
    window.bboxyFreezeLayer=(timeMs=450)=>{for(const time of [0,0.000001,timeMs]){const ready=frames;frames=[];for(const f of ready)f.fn(time);}};
    let glow;
    Object.defineProperty(window,'Glow',{configurable:true,get:()=>glow,set(value){glow=value;const attach=value.attach;value.attach=function(wrap,seat,opts){const seed=randomIndex;const cv=attach(wrap,seat,opts);glows.push({wrap,cv,seed,activity:seat.state,focused:!!opts?.focused,time_ms:450});return cv;};}});
  }
  if (window.bboxyStreamFixture) {
    const timeout=window.setTimeout.bind(window);
    // The evidence-climbing demo starts at sender-stored; freeze its 700ms timer.
    window.setTimeout=(fn,delay,...args)=>delay===700?0:timeout(fn,delay,...args);
  }
  let ui;
  Object.defineProperty(window,'UI',{configurable:true,get:()=>ui,set(value){
    ui=value;const original=value.material;
    value.material=function(name,options){const result=original(name,options);materials.push({kind:name,params:result.params});return result;};
    for (const name of ['Group','Path','Text','Stack','Spacer','Button','List','Table','Input','Overlay','Modal']) {
      const original=value[name];
      value[name]=function(...args){const node=original(...args);uiNodes.set(node,{kind:name.toLowerCase(),args});return node;};
    }
    const box=value.Box;
    value.Box=function(style,...children){const node=box(style,...children);styles.push({style,node});uiNodes.set(node,{kind:"box",style,children});return node;};
  }});
  let components;
  Object.defineProperty(window, 'C', {
    configurable: true,
    get: () => components,
    set(value) {
      components = value;
      for (const name of ['sidebar', 'pane', 'seatRow', 'tabRow', 'miniGrid',
        'spaceHeader', 'graphPage', 'provenanceCard', 'subscriptionsCard',
      'externalLane', 'epicGroup', 'streamTabs', 'messageRow', 'glyph', 'titleStrip', 'subagentRows', 'taskRow', 'artifactRow', 'titleBar', 'picker', 'settingsPage', 'docView', 'docModal']) {
        const original = value[name];
        if (typeof original !== 'function') continue;
        value[name] = function (...args) {
          const node = original.apply(this, args);
          if(Array.isArray(node))node.forEach((item,index)=>calls.push({name,args,node:item,index}));else calls.push({ name, args, node });
          return node;
        };
      }
    },
  });
  window.bboxyCaptureInputs = selector => {
    const root = document.querySelector(selector);
    const origin = root.getBoundingClientRect();
    if(window.bboxySurfaceFixture){
      const kind=window.bboxySurfaceFixture,panel=root.querySelector(kind==='launcher'?'.spot-back .picker':'.doc-modal'),host=panel.parentElement;
      const rect=n=>{const r=n.getBoundingClientRect();return {x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height};};
      host.dataset.bboxySurfaceHost='';panel.dataset.bboxySurfacePanel='';
      return {version:1,layer:'floating-surfaces',scope:'container surfaces only; module children hidden',floating:{kind,host:rect(host),panel:rect(panel),slot_height:panel.getBoundingClientRect().height,viewport_width:innerWidth}};
    }
    const serialized = calls.filter(c => c.node?.isConnected && root.contains(c.node)).map(c => {
      const r = c.node.getBoundingClientRect();
      return { name: c.name, args: c.args,
        header: c.node.querySelector?.('.sb-top')?.getBoundingClientRect().height || 0,
        rect: { x: r.x - origin.x, y: r.y - origin.y, w: r.width, h: r.height } };
    });
    const color=(value,node)=>{const probe=document.createElement('span');probe.style.color=value;node.append(probe);const c=getComputedStyle(probe).color;probe.remove();const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d');ctx.fillStyle=c;ctx.fillRect(0,0,1,1);return [...ctx.getImageData(0,0,1,1).data].map(c=>c/255);};
    const bounds=node=>{const r=node.getBoundingClientRect();return {x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height};};
    const radii=css=>[css.borderTopLeftRadius,css.borderTopRightRadius,css.borderBottomRightRadius,css.borderBottomLeftRadius].map(parseFloat);
    const textDiag=node=>{
      const result={kind:'text',value:node.innerText,rect:bounds(node)};
      const css=getComputedStyle(node),cv=document.createElement('canvas'),ctx=cv.getContext('2d');ctx.font=css.font;
      const tm=ctx.measureText(node.innerText),marker=document.createElement('span');marker.style.cssText='display:inline-block;width:0;height:0;padding:0;margin:0;border:0;line-height:0';node.prepend(marker);
      result.font_diagnostics={font:css.font,ascent:tm.fontBoundingBoxAscent,descent:tm.fontBoundingBoxDescent,baseline:marker.getBoundingClientRect().y-node.getBoundingClientRect().y,source:'inline-marker'};marker.remove();
      return result;
    };
    const rawTextDiag=node=>{
      const span=document.createElement('span'),parent=node.parentNode;span.style.cssText='font:inherit;line-height:inherit;padding:0;margin:0;border:0;display:inline-block';
      parent.replaceChild(span,node);span.append(node);const result=textDiag(span);parent.replaceChild(node,span);return result;
    };
    const diag=node=>{
      if(node.nodeType===Node.TEXT_NODE)return rawTextDiag(node);
      if(node.classList.contains('ui-text'))return textDiag(node);
      const kind=node.classList.contains('hicon')?'icon':node.classList.contains('ui-spacer')?'spacer':node.classList.contains('ui-badge')?'badge':'box';
      return {kind,rect:bounds(node),children:kind==='badge'?[...node.childNodes].filter(n=>n.nodeType===Node.TEXT_NODE&&n.textContent).map(rawTextDiag):[]};
    };
    const inputDiag=node=>{
      const css=getComputedStyle(node),r=node.getBoundingClientRect(),mirror=document.createElement('div'),span=document.createElement('span');
      mirror.style.cssText='position:fixed;pointer-events:none;opacity:0;box-sizing:border-box;border:0;margin:0;white-space:pre';
      Object.assign(mirror.style,{left:r.x+'px',top:r.y+'px',width:r.width+'px',height:r.height+'px',padding:css.padding,font:css.font,lineHeight:css.lineHeight});
      span.style.cssText='font:inherit;line-height:inherit;display:inline-block';span.textContent=node.value||node.placeholder;mirror.append(span);document.body.append(mirror);
      const child=textDiag(span);child.font_diagnostics.source='input-css-font-probe';mirror.remove();
      return {kind:'control',rect:bounds(node),children:[child]};
    };
    const group=nodes=>{
      const rects=nodes.map(bounds),x=Math.min(...rects.map(r=>r.x)),y=Math.min(...rects.map(r=>r.y));
      return {kind:'stack',rect:{x,y,w:Math.max(...rects.map(r=>r.x+r.w))-x,h:Math.max(...rects.map(r=>r.y+r.h))-y},children:nodes.map(n=>({kind:'box',rect:bounds(n),children:[textDiag(n.querySelector('.ui-text'))]}))};
    };
    const exportUi=node=>{
      const call=uiNodes.get(node);
      if(!call){
        if(node.classList.contains('ui-checker'))return {kind:'mask-backdrop',size:[parseFloat(node.style.width),parseFloat(node.style.height)],rect:bounds(node),children:[...node.children].map(exportUi)};
        if(node.classList.contains('ef-rows'))return {kind:'mask-content',rect:bounds(node),children:[...node.children].map(exportUi)};
        throw Error('unrecorded UI component');
      }
      const result={kind:call.kind,rect:bounds(node)};
      if(call.kind==='text')Object.assign(result,textDiag(node),{value:call.args[0],options:call.args[1]||{}});
      else if(call.kind==='box'){result.style=call.style;result.children=call.children.map(exportUi);}
      else if(call.kind==='group'){result.options=call.args[0]||{};result.children=call.args.slice(1).map(exportUi);}
      else if(call.kind==='stack'){result.options=call.args[0]||{};result.children=call.args.slice(1).map(n=>({...exportUi(n),...(node.closest('.ef-rows')&&n.classList.contains('ui-box')?{no_shrink:true}:{})}));}
      else if(call.kind==='overlay'){result.children=call.args.map(exportUi);}
      else if(call.kind==='modal'){
        result.title=call.args[0];result.body=exportUi(call.args[1]);result.actions=(call.args[2]||[]).map(exportUi);
        const column=node.firstElementChild,actions=column.lastElementChild;
        result.children=[{kind:'stack',rect:bounds(column),children:[textDiag(column.firstElementChild),result.body,{kind:'stack',rect:bounds(actions),children:[diag(actions.firstElementChild),...result.actions]}]}];
      }
      else if(call.kind==='button'){
        result.label=call.args[0];result.options=call.args[1]||{};
        result.children=[...node.childNodes].filter(n=>n.nodeType!==Node.TEXT_NODE||n.textContent).map(diag);
      }else if(call.kind==='list'){
        result.items=call.args[0];result.options=call.args[1]||{};
        result.children=[...node.children].map(row=>({kind:'box',rect:bounds(row),children:[...row.children].map(diag)}));
      }else if(call.kind==='table'){
        result.columns=call.args[0];result.rows=call.args[1];result.options=call.args[2]||{};
        const cols=result.columns.length,cells=[...node.children];result.children=[];
        for(let i=0;i<cells.length;i+=cols)result.children.push(group(cells.slice(i,i+cols)));
      }else if(call.kind==='input'){
        result.options=call.args[0]||{};const panel=node.querySelector('.ui-input');
        result.children=[{kind:'box',rect:bounds(panel),children:[...panel.children].map(n=>n.tagName==='INPUT'?inputDiag(n):diag(n))}];
        for(const child of node.children)if(child!==panel)result.children.push(textDiag(child));
      }
      return result;
    };
    const leaf=n=>n.classList.contains('glyph')||n.classList.contains('hicon')||(!n.children.length&&!n.classList.contains('cell'));
    const parts=(node,path='',out=[])=>{
      const key=path||'row',css=getComputedStyle(node);if(css.display==='none'||(window.bboxyPaneFixture&&node.matches('.reconnect.out')))return out;
      const r={key,rect:bounds(node)};
      if(leaf(node)&&!node.classList.contains('glyph')&&!node.classList.contains('hicon')&&node.textContent){Object.assign(r,{kind:'text',...textDiag(node)});r.key=key;}
      r.css={font:css.font,line_height:css.lineHeight,background:css.backgroundColor,shadow:css.boxShadow,opacity:css.opacity,border_radius:css.borderRadius,flex:css.flex,padding:css.padding};out.push(r);
      if(!leaf(node)&&!(window.bboxyPaneFixture&&node.classList.contains('term'))){const counts={};for(const ch of [...node.childNodes]){
        if(ch.nodeType===Node.TEXT_NODE&&ch.textContent.trim()){out.push({...rawTextDiag(ch),key:(path?path+'/':'')+'text'});continue;}
        if(ch.nodeType!==Node.ELEMENT_NODE)continue;
        const cls=ch.classList[0]||ch.tagName.toLowerCase(),i=counts[cls]||0;counts[cls]=i+1;parts(ch,(path?path+'/':'')+cls+(i?i:''),out);
      }}
      return out;
    };
    const tabParts=node=>{
      const diagnostics=[];
      diagnostics.push({key:'tab',rect:bounds(node)});
      const header=node.querySelector(':scope > .tab-row');
      if(header)parts(header,'header',diagnostics);
      const body=node.querySelector(':scope > .tab-body');
      if(body)diagnostics.push({key:'body',rect:bounds(body)});
      let seat=-1,nestedIndex=0;
      for(const child of [...(body||node).children]){
        if(child===header)continue;
        if(child.classList.contains('seat-row')){seat++;nestedIndex=0;diagnostics.push(...parts(child).map(p=>({...p,key:`seat${seat}/${p.key}`})));}
        else if(seat>=0){diagnostics.push(...parts(child).map(p=>({...p,key:`seat${seat}/child${nestedIndex}/${p.key}`})));nestedIndex++;}
      }
      return diagnostics;
    };
    if(window.bboxyDocumentFixture){
      const call=calls.find(c=>['docView','docModal'].includes(c.name)&&c.node?.isConnected&&root.contains(c.node));
      if(!call)throw Error('Document constructor missing');
      const node=call.node,modal=call.name==='docModal',view=modal?node.querySelector('.doc-view'):node,diagnostics=[];
      const add=(key,n,text=false)=>{if(!n)throw Error('Missing document part '+key);diagnostics.push(text?{...textDiag(n),key}:{key,kind:'box',rect:bounds(n)});};
      add('document',node);if(modal){add('scroller',node.querySelector('.dm-body'));add('footer',node.querySelector('.dm-foot'));[...node.querySelectorAll('.dm-act')].forEach((n,i)=>{add('action'+i,n);add('action'+i+'/label',n.firstChild,true);if(n.lastChild.textContent)add('action'+i+'/key',n.lastChild,true);});}
      add('view',view);add('main',view.querySelector('.doc-main'));add('head',view.querySelector('.doc-head'));add('top',view.querySelector('.doc-top'));add('path',view.querySelector('.doc-path'),true);add('title',view.querySelector('h1'),true);
      for(const [key,sel]of [['type','.doc-type'],['status','.doc-status'],['super','.doc-super'],['broken','.doc-broken'],['frontmatter','.doc-fm'],['body','.md'],['provenance','.doc-prov'],['rail','.doc-rail'],['rail/graph','.nbhd'],['rail/legend','.rail-legend']]){const n=view.querySelector(sel);if(n)add(key,n,key==='type'||key==='status');}
      [...view.querySelectorAll('.doc-fm > .k')].forEach((n,i)=>{add('fm'+i+'/key',n,true);add('fm'+i+'/value',n.nextElementSibling);[...n.nextElementSibling.children].forEach((v,j)=>{if(!v.children.length)add('fm'+i+'/value'+j,v,true);});});
      [...view.querySelectorAll('.md > *')].forEach((n,i)=>{add('block'+i,n,/^H[234]$/.test(n.tagName)||n.tagName==='PRE');if(n.tagName==='UL')[...n.children].forEach((li,j)=>add('block'+i+'/li'+j,li));});
      [...view.querySelectorAll('.md > *')].forEach((n,i)=>{if(!n.classList.contains('dgfig'))return;const k='block'+i;add(k+'/figure',n);for(const [name,sel,tx]of [['head','.dgfig-h',false],['kind','.dgfig-h>.kind',true],['count','.dgfig-h>.cnt',true],['segment','.dgfig-h>.seg',false],['button-drawing','.seg button:first-child',false],['button-source','.seg button:last-child',false],['expand','.exp',false],['drawing','.dgfig-b',false]])add(k+'/'+name,n.querySelector(sel),tx);if(n.querySelector('svg.dg')){add(k+'/canvas',n.querySelector('svg.dg'));[...n.querySelectorAll('.dg-node')].forEach((g,j)=>{add(k+'/node'+j,g.querySelector('rect,path,circle'));const t=g.querySelector('text');if(t){const p=t.getStartPositionOfChar(0).matrixTransform(t.getScreenCTM()),css=getComputedStyle(t),ctx=document.createElement('canvas').getContext('2d');ctx.font=css.fontStyle+' '+css.fontWeight+' '+parseFloat(css.fontSize)+'px '+css.fontFamily;const fm=ctx.measureText(t.textContent),r=bounds(t);diagnostics.push({key:k+'/label'+j,kind:'text',rect:r,font_diagnostics:{baseline:p.y-origin.y-r.y+(fm.fontBoundingBoxAscent-fm.fontBoundingBoxDescent)/2,source:'SVG central anchor; diagnostic only'}})}});}if(n.querySelector('.dgfig-src'))add(k+'/source',n.querySelector('.dgfig-src'));});
      [...view.querySelectorAll('.rail-h')].forEach((n,i)=>add('rail/h'+i,n,true));[...view.querySelectorAll('.rail-back')].forEach((n,i)=>add('rail/back'+i,n));
      const parent=node.parentElement,pb=bounds(parent),pc=getComputedStyle(parent),pad=['Left','Top','Right','Bottom'].map(k=>parseFloat(pc['padding'+k])||0);
      const host=modal?{x:pb.x+pad[0],y:pb.y+pad[1],w:pb.w-pad[0]-pad[2],h:pb.h-pad[1]-pad[3]}:{...bounds(node),h:0};
      const textBounds=[],walk=document.createTreeWalker(node,NodeFilter.SHOW_TEXT);while(walk.nextNode()){const n=walk.currentNode;if(!n.textContent.trim())continue;const r=document.createRange();r.selectNodeContents(n);for(const b of r.getClientRects())if(b.width&&b.height)textBounds.push({x:b.x-origin.x,y:b.y-origin.y,w:b.width,h:b.height});}
      node.dataset.bboxyDocument='';
      const probes=[...node.querySelectorAll('.doc-main,.doc-head,.doc-top,.doc-fm,.doc-prov,.doc-rail,.md,.md>*,.md li,.dm-foot,.dm-act,.wl')].map(n=>{const s=getComputedStyle(n);return {name:n.className||n.tagName,rect:bounds(n),font:s.font,line_height:s.lineHeight,margin:s.margin,padding:s.padding,border:s.border,display:s.display,gap:s.gap,align:s.alignItems,max_width:s.maxWidth};});
      const diagrams=[...(call.args[0].body||'').matchAll(/```mermaid[^\n]*\n([\s\S]*?)\n```/g)].map((match,i)=>{const source=match[1],show_source=[...view.querySelectorAll('.dgfig')][i]?.querySelector('.seg .on')?.textContent==='Source';try{return {source,graph:C.parseMermaid(source),show_source};}catch(error){return {source,error:String(error.message||error)};}});
      const md=view.querySelector('.md'),ms=getComputedStyle(md),cv=document.createElement('canvas'),mc=cv.getContext('2d');mc.font=ms.font;const chProbe=document.createElement('span');chProbe.style.cssText='display:block;width:1ch;height:0';md.append(chProbe);const ch={font:ms.font,zero:mc.measureText('0').width,unit:chProbe.getBoundingClientRect().width,font_kerning:ms.fontKerning,font_optical:ms.fontOpticalSizing};chProbe.remove();
      return JSON.parse(JSON.stringify({version:1,layer:'documents',frames:[{view:{document:call.args[0],modal,seat_name:call.args[1]?.seat?.name||null,viewport:[innerWidth,innerHeight],diagrams},host,parts:diagnostics,textBounds,probes,ch}]}));
    }
    if(window.bboxySettingsFixture){
      const call=calls.find(c=>c.name==='settingsPage'&&c.node?.isConnected&&root.contains(c.node));
      if(!call)throw Error('Settings constructor missing');
      const node=call.node,options=call.args[0]||{},diagnostics=[];
      const add=(key,n,text=false)=>{if(!n)throw Error('Missing Settings part '+key);diagnostics.push(text?{...textDiag(n),key}:{key,kind:'box',rect:bounds(n)});};
      add('settings',node);add('side',node.querySelector('.s-side'));add('main',node.querySelector('.s-main'));
      add('search',node.querySelector('.s-q'));add('source',node.querySelector('.s-src'));diagnostics.push({...rawTextDiag(node.querySelector('.s-src').firstChild),key:'source/text'});
      [...node.querySelectorAll('.s-navi')].forEach((n,i)=>{add('nav'+i,n);add('nav'+i+'/icon',n.firstChild);add('nav'+i+'/text',n.lastChild,true)});
      [...node.querySelectorAll('.s-main .s-card')].forEach((n,i)=>{
        const k='card'+i;add(k,n);
        const head=n.querySelector(':scope>.s-card-h');if(head){add(k+'/head',head);add(k+'/title',head.querySelector('.t'),true)}
        [...n.querySelectorAll('.s-row')].forEach((r,j)=>{const q=k+'/r'+j;add(q,r);add(q+'/label',r.querySelector('.lbl'));const hint=r.querySelector('.lbl small');if(hint)add(q+'/hint',hint,true);add(q+'/ctl',r.querySelector('.ctl'));});
        [...n.querySelectorAll('.fl')].forEach((r,j)=>{const q=k+'/f'+j;add(q,r);add(q+'/label',r.firstChild);const ctrl=r.children[1];if(ctrl)add(q+'/control',ctrl)});
        const band=n.querySelector(':scope>.band');if(band){add(k+'/band',band);add(k+'/name',band.querySelector('.rn'),true)}
        const hh=n.querySelector(':scope>.hh');if(hh){add(k+'/head',hh);if(hh.querySelector('.tile'))add(k+'/tile',hh.querySelector('.tile'));add(k+'/hname',hh.querySelector('.ht b')||hh.querySelector('.ht input'));if(hh.querySelector('.ht code'))add(k+'/command',hh.querySelector('.ht code'),true)}
      });
      for(const [key,sel]of [['account/hero','.hero'],['account/avatar','.hero .logo'],['account/name','.hname'],['account/fp','.fp'],['account/algo','.chips .chip:first-child'],['account/devices','.subkeys'],['account/note','.s-main > .s-note']]){const n=node.querySelector(sel);if(n)add(key,n);}
      const textBounds=[],walk=document.createTreeWalker(node,NodeFilter.SHOW_TEXT);while(walk.nextNode()){const n=walk.currentNode;if(!n.textContent.trim()||n.parentElement.closest('select,option'))continue;const range=document.createRange();range.selectNodeContents(n);for(const r of range.getClientRects())if(r.width&&r.height)textBounds.push({x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height});}
      for(const [key,sel]of [['account/enroll','.sk.add'],['account/enroll/title','.sk.add b'],['account/enroll/hint','.sk.add span'],['account/enroll/caption','.sk.add small']]){const n=node.querySelector(sel);if(n)add(key,n,key!=='account/enroll');}
      const records=v=>({...v,roles:Object.entries(v.roles),harness:Object.entries(v.harness)});
      const view={section:options.section||'Account',draftHarness:options.draftHarness||null,values:records(C.SET),defaults:records(C.SET_DEF),launch:C.LAUNCH.map(l=>({harness:l.harness,label:l.label,tile:l.tile}))};
      node.dataset.bboxySettings='';
      const parent=node.parentElement,pc=getComputedStyle(parent),pr=bounds(parent),bt=parseFloat(pc.borderTopWidth)||0,br=parseFloat(pc.borderRightWidth)||0,bb=parseFloat(pc.borderBottomWidth)||0,bl=parseFloat(pc.borderLeftWidth)||0;const clip={rect:{x:pr.x+bl,y:pr.y+bt,w:pr.w-bl-br,h:pr.h-bt-bb},radii:radii(pc).map(v=>Math.max(0,v-Math.max(bt,br,bb,bl)))};
      return JSON.parse(JSON.stringify({version:1,layer:'settings',frames:[{view,host:bounds(node),clip,textBounds,probes:[...node.querySelectorAll('.mrow,.thumb,.cap,.wo,.wo input,.wo-pct,.ht b,.sk,.fp,.acts,.act,.hero,.subkeys,.s-group,select')].map(n=>{const s=getComputedStyle(n);return {tag:n.className||n.tagName,value:n.textContent,rect:bounds(n),font:s.font,lineHeight:s.lineHeight,padding:s.padding,width:s.width,height:s.height,display:s.display,alignItems:s.alignItems}}),parts:diagnostics}]}));
    }
    if(window.bboxyGraphFixture){
      const graphCall=calls.find(c=>c.name==='graphPage'&&c.node?.isConnected&&root.contains(c.node));
      const standalone=!graphCall&&calls.find(c=>['provenanceCard','subscriptionsCard'].includes(c.name)&&c.node?.isConnected&&root.contains(c.node));
      if(!graphCall&&!standalone)throw Error('Graph constructor missing');
      const node=(graphCall||standalone).node,view=graphCall?graphCall.args[0]:standalone.name==='provenanceCard'?{provenance:standalone.args[0],only:['provenance']}:{subscriptions:standalone.args[0],only:['subscriptions']};
      view.harnesses=C.LAUNCH.filter(l=>l.harness).map(l=>({name:l.harness,tile:l.tile}));
      const diagnostics=[],add=(key,n,isText=false)=>{if(!n)throw Error('Missing graph diagnostic '+key);diagnostics.push(isText?{...textDiag(n),key}:{key,kind:'box',rect:bounds(n)});};
      if(graphCall)add('graph',node);
      const cards=graphCall?[...node.children]:[node];
      for(const card of cards){
        const head=card.querySelector('.s-card-h'),title=head.firstElementChild.textContent;
        const k=title.startsWith('Agents')?'agents':title.startsWith('Graph')?'work':title.startsWith('Subscriptions')?'subscriptions':title.startsWith('Channels')?'channels':title.startsWith('Provenance')?'provenance':'history';
        add(k,card);add(k+'/head',head);add(k+'/title',head.firstElementChild,true);
        if(k==='agents'){
          const rows=[...card.querySelectorAll('.g-row')];
          rows.forEach((r,i)=>{const p=i?'agents/r'+(i-1):'agents/header';add(p,r);[...r.children].forEach((cell,j)=>{if(!i)add(p+'/c'+j,cell,true);else if(j===0){add(p+'/seat',cell);add(p+'/tile',cell.querySelector('.tile'));add(p+'/name',cell.querySelector('b'),true);add(p+'/glyph',cell.querySelector('.glyph'));}else add(p+'/'+['seat','role','now','blocked'][j],cell,true);});});
        }else if(k==='work'){
          const tree=card.querySelector('.g-tree');add('work/tree',tree);let at=0;const rows=[...tree.children];
          (view.epics||[]).forEach((ep,ei)=>{let p='work/e'+ei,r=rows[at++];add(p,r);['kind','title','count'].forEach((v,j)=>add(p+'/'+v,r.children[j],true));
            (ep.items||[]).forEach((it,wi)=>{const p='work/e'+ei+'/i'+wi,r=rows[at++];add(p,r);for(const [cls,name]of [['g-branch','branch'],['tglyph','mark'],['ekind','kind'],['g-title','title'],['eass','assignee'],['g-dep','waits']]){const n=r.querySelector('.'+cls);if(n)add(p+'/'+name,n,!n.querySelector('.hicon'));}const inv=r.querySelector('.invchip');if(inv)add(p+'/invariant',inv);
              for(let bi=0;bi<(it.binds||[]).length;bi++){const b=rows[at++],q=p+'/b'+bi;add(q,b);['branch','mark','title','rel'].forEach((v,j)=>add(q+'/'+v,b.children[j],!b.children[j].querySelector('.hicon')));}
              for(let ai=0;ai<(it.artifacts||[]).length;ai++){const a=rows[at++],q=p+'/a'+ai;add(q,a);['branch','mark','kind','ref','responsible'].forEach((v,j)=>{if(a.children[j])add(q+'/'+v,a.children[j],true)});}
            });
          });
        }else if(k==='subscriptions'){
          [...card.querySelectorAll('.sub-row')].forEach((r,i)=>{const p=k+'/r'+i;add(p,r);add(p+'/seat',r.firstElementChild,true);add(p+'/patterns',r.lastElementChild);[...r.querySelectorAll('.sub-pat')].forEach((pt,j)=>{const q=p+'/p'+j;add(q,pt);add(q+'/pattern',pt.querySelector('code'),true);if(pt.querySelector('i'))add(q+'/auto',pt.querySelector('i'),true)});});
        }else if(k==='channels'){
          add(k+'/wrap',card.querySelector('.cg-wrap'));add(k+'/tree',card.querySelector('.chgraph'));add(k+'/key',card.querySelector('.cg-key'));
          const svgText=(key,n)=>{if(!n)throw Error('missing channel label '+key);const r=bounds(n),matrix=n.getScreenCTM(),p=n.getStartPositionOfChar(0).matrixTransform(matrix),css=getComputedStyle(n),ctx=document.createElement('canvas').getContext('2d');ctx.font=css.fontStyle+' '+css.fontWeight+' '+(parseFloat(css.fontSize)*Math.hypot(matrix.a,matrix.b))+'px '+css.fontFamily;const fm=ctx.measureText(n.textContent);diagnostics.push({key,kind:'text',value:n.textContent,rect:r,font_diagnostics:{baseline:p.y-origin.y-r.y+(fm.fontBoundingBoxAscent-fm.fontBoundingBoxDescent)/2,source:'SVG central anchor + device-space Canvas font metrics; diagnostic only'}});};
          [...card.querySelectorAll('.cg-folder')].forEach((n,i)=>svgText(k+'/folder'+i,n));
          let fi=0;(view.channels.folders||[]).forEach((f,i)=>{if(f.filter)svgText(k+'/filter'+i,card.querySelectorAll('.cg-note')[fi++]);});
          [...card.querySelectorAll('.cg-sink')].forEach((n,i)=>svgText(k+'/sink'+i+'/name',n));
          [...card.querySelectorAll('.cg-path')].forEach((n,i)=>svgText(k+'/sink'+i+'/path',n));
          const labels=[...card.querySelectorAll('.cg-pill-t')];let at=0;
          (view.channels.links||[]).forEach((l,i)=>{if(l.filter||l.rate)svgText(k+'/pill'+i+'/text',labels[at++]);});
          (view.channels.bridges||[]).forEach((br,i)=>svgText(k+'/bridge'+i+'/text',labels[at++]));
        }else if(k==='history'){
          [...card.querySelectorAll('.g-hist')].forEach((r,i)=>{const p=k+'/r'+i;add(p,r);['time','seat','event','mark','ref','grade'].forEach((v,j)=>add(p+'/'+v,r.children[j],true));});
        }else if(k==='provenance'){
          [...card.querySelectorAll('.p-item')].forEach((r,i)=>{const p=k+'/i'+i;add(p,r);add(p+'/head',r.querySelector('.p-head'));add(p+'/kind',r.querySelector('.ekind'),true);add(p+'/title',r.querySelector('b'),true);add(p+'/by',r.querySelector('.p-by'),true);
            const der=r.querySelector('.p-derived');if(der){add(p+'/derived',der);add(p+'/derived/label',der.firstElementChild,true);add(p+'/links',der.lastElementChild);[...der.querySelectorAll('.wl')].forEach((link,j)=>{const q=p+'/links/l'+j;add(q,link);add(q+'/slug',link.firstElementChild);if(link.lastElementChild!==link.firstElementChild)add(q+'/rel',link.lastElementChild);});}
            const dig=r.querySelector('.p-digest');if(dig){add(p+'/digest',dig);['label','content','signature'].forEach((v,j)=>add(p+'/digest/'+v,dig.children[j],true));}
          });
          const loghead=card.querySelector('.p-loghead');if(loghead){add(k+'/loghead',loghead);/* text range separate from its padded row */}
          [...card.querySelectorAll('.p-log')].forEach((r,i)=>{const p=k+'/log'+i;add(p,r);['hash','time','text'].forEach((v,j)=>add(p+'/'+v,r.children[j],true));});
        }
      }
      node.dataset.bboxyGraph='';
      const parent=node.parentElement,pc=getComputedStyle(parent),externalClip=graphCall&&['hidden','auto','scroll'].includes(pc.overflowY)?(()=>{
        const r=bounds(parent),t=parseFloat(pc.borderTopWidth)||0,b=parseFloat(pc.borderBottomWidth)||0,l=parseFloat(pc.borderLeftWidth)||0,rr=parseFloat(pc.borderRightWidth)||0;
        return {rect:{x:r.x+l,y:r.y+t,w:r.w-l-rr,h:r.h-t-b},radii:radii(pc).map(v=>Math.max(0,v-Math.max(t,l,b,rr)))};
      })():null;
      return JSON.parse(JSON.stringify({version:1,layer:'graphs',frames:[{view,standalone:!!standalone,host:bounds(node),clip:externalClip,parts:diagnostics}]},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    if(window.bboxyStreamFixture){
      const frames=[...root.querySelectorAll('.stream,.stream-dock')].filter(n=>!n.parentElement.closest('.stream,.stream-dock')).map(node=>{
        const dock=node.classList.contains('stream-dock'),framed=!dock||!node.parentElement.matches('.body'),frame=dock&&framed?node.parentElement:node;
        const list=calls.filter(c=>['messageRow','epicGroup','externalLane','streamTabs'].includes(c.name)&&c.node?.isConnected&&node.contains(c.node));
        const entries=list.map(c=>{
          if(c.name==='messageRow')return {node:c.node,kind:'message',data:c.args[0],show_channel:c.args[1]?.showCh};
          if(c.name==='epicGroup')return {node:c.node,kind:'epic',data:c.args[0]};
          if(c.name==='externalLane')return {node:c.node,kind:'external',events:c.args[0],open:c.args[1]?.open!==false};
          return {node:c.node,kind:'tabs',channels:c.args[0],selected:c.args[1]?.sel||'all'};
        });
        for(const n of node.querySelectorAll('.filter'))entries.push({node:n,kind:'filter',name:n.querySelector('b').textContent});
        for(const n of node.querySelectorAll('.lab-note')){const css=getComputedStyle(n);entries.push({node:n,kind:'note',text:n.textContent,padding:[css.paddingTop,css.paddingRight,css.paddingBottom,css.paddingLeft].map(parseFloat)});}
        entries.sort((a,b)=>a.node.compareDocumentPosition(b.node)&Node.DOCUMENT_POSITION_FOLLOWING?-1:1);
        const diagnostics=[{key:'frame',rect:bounds(frame)}];
        if(dock)diagnostics.push({key:'dock',rect:bounds(node)});
        for(let i=0;i<entries.length;i++){
          const entry=entries[i],key='e'+i;
          const all=parts(entry.node,key);
          // Audit lines use one styled inline text flow, not positioned children.
          // Chip and tick content is captured as its semantic box plus text range.
          const drop=p=>p.key.endsWith('/ripple')||p.key.includes('/aline/')||p.key.includes('/rj-t/');
          const array=all.filter(p=>!drop(p));
          for(const p of array){
            const n=entry.node;
            if(p.key.endsWith('/aline'))Object.assign(p,textDiag(n.querySelector('.aline')),{key:p.key});
            if(p.key.endsWith('/rj-t'))Object.assign(p,textDiag(n.querySelector('.rj-t')),{key:p.key});
            // The unknown mark is a CSS border with font-size:0, not a glyph.
            if(p.key.endsWith('/hollow')){p.kind='box';delete p.font_diagnostics;delete p.value;}
            // A zero-size marker becomes a separate flex item in a text-only
            // channel button. Read its real anonymous text range instead.
            const tab=/\/ch-tab(\d*)$/.exec(p.key);
            if(tab&&p.kind==='text'){
              const node=n.children[Number(tab[1]||0)],range=document.createRange();
              range.selectNodeContents(node.firstChild);
              p.font_diagnostics.baseline=range.getBoundingClientRect().y-node.getBoundingClientRect().y+p.font_diagnostics.ascent;
              p.font_diagnostics.source='inline-flex-text-range';
            }
          }
          diagnostics.push(...array);
          if(entry.node.classList.contains('unread')){
            const css=getComputedStyle(entry.node,'::after'),r=bounds(entry.node),w=parseFloat(css.width),h=parseFloat(css.height);
            if(!Number.isFinite(w+h))throw Error('Unread mark dimensions missing');
            diagnostics.push({key:key+'/unread-dot',kind:'box',rect:{x:r.x+r.w-parseFloat(css.right)-w,y:r.y+parseFloat(css.top),w,h}});
          }
        }
        frame.dataset.bboxyStream='';
        return {host:bounds(frame),view:{dock,framed,entries:entries.map(({node,...e})=>e)},parts:diagnostics};
      });
      if(!frames.length)throw Error('Stream frame missing');
      return JSON.parse(JSON.stringify({version:1,layer:'streams',frames},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    if(window.bboxyLauncherFixture){
      const call=calls.find(c=>c.name==='picker'&&c.node?.isConnected&&root.contains(c.node));
      if(!call)throw Error('Launcher constructor missing');
      const node=call.node,opts=call.args[0]||{};
      const kids=it=>typeof it.children==='function'?it.children():it.children;
      const tree=(items,depth=0)=>{if(depth>8)throw Error('Launcher tree exceeds eight levels');return items.map(it=>({
        id:it.id,label:it.label,sub:it.sub||'',harness:it.harness,kind:it.kind||it.hint||'',tile:it.tile||null,section:it.section||null,aliases:it.aliases||[],tags:it.tags||[],
        navigation:!!(it.seat||it.swarm||it.graphOpen||it.settingsOpen||it.section==='Navigate'||it.section==='Swarms'||it.kind==='Navigate'||it.kind==='Open'),
        children:it.children?tree(kids(it)||[],depth+1):null,
      }));};
      const nm=C.rollName(C.rng('roll'+(opts.seed||1))),input=node.querySelector('input');
      const diagnostics=parts(node,'picker');
      diagnostics.push({...inputDiag(input).children[0],key:'picker/inrow/input/text'});
      if(node.closest('.spot-back')){
        const nums=[...node.querySelectorAll('.opt .num')];let index=0;
        for(const p of diagnostics.filter(p=>p.key.match(/^picker\/div\/opt\d*\/num$/))){
          const n=nums[index++],range=document.createRange();range.selectNodeContents(n);
          const r=range.getBoundingClientRect(),b=n.getBoundingClientRect(),css=getComputedStyle(n),cv=document.createElement('canvas'),ctx=cv.getContext('2d');ctx.font=css.font;
          p.font_diagnostics={font:css.font,baseline:r.y-b.y+ctx.measureText(n.textContent).fontBoundingBoxAscent,source:'inline-flex-text-range'};
        }
      }

      node.dataset.bboxyLauncher='';
      const banner=node.parentElement.querySelector('.spot-banner');
      const grid=banner&&calls.find(c=>c.name==='miniGrid'&&c.node?.isConnected&&banner.contains(c.node));
      if(banner){
        banner.dataset.bboxyLauncher='';parts(banner,'banner',diagnostics);
        const of=banner.querySelector('.of'),raw=[...of.childNodes].filter(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim());
        for(let i=diagnostics.length-1;i>=0;i--)if(diagnostics[i].key==='banner/txt/of/text')diagnostics.splice(i,1);
        for(let i=0;i<raw.length;i++){
          const node=raw[i],range=document.createRange();range.selectNodeContents(node);const r=range.getBoundingClientRect(),css=getComputedStyle(of),cv=document.createElement('canvas'),ctx=cv.getContext('2d');ctx.font=css.font;
          const key='banner/txt/of/text'+(i||'');
          diagnostics.push({key,kind:'text',value:node.textContent,rect:{x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height},font_diagnostics:{baseline:ctx.measureText(node.textContent).fontBoundingBoxAscent,source:'text-range-font-ascent'}});
        }
      }
      return {version:1,layer:'launchers',spotlight:!!node.closest('.spot-back'),host:bounds(node),
        args:{items:tree(opts.items||C.LAUNCH),query:opts.query||'',path:opts.path||[],placeholder:opts.placeholder||'new seat · type to filter',cap:opts.cap||null,name:nm,swatch:color(C.seatColor({key:nm}),node),actions:Array.isArray(opts.actions)?opts.actions:[['Open','↵'],['Into','→'],['Back','←'],['Reroll','⌘R']]},
        banner:grid?{host:bounds(banner),tree:grid.args[0],focused_key:grid.args[1],title:banner.querySelector('.txt .big').textContent,anchor_color:color(C.seatColor(grid.args[0].a),node)}:null,
        parts:diagnostics};
    }
    if(window.bboxyTitleBarFixture){
      const call=calls.find(c=>c.name==='titleBar'&&c.node?.isConnected&&root.contains(c.node));
      const node=call?.node||root.querySelector('.titlebar');if(!node)throw Error('TitleBar root missing');
      node.dataset.bboxyTitlebar='';
      const parent=node.parentElement,css=getComputedStyle(parent),r=parent.getBoundingClientRect();
      const border=parseFloat(css.borderLeftWidth)||0;
      return JSON.parse(JSON.stringify({version:1,layer:'titlebars',args:call?.args||[{title:'bboxy450-term',crumb:'swarm / Vesper Greywind',sidebarOpen:true}],host:bounds(node),ancestor_clip:css.overflow==='hidden'?{rect:{x:r.x-origin.x+border,y:r.y-origin.y+border,w:r.width-2*border,h:r.height-2*border},radii:radii(css).map(v=>Math.max(0,v-border))}:null,parts:parts(node,'bar')},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    if(window.bboxyPaneFixture){
      const items=calls.filter(c=>c.name==='pane'&&c.node?.isConnected&&root.contains(c.node)).map(c=>{
        c.node.dataset.bboxyPane='';
        const glow=glows.find(g=>g.wrap.contains(c.node));
        // The root's placement/available width belongs to the enclosing fixture.
        // Height is solved from public min-height + content, not copied from DOM.
        const r=bounds(c.node),diagnostics=parts(c.node,'pane'),lines=[[]];let run=0;
        const term=c.node.querySelector('.term');
        const rangeDiag=(node,start,end,key)=>{
          const range=document.createRange();range.setStart(node,start);range.setEnd(node,end);const r=range.getBoundingClientRect(),css=getComputedStyle(node.parentElement),cv=document.createElement('canvas'),ctx=cv.getContext('2d');ctx.font=css.font;
          const value=node.textContent.slice(start,end),m=ctx.measureText(value);
          diagnostics.push({key,kind:'text',value,rect:{x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height},font_diagnostics:{baseline:m.fontBoundingBoxAscent,source:'text-range-font-ascent'}});
        };
        const walk=(node,flags={})=>{
          if(node.nodeType===Node.TEXT_NODE){let offset=0;const chunks=node.textContent.split('\n');chunks.forEach((value,i)=>{if(value){const key='pane/term/run'+run++;lines.at(-1).push({key,text:value,...flags});rangeDiag(node,offset,offset+value.length,key);}offset+=value.length+1;if(i<chunks.length-1)lines.push([]);});return;}
          if(node.classList.contains('gap-mark')){parts(node,'pane/term/gap-mark'+(diagnostics.some(p=>p.key==='pane/term/gap-mark')?'1':''),diagnostics);return;}
          if(node.classList.contains('cur')){const key='pane/term/cur';lines.at(-1).push({key,cursor:true});diagnostics.push({key,rect:bounds(node)});return;}
          if(node.classList.contains('path-tip')){
            const key='pane/term/path-tip',parts=[];diagnostics.push({key,rect:bounds(node)});
            for(const child of node.childNodes){const n=child.nodeType===Node.TEXT_NODE?child:child.firstChild;if(!n)continue;const key='pane/term/run'+run++;parts.push({key,text:n.textContent,kbd:child.nodeType!==Node.TEXT_NODE});rangeDiag(n,0,n.textContent.length,key);}
            lines.at(-1).push({key,hint:parts});return;
          }
          for(const child of node.childNodes)walk(child,{...flags,foreground:flags.foreground||node.tagName==='B'||node.classList.contains('path'),hover:flags.hover||node.classList.contains('hover')});
        };
        for(const child of term.childNodes)walk(child);
        return {args:c.args,lines,host:{x:r.x,y:r.y,w:r.w},parts:diagnostics,glow:glow?{seed:glow.seed,time_ms:0,focused:glow.focused}:null};
      });
      if(!items.length)throw Error('missing Pane constructor');
      return JSON.parse(JSON.stringify({version:1,layer:'panes',scope:'Pane and Strip; notes and adjacent SeatRows are shared fixture context',items},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    if(window.bboxySwarmFixture){
      const cardParts=(card,prefix='')=>{
        let diagnostics=[{key:'card',rect:bounds(card)}];
        const head=card.querySelector(':scope > .space-head');parts(head,'head',diagnostics);
        const body=card.querySelector(':scope > .space-body');
        if(body){
          diagnostics.push({key:'body',rect:bounds(body)});let ti=0,si=0,xi=0;
          for(const node of body.children){
            if(!node.className){diagnostics.push(...tabParts(node).map(p=>({...p,key:`body/tab${ti}/${p.key}`})));ti++;}
            else if(node.classList.contains('newseat'))parts(node,'body/newseat',diagnostics);
            else if(node.classList.contains('participants'))parts(node,'body/shadows',diagnostics);
            else if(node.classList.contains('phantom')){diagnostics.push(...parts(node).map(p=>({...p,key:`body/shadow${si}/${p.key}`})));si++;}
            else {diagnostics.push(...parts(node).map(p=>({...p,key:`body/extra${xi}/${p.key}`})));xi++;}
          }
        }
        return diagnostics.map(p=>({...p,key:prefix+p.key}));
      };
      const sidebar=window.bboxySwarmFixture==='sidebar';
      const call=calls.findLast(c=>c.name===(sidebar?'sidebar':'spaceHeader')&&c.node?.isConnected&&root.contains(c.node));
      if(!call)throw Error('missing SwarmCard/Sidebar constructor');
      const target=sidebar?call.node:call.node.closest('.space');target.dataset.bboxySwarm='';
      let diagnostics=[],bodyTabs=[];
      if(sidebar){
        diagnostics.push({key:'sidebar',rect:bounds(target)});
        const top=target.querySelector(':scope > .sb-top'),scroll=target.querySelector(':scope > .sb-scroll'),handle=target.querySelector(':scope > .handle');
        parts(top,'top',diagnostics);if(handle)parts(handle,'handle',diagnostics);
        diagnostics.push({key:'scroll',rect:bounds(scroll)});
        [...scroll.querySelectorAll(':scope > .space')].forEach((card,i)=>diagnostics.push(...cardParts(card,`swarm${i}/`)));
        const add=scroll.querySelector(':scope > .new-cohort');if(add)parts(add,'new-swarm',diagnostics);
      }else{
        diagnostics=cardParts(target);
        bodyTabs=calls.filter(c=>c.name==='tabRow'&&c.node?.isConnected&&target.contains(c.node)).map(c=>c.args);
      }
      // The phone is external fixture context. Export its padding-box clip,
      // independently of all sidebar descendants and their layout diagnostics.
      const phone=target.closest('.phone');let ancestor_clip=null;
      if(phone){
        const r=bounds(phone),s=getComputedStyle(phone),b=['Top','Right','Bottom','Left'].map(side=>parseFloat(s['border'+side+'Width'])||0);
        ancestor_clip={rect:{x:r.x+b[3],y:r.y+b[0],w:r.w-b[1]-b[3],h:r.h-b[0]-b[2]},radii:['TopLeft','TopRight','BottomRight','BottomLeft'].map((corner,i)=>Math.max(0,parseFloat(s['border'+corner+'Radius'])-Math.max(b[[0,0,2,2][i]],b[[3,1,1,3][i]])))};
      }
      return JSON.parse(JSON.stringify({version:1,layer:'swarms',kind:sidebar?'sidebar':'card',args:call.args,body_tabs:bodyTabs,host:bounds(sidebar?target:target.parentElement),phone:!!phone,ancestor_clip,scroll:target.querySelector('.sb-scroll')?.scrollTop||0,parts:diagnostics},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    if(window.bboxyTabsFixture){
      const tab=window.bboxyTabsFixture==='tab-row';
      const selected=calls.filter(c=>c.name===(tab?'tabRow':'miniGrid')&&c.node?.isConnected&&root.contains(c.node));
      if(!selected.length)throw Error('missing TabRow/MiniGrid constructor');
      const host=root.firstElementChild;
      const items=selected.map((c,i)=>{
        c.node.dataset.bboxyTabs='';
        let diagnostics=[];
        if(tab){
          diagnostics=tabParts(c.node);
        }else diagnostics=parts(c.node,`grid${i}`);
        const css=getComputedStyle(c.node);
        return {args:c.args,size:[parseFloat(css.getPropertyValue('--mg-w')),parseFloat(css.getPropertyValue('--mg-h'))],parts:diagnostics};
      });
      return JSON.parse(JSON.stringify({version:1,layer:'tabs',kind:window.bboxyTabsFixture,host:bounds(host),gap:parseFloat(getComputedStyle(host).gap)||0,items},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    if(window.bboxySeatRowFixture){
      const selected=calls.filter(c=>['seatRow','subagentRows','taskRow','artifactRow'].includes(c.name)&&c.node?.isConnected&&root.contains(c.node));
      const parents=[...new Set(selected.map(c=>c.node.parentElement))];
      const groups=parents.map(parent=>({host:bounds(parent),gap:parseFloat(parent.style.gap)||0,row_height:parseFloat(getComputedStyle(parent).getPropertyValue('--row-h')),rows:selected.filter(c=>c.node.parentElement===parent).map(c=>{c.node.dataset.bboxySeatRow='';return {kind:c.name,args:c.args,index:c.index,rect:bounds(c.node),parts:parts(c.node)};})}));
      return JSON.parse(JSON.stringify({version:1,layer:'seat-row',scope:'SeatRow and nested subagent/task/artifact rows; notes are shared context, row-menu overlays hidden on both sides',groups},(k,v)=>typeof v==='function'?{$callback:true}:v instanceof Node?undefined:v));
    }
    // Derived only from the public Path record, never SVG getBBox/path APIs.
    const pathGeometry=(o,scale)=>{
      let lo=[Infinity,Infinity],hi=[-Infinity,-Infinity],at=[0,0],endpoints=[];
      const include=p=>p.forEach((n,j)=>{lo[j]=Math.min(lo[j],n);hi[j]=Math.max(hi[j],n)});
      for(const p of o.points){
        if(p.move||p.line){at=(p.move||p.line).map(n=>n*scale);include(at);endpoints.push(at);continue;}
        const c=p.cubic.map(p=>p.map(n=>n*scale));
        for(let j=0;j<2;j++){
          const [p0,p1,p2,p3]=[at[j],c[0][j],c[1][j],c[2][j]],a=-p0+3*p1-3*p2+p3,b=2*(p0-2*p1+p2),d=p1-p0,disc=b*b-4*a*d;
          const roots=Math.abs(a)<1e-12?[Math.abs(b)>1e-12?-d/b:-1]:disc>=0?[(-b+Math.sqrt(disc))/(2*a),(-b-Math.sqrt(disc))/(2*a)]:[];
          for(const t of roots)if(t>0&&t<1){const u=1-t;include([0,1].map(k=>u*u*u*at[k]+3*u*u*t*c[0][k]+3*u*t*t*c[1][k]+t*t*t*c[2][k]));}
        }
        include(c[2]);endpoints.push(c[2]);at=c[2];
      }
      return {bounds:endpoints.length?[lo[0],lo[1],hi[0]-lo[0],hi[1]-lo[1]]:[0,0,0,0],endpoints};
    };
    if(window.bboxyPathFixture){
      const paths=[...root.querySelectorAll('.ui-path')].map(node=>{
        const call=uiNodes.get(node);if(!call||call.kind!=='path')throw Error('unrecorded public Path');
        node.dataset.bboxyPath='';const scale=Number(node.closest('.path-host').dataset.scale);return {options:call.args[0],geometry:pathGeometry(call.args[0],scale),host:bounds(node),scale};
      });
      return {version:1,layer:'paths',paths};
    }
    let component;
    if(window.bboxyComponentFixture){
      const demo=root.querySelector('.ui-scope[data-density] > div[style*="margin-top"]')?.firstElementChild;
      if(!demo)throw Error('missing single component demo');
      demo.dataset.bboxyCaptureDemo='';component={tree:exportUi(demo),host:bounds(demo)};
    }
    const glowFor=node=>{const g=glows.find(g=>g.wrap.contains(node));return g?{seed:g.seed,activity:g.activity,focused:g.focused,time_ms:g.time_ms,canvas:bounds(g.cv)}:null;};
    const atoms=window.bboxyAtomFixture?[...root.querySelectorAll('.glyph,.hicon')].map(node=>{
      const css=getComputedStyle(node),glyph=node.classList.contains('glyph');
      const result={kind:glyph?'glyph':'icon',rect:bounds(node),color:color(css.color,node),opacity:Number(css.opacity),
        ...(glyph?{activity:['active','idle','waiting','errored'].find(x=>node.classList.contains(x)),grade:['reported','shim','inferred'].find(x=>node.classList.contains('g-'+x)),wait_kind:['permission','question','auth'].find(x=>node.classList.contains('k-'+x))}:{name:node.title})};
      if(glyph){
        const main=[...node.childNodes].find(n=>n.nodeType===Node.TEXT_NODE&&n.textContent.trim());
        if(!main)throw Error('glyph text is missing');
        result.text=rawTextDiag(main);
        const mark=node.querySelector(':scope > .kind');
        if(mark){result.kind_rect=bounds(mark);result.kind_text=rawTextDiag(mark.firstChild);}
      }
      return result;
    }):[];
    const squares=[...root.querySelectorAll('.ui-sq')].map((node,index)=>{const input=materials[index];const css=getComputedStyle(node);return {...input,glow:glowFor(node),rect:bounds(node),radii:radii(css),color:color(input.params.color||'var(--accent)',node),tint:color(input.params.tint==='none (Style fill)'?'transparent':input.params.tint||'transparent',node)};});
    const boxes=styles.filter(c=>c.node.isConnected&&root.contains(c.node)).map(({style,node})=>{
      const css=getComputedStyle(node),text=node.querySelector('.ui-text'),tc=getComputedStyle(text);
      return {style,glow:glowFor(node),rect:bounds(node),computed:{fill:color(css.backgroundColor,node),radii:radii(css),padding:[css.paddingTop,css.paddingRight,css.paddingBottom,css.paddingLeft].map(parseFloat),border_width:parseFloat(css.borderTopWidth)},
        text:{value:text.innerText,rect:bounds(text),font_diagnostics:textDiag(text).font_diagnostics,size:parseFloat(tc.fontSize),color:color(tc.color,text),weight:Number(tc.fontWeight)}};
    });
    return JSON.parse(JSON.stringify({ version: 1, layer:window.bboxyAtomFixture?'atoms':window.bboxyComponentFixture?'components':undefined, component, atoms, calls: serialized, materials:squares, styles:boxes }, (key, value) => {
      if (typeof value === 'function') return { $callback: true };
      if (value instanceof Node) return undefined;
      if (value instanceof Set) return [...value];
      return value;
    }));
  };
})();
