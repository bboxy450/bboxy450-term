// Capture component inputs, not DOM internals. Both renderers receive the same
// data and outer fixture rectangles; native components still lay out/draw their
// own contents. Keep references until capture so live seat state is current.
(() => {
  const calls = [];
  const materials = [];
  const styles = [];
  const glows = [];
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
    const box=value.Box;
    value.Box=function(style,...children){const node=box(style,...children);styles.push({style,node});return node;};
  }});
  let components;
  Object.defineProperty(window, 'C', {
    configurable: true,
    get: () => components,
    set(value) {
      components = value;
      for (const name of ['sidebar', 'pane', 'seatRow', 'tabRow', 'miniGrid',
        'spaceHeader', 'graphPage', 'provenanceCard', 'subscriptionsCard',
      'externalLane', 'messageRow', 'glyph', 'titleStrip']) {
        const original = value[name];
        if (typeof original !== 'function') continue;
        value[name] = function (...args) {
          const node = original.apply(this, args);
          calls.push({ name, args, node });
          return node;
        };
      }
    },
  });
  window.bboxyCaptureInputs = selector => {
    const root = document.querySelector(selector);
    const origin = root.getBoundingClientRect();
    const serialized = calls.filter(c => c.node?.isConnected && root.contains(c.node)).map(c => {
      const r = c.node.getBoundingClientRect();
      return { name: c.name, args: c.args,
        header: c.node.querySelector?.('.sb-top')?.getBoundingClientRect().height || 0,
        rect: { x: r.x - origin.x, y: r.y - origin.y, w: r.width, h: r.height } };
    });
    const color=(value,node)=>{const probe=document.createElement('span');probe.style.color=value;node.append(probe);const c=getComputedStyle(probe).color;probe.remove();const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d');ctx.fillStyle=c;ctx.fillRect(0,0,1,1);return [...ctx.getImageData(0,0,1,1).data].map(c=>c/255);};
    const bounds=node=>{const r=node.getBoundingClientRect();return {x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height};};
    const radii=css=>[css.borderTopLeftRadius,css.borderTopRightRadius,css.borderBottomRightRadius,css.borderBottomLeftRadius].map(parseFloat);
    const glowFor=node=>{const g=glows.find(g=>g.wrap.contains(node));return g?{seed:g.seed,activity:g.activity,focused:g.focused,time_ms:g.time_ms,canvas:bounds(g.cv)}:null;};
    const squares=[...root.querySelectorAll('.ui-sq')].map((node,index)=>{const input=materials[index];const css=getComputedStyle(node);return {...input,glow:glowFor(node),rect:bounds(node),radii:radii(css),color:color(input.params.color||'var(--accent)',node),tint:color(input.params.tint==='none (Style fill)'?'transparent':input.params.tint||'transparent',node)};});
    const boxes=styles.filter(c=>c.node.isConnected&&root.contains(c.node)).map(({style,node})=>{
      const css=getComputedStyle(node),text=node.querySelector('.ui-text'),tc=getComputedStyle(text);
      return {style,glow:glowFor(node),rect:bounds(node),computed:{fill:color(css.backgroundColor,node),radii:radii(css),padding:[css.paddingTop,css.paddingRight,css.paddingBottom,css.paddingLeft].map(parseFloat),border_width:parseFloat(css.borderTopWidth)},
        text:{value:text.innerText,rect:bounds(text),size:parseFloat(tc.fontSize),color:color(tc.color,text),weight:Number(tc.fontWeight)}};
    });
    return JSON.parse(JSON.stringify({ version: 1, calls: serialized, materials:squares, styles:boxes }, (key, value) => {
      if (typeof value === 'function') return { $callback: true };
      if (value instanceof Node) return undefined;
      if (value instanceof Set) return [...value];
      return value;
    }));
  };
})();
