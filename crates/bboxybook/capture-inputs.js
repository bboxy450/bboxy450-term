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
    window.bboxyFreezeLayer=()=>{for(const time of [0,0.000001,450]){const ready=frames;frames=[];for(const f of ready)f.fn(time);}};
    let glow;
    Object.defineProperty(window,'Glow',{configurable:true,get:()=>glow,set(value){glow=value;const attach=value.attach;value.attach=function(wrap,seat,opts){const seed=randomIndex;const cv=attach(wrap,seat,opts);glows.push({wrap,cv,seed,activity:seat.state,focused:!!opts?.focused,time_ms:450});return cv;};}});
  }
  let ui;
  Object.defineProperty(window,'UI',{configurable:true,get:()=>ui,set(value){
    ui=value;const original=value.material;
    value.material=function(name,options){const result=original(name,options);materials.push({kind:name,params:result.params});return result;};
    for (const name of ['Text','Stack','Spacer','Button','List','Table','Input','Overlay','Modal']) {
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
      'externalLane', 'messageRow', 'glyph', 'titleStrip', 'subagentRows', 'taskRow', 'artifactRow']) {
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
      const call=uiNodes.get(node);if(!call)throw Error('unrecorded UI component');
      const result={kind:call.kind,rect:bounds(node)};
      if(call.kind==='text')Object.assign(result,textDiag(node),{value:call.args[0],options:call.args[1]||{}});
      else if(call.kind==='box'){result.style=call.style;result.children=call.children.map(exportUi);}
      else if(call.kind==='stack'){result.options=call.args[0]||{};result.children=call.args.slice(1).map(exportUi);}
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
      const key=path||'row',css=getComputedStyle(node);if(css.display==='none')return out;
      const r={key,rect:bounds(node)};
      if(leaf(node)&&!node.classList.contains('glyph')&&!node.classList.contains('hicon')&&node.textContent){Object.assign(r,{kind:'text',...textDiag(node)});r.key=key;}
      r.css={font:css.font,line_height:css.lineHeight,background:css.backgroundColor,shadow:css.boxShadow,opacity:css.opacity,border_radius:css.borderRadius,flex:css.flex,padding:css.padding};out.push(r);
      if(!leaf(node)){const counts={};for(const ch of [...node.childNodes]){
        if(ch.nodeType===Node.TEXT_NODE&&ch.textContent.trim()){out.push({...rawTextDiag(ch),key:(path?path+'/':'')+'text'});continue;}
        if(ch.nodeType!==Node.ELEMENT_NODE)continue;
        const cls=ch.classList[0]||ch.tagName.toLowerCase(),i=counts[cls]||0;counts[cls]=i+1;parts(ch,(path?path+'/':'')+cls+(i?i:''),out);
      }}
      return out;
    };
    if(window.bboxyTabsFixture){
      const tab=window.bboxyTabsFixture==='tab-row';
      const selected=calls.filter(c=>c.name===(tab?'tabRow':'miniGrid')&&c.node?.isConnected&&root.contains(c.node));
      if(!selected.length)throw Error('missing TabRow/MiniGrid constructor');
      const host=root.firstElementChild;
      const items=selected.map((c,i)=>{
        c.node.dataset.bboxyTabs='';
        let diagnostics=[];
        if(tab){
          diagnostics.push({key:'tab',rect:bounds(c.node)});
          const header=c.node.querySelector(':scope > .tab-row');
          if(header)parts(header,'header',diagnostics);
          const body=c.node.querySelector(':scope > .tab-body');
          if(body)diagnostics.push({key:'body',rect:bounds(body)});
          let seat=-1,child=0;
          for(const node of [...(body||c.node).children]){
            if(node===header)continue;
            if(node.classList.contains('seat-row')){seat++;child=0;diagnostics.push(...parts(node).map(p=>({...p,key:`seat${seat}/${p.key}`})));}
            else if(seat>=0){diagnostics.push(...parts(node).map(p=>({...p,key:`seat${seat}/child${child}/${p.key}`})));child++;}
          }
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
    let component;
    if(window.bboxyComponentFixture){
      const demo=root.querySelector('.ui-scope[data-density] > div[style*="margin-top"]')?.firstElementChild;
      if(!demo)throw Error('missing single component demo');
      demo.dataset.bboxyCaptureDemo='';component={tree:exportUi(demo),host:bounds(demo)};
    }
    const glowFor=node=>{const g=glows.find(g=>g.wrap.contains(node));return g?{seed:g.seed,activity:g.activity,focused:g.focused,time_ms:g.time_ms,canvas:bounds(g.cv)}:null;};
    const atoms=window.bboxyAtomFixture?[...root.querySelectorAll('.glyph,.hicon')].map(node=>{
      const css=getComputedStyle(node),glyph=node.classList.contains('glyph');
      return {kind:glyph?'glyph':'icon',rect:bounds(node),color:color(css.color,node),opacity:Number(css.opacity),
        ...(glyph?{activity:['active','idle','waiting','errored'].find(x=>node.classList.contains(x)),grade:['reported','shim','inferred'].find(x=>node.classList.contains('g-'+x)),wait_kind:['permission','question','auth'].find(x=>node.classList.contains('k-'+x))}:{name:node.title})};
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
