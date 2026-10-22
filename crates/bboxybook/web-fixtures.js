// The capture adapter calls the lab's actual exported components. Fixture-only
// atoms use the lab's classes; it never embeds or screenshots the native PNG.
function fixture(c) {
  const el=Lab.el,body=document.querySelector('.card > .body'),r=el('div',{style:`width:${c.width-28}px;height:${c.height-28}px;overflow:hidden`});body.replaceChildren(r);
  const s={key:'sc0',name:'Vesper Greywind',harness:'claude',role:'oracle',title:'swarm control',state:'active',grade:'reported',color:Lab.prefs.theme==='light'?'#B8860B':'#FFC53D'};
  const b={key:'sc1',name:'Nyx Copperwing',harness:'codex',role:'executor',title:'messaging spec',state:'active',grade:'reported',color:Lab.prefs.theme==='light'?'#C2185B':'#FF4FA3'};
  const tree={dir:'v',ratio:.6,a:s,b:{dir:'h',ratio:.5,a:b,b:{...s,key:'sc3',name:'Rook Ashwhisker',harness:'codex',role:'executor',state:'waiting'}}};
  const art={kind:'image',ref:'target/render/smoke.png',digest:'a723f1c9',responsible:b.name,responsibleColor:b.color};
  const task={id:'task-render',title:'Renderer harness',state:'reported',assignee:b.name,artifacts:[art]};
  const epic={id:'epic-render',title:'Render terminal',items:[{type:'spec',title:'Renderer',state:'done',assignee:s.name},task,{title:'Chrome parity',state:'claimed',assignee:b.name}]};
  const prov={items:[{type:'spec',title:'Renderer',created_by:s.name,updated_by:b.name,createdColor:s.color,updatedColor:b.color,derived_from:'[[renderer]]{derives:0.8} [[terminal-core]]{constrains:1.0}',digest:{content:'a723f1c9',signed:true,by:'Boxy'}}],log:[{hash:'12fa723',time:'13:40',text:'Nyx Copperwing created renderer.md'},{hash:'23ca723',time:'23:59',checkpoint:true,by:'Boxy'}]};
  const subs=[{seat:s.name,color:s.color,patterns:[{pattern:'task/*',auto:true,why:'delegated tasks'},{pattern:'seat/*/state'},{pattern:'doc/specs/**'}]},{seat:b.name,color:b.color,patterns:[{pattern:'task/w2-1',auto:true,why:'delegated task'},{pattern:'doc/specs/mux-daemon',auto:true,why:'derived_from'},{pattern:'artifact/*/for:w2-1'}]}];
  const external=[{source:'github',text:'PR #14 review requested',time:'13:45'},{source:'ci',text:'macos-15 · render smoke · passed',time:'13:58'},{source:'cloud',text:'eu-2 · seat reconnected',time:'14:10'}];
  const options={focused:c.state==='focused',unfocused:c.state==='unfocused',shortcut:'⌘1'};
  const icon=name=>C.harnessIcon(name);
  let node;
  switch(c.component){
    case 'tile':node=el('span',{class:'tile',style:'width:30px;height:30px;border-radius:8px;display:inline-flex;align-items:center;justify-content:center;background:#D97757;color:white'},icon('claude'));break;
    case 'badge':node=el('span',{class:'badge'},'3');break;
    case 'kbd':node=C.kbd('⌘1');break;
    case 'role_mark':node=C.roleMark('oracle');break;
    case 'where_mark':node=el('div',{style:'display:flex;gap:24px'},icon('ssh'),icon('cloud'));break;
    case 'swatch':node=el('span',{class:'swatch',style:'display:block;width:12px;height:12px;border-radius:50%;background:#FFC53D;border:1.5px solid white'});break;
    case 'hairline':node=el('div',{style:'width:240px;height:1px;margin-top:5px;background:var(--hair)'});break;
    case 'pill':node=el('div',{class:'act-pill',style:'display:inline-flex;gap:24px;padding:8px 14px;border-radius:999px;background:var(--surface)'},'Launch ↵','Split right ⌘↵','Split down ⌘⇧↵');break;
    case 'crumb':node=el('span',{class:'crumb'},'SSH › rig');break;
    case 'chevron':node=el('span',{class:'chev'},'›');break;
    case 'bumper':node=el('div',{style:'width:260px;height:4px;background:#FFC53D;border-radius:0 0 12px 12px'});break;
    case 'close_dot':node=el('span',{class:'close',style:'display:block;width:12px;height:12px;border-radius:50%;background:#FF5F57'},c.state==='focused'?'×':'');break;
    case 'mute_toggle':case 'muted_mark':node=el('span',{class:'mutemark',style:'color:var(--c-waiting)'},icon(c.state==='focused'||c.component==='muted_mark'?'muted':'unmuted'));break;
    case 'ghost':node=el('div',{class:'drag-ghost',style:'position:static;display:inline-block;--seat:'+b.color},b.name);break;
    case 'drop_zone':node=C.pane(s,{focused:true,height:100});node.style.width='500px';node.style.position='relative';node.append(el('div',{style:'position:absolute;right:0;top:0;width:50%;height:100%;background:color-mix(in srgb,var(--accent) 22%,transparent)'}));break;
    case 'divider':node=el('div',{style:'display:flex;gap:2px'},el('div',{style:'width:120px;height:80px;background:var(--surface-2)'}),el('div',{style:'width:120px;height:80px;background:var(--surface-2)'}));break;
    case 'cursor':node=el('span',{style:'display:block;width:8px;height:16px;background:var(--fg)'});break;
    case 'title_bar':node=el('div',{class:'titlebar',style:'display:flex;align-items:center;gap:10px;height:38px;background:var(--surface);padding:0 14px'},icon('sidebarfoxopen'),C.logo(18),'bboxy450-term',el('span',{style:'margin-left:80px;color:var(--fg-dim)'},'swarm / Vesper Greywind'));break;
    case 'swarm_card':{const model={spaces:[{name:'term',key:'space-term',tabs:[{name:'core',tree,seats:[s,b,tree.b.b]}]}]};node=C.sidebar(model,{height:c.height-28});node.style.width='300px';break;}
    case 'new_seat_row':node=el('div',{class:'tab-row newseat',style:'width:300px'},el('span',{class:'plus'},'+'),'new seat',el('span',{style:'margin-left:auto'},C.kbd('⌘T')));break;
    case 'closed_strip':node=el('div',{class:'tab-row closed-strip',style:'width:300px'},'Nyx Copperwing · 52s',el('button',{class:'undo'},'undo ⌘Z'));break;
    case 'new_swarm_button':node=el('div',{class:'new-cohort',style:'width:260px'},'+ new swarm');break;
    case 'stream_dock':node=el('div',{class:'stream-dock',style:'width:700px'},C.messageRow({from:s.name,role:'oracle',time:'12:07',text:'B claimed. File:line pointers checked. Report is ready.',unread:true}),C.messageRow({from:b.name,role:'executor',time:'12:07',text:'B claimed. File:line pointers checked. Report is ready.',unread:true}));break;
    case 'pane':node=C.pane(s,{...options,height:c.height-28});node.style.width='700px';break;
    case 'banner':node=el('div',{class:'banner'},el('span',{},'Split right'),C.miniGrid(tree,b.key,true));break;
    case 'glow_canvas':node=C.pane(s,{focused:true,height:100});node.style.width='450px';break;
    case 'gap_marker':node=C.gapMarker(c.state==='retained'?{from:'000042'}:null);node.style.width='700px';break;
    case 'tick_ladder':node=C.tickLadder(c.state);break;
    case 'nested_subagent_rows':{const n=c.state==='overflow'?10:2;node=el('div',{style:'width:700px'},...C.subagentRows({...s,harness:c.state==='codex'?'codex':'claude',subagents:Array.from({length:n},(_,i)=>({state:i===1?'waiting':'active',kind:'question',description:i===1?'Check assumptions':'Inspect renderer'}))}));break;}
    case 'task_row':node=C.taskRow(task,b,{onAccept:()=>{}});node.style.width='500px';break;
    case 'epic_group':node=C.epicGroup(epic,{});node.style.width='700px';break;
    case 'artifact_row':node=C.artifactRow(art,{});break;
    case 'responsible_chip':node=el('span',{class:'achip',style:'color:'+b.color+';border-color:'+b.color},b.name);break;
    case 'touched_count':node=el('span',{class:'touched'},icon('archive'),'2 artifacts');break;
    case 'wikilink_chip':node=C.renderLinks('[[renderer]]{derives:0.8}');break;
    case 'provenance_card':node=C.provenanceCard(prov);node.style.width='700px';break;
    case 'subscriptions_card':node=C.subscriptionsCard(subs);node.style.width='700px';break;
    case 'external_lane':node=C.externalLane(external,{open:true});node.style.width='700px';break;
    case 'graph_tab':node=C.graphPage({swarm:'term',seats:[s,{...b,tasks:[task]}],epics:[epic],history:[{time:'13:40',seat:b.name,event:'created',kind:'markdown',ref:'renderer.md',grade:'reported'}],provenance:prov,subscriptions:subs});break;
    default:throw Error('No web capture fixture for '+c.component);
  }
  if(c.state==='unfocused'){node.style.filter='saturate('+TOKENS.dim.saturate+')';node.style.opacity=TOKENS.dim.opacity;}
  r.append(node);return true;
}
