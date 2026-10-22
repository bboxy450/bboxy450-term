// Capture component inputs, not DOM internals. Both renderers receive the same
// data and outer fixture rectangles; native components still lay out/draw their
// own contents. Keep references until capture so live seat state is current.
(() => {
  const calls = [];
  const materials = [];
  let ui;
  Object.defineProperty(window,'UI',{configurable:true,get:()=>ui,set(value){ui=value;const original=value.material;value.material=function(name,options){const result=original(name,options);materials.push({kind:name,params:result.params});return result;};}});
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
    const squares=[...root.querySelectorAll('.ui-sq')].map((node,index)=>{const input=materials[index];const r=node.getBoundingClientRect();const css=getComputedStyle(node);return {...input,rect:{x:r.x-origin.x,y:r.y-origin.y,w:r.width,h:r.height},radii:[css.borderTopLeftRadius,css.borderTopRightRadius,css.borderBottomRightRadius,css.borderBottomLeftRadius].map(parseFloat),color:color(input.params.color||'var(--accent)',node),tint:color(input.params.tint||'var(--surface)',node)};});
    return JSON.parse(JSON.stringify({ version: 1, calls: serialized, materials:squares }, (key, value) => {
      if (typeof value === 'function') return { $callback: true };
      if (value instanceof Node) return undefined;
      if (value instanceof Set) return [...value];
      return value;
    }));
  };
})();
