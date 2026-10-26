"""Assertions over the same positioned layout the host has just painted."""
import json,os,sys
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'scripts/e2e'))
from ui import App,nodes,require
app=App();phase,seat,name=sys.argv[1:]
layout=app.ctl('ui','layout',label='t7-layout-'+phase)
all_nodes=list(nodes(layout));panes={p['seat']:p for p in layout['panes']}
closed=[n for n in all_nodes if n['kind'].endswith('/closed/'+seat)]
if phase=='before':
    require(len(panes)==3 and seat in panes,'fixture must have three real solved panes')
    require(layout['focused']==seat and panes[seat]['focused'],'fixture did not focus its middle pane')
    require(len({tuple(p['rect']) for p in panes.values()})==3,'split fixture has coincident pane rectangles')
elif phase.startswith('grace'):
    require(seat not in panes and len(panes)==2,'closed pane remained in the solved stage')
    require(closed,'closed grace strip is not in the produced layout')
    strip=next(n for n in closed if n['rect'][2]>0 and n['rect'][3]>0)
    def walk(n):
        yield n
        for c in n.get('children',[]):yield from walk(c)
    inside=list(walk(strip))
    require(any(n['kind'].endswith('/undo') and n['rect'][2]>0 for n in inside),'grace strip has no positioned Undo control')
    require(any('undo' in n.get('text','') and '⌘Z' in n.get('text','') for n in inside),'grace strip has no drawn Cmd-Z label')
    require(any(n.get('text','').endswith(' s') for n in inside),'grace strip has no drawn remaining seconds')
elif phase=='restored':
    before=json.loads((app.work/'t7-layout-before.jsonl').read_text())['data']
    old={p['seat']:p for p in before['panes']}
    require(set(panes)==set(old),'undo changed pane membership')
    for key in old:
        for field in ['rect','cols','rows','focused']:
            require(panes[key][field]==old[key][field],f'{key}: undo changed solved {field}')
    for field in ['focused','tab','swarm']:
        require(layout[field]==before[field],f'undo changed {field}')
    require(not closed,'undo retained a closed strip')
elif phase=='gone':
    require(seat not in panes and len(panes)==2,'expired pane remains on stage')
    require(not closed,'expired Undo strip remains visible')
    require(not any(n['kind'].startswith('named:swarm') and n['kind'].endswith('/name') and
                    any(c.get('text')==name for c in n.get('children',[])) for n in all_nodes),'expired seat row remains in sidebar')
    text=' '.join(n.get('text','') for n in all_nodes)
    require(text.count(name+' closed; its history is kept')==1,'final stream notice is not drawn exactly once')
else:raise AssertionError('unknown phase '+phase)
app.capture('window','t7-window-'+phase,layout['frame_seq'])
print(json.dumps({'phase':phase,'frame_seq':layout['frame_seq'],'panes':{s:p['rect'] for s,p in panes.items()},'focused':layout['focused'],'closed_strips':len(closed)}))
