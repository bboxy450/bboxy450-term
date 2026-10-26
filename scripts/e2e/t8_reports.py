"""Real-PTY OSC reports; fixture acknowledgements never contain an app verdict."""
import json,math,os,re,sys,termios,time,tty
from pathlib import Path

def program():
    saved=termios.tcgetattr(0);tty.setraw(0);buffer=b'';phase=0
    try:
        os.write(1,b'\x1b[?25lT8_READY\r\n')
        while True:
            data=os.read(0,4096)
            if not data:return
            buffer+=data
            while b'\n' in buffer:
                line,buffer=buffer.split(b'\n',1);command=line.decode();phase+=1
                base='\x1b]7501;state=idle:app=t8\x07'
                frames={
                    'idle':base,'working':'\x1b]7501;state=working:app=t8\x07',
                    'unknown':base+'\x1b]7501;state=not-a-state:app=t8:id=bad-state\x07',
                    'bad-id':base+'\x1b]7501;state=working:app=t8:id=bad/id\x07',
                    'oversize':base+'\x1b]7501;state=working:app=t8:id=oversize:msg='+('A'*5000)+'\x07',
                    'version':base+'\x1b]7502;v=999:ev=SubagentStart:d=e30=\x07',
                    'child':'\x1b]7501;state=working:app=t8:id=t8-child\x07',
                    'clear':'\x1b]7501;state=clear:app=t8:id=t8-child\x07'+base,
                }
                assert command in frames,command
                os.write(1,(frames[command]+f'\r\nT8_DONE {phase} {command}\r\n').encode())
    finally:termios.tcsetattr(0,termios.TCSANOW,saved)

if len(sys.argv)>1 and sys.argv[1]=='program':
    program();raise SystemExit(0)
sys.path.insert(0,str(Path(__file__).resolve().parents[2]/'scripts/e2e'))
from ui import App,nodes,require
from rim_clock import png,region
app=App();seat=app.ctl('seat','spawn','claude','--role','executor','--place','tab','--cwd',app.work,'--name','T8 reported fixture')['seat']
app.wait(seat,'T8_READY')
end=time.monotonic()+30
while True:
    quiet=next(r for r in app.ctl('seat','ls') if r['record']['id']==seat)['state']
    if quiet['activity']['kind']=='idle' and quiet['grade']=='inferred':break
    require(time.monotonic()<end,'silent fixture never became inferred idle: '+str(quiet))
    time.sleep(.05)
app.ctl('ui','clock','freeze');phase=0;observations=[]
def row():return next(r for r in app.ctl('seat','ls') if r['record']['id']==seat)
def capture(label):
    layout=app.ctl('ui','focus',seat,label=label+'-layout')
    tree=list(nodes(layout));name=next(n for n in tree if n['kind'].startswith('named:swarm') and n['kind'].endswith('/name') and any(c.get('text')=='T8 reported fixture' for c in n.get('children',[])))
    key=name['kind'][:-len('/name')]+'/right/glyph';glyph=next(n for n in tree if n['kind']==key)
    path=app.capture('window',label,layout['frame_seq']);return png(path),glyph['rect'],layout
before=capture('t8-inferred');observations.append(row())
def send(command):
    global phase
    phase+=1;app.ctl('seat','type',seat,command+'\n');app.wait(seat,'(?m)^T8_DONE '+str(phase)+' '+re.escape(command)+'$')
send('idle');app.ctl('seat','wait',seat,'--state','idle','--grade','reported','--timeout','30000')
after=capture('t8-reported');observations.append(row())
assert before[1]==after[1]
x,y,w,h=after[1];x0,y0=math.floor(x),math.floor(y);box=[x0-3,y0-3,math.ceil(x+w)+3,math.ceil(y+h)+3]
a,b=region(before[0],box),region(after[0],box)
# Same idle activity, seat and produced-frame clock; grade is the only changed ink.
def grade_ink(image):
    background=region(after[0],[x0-3,y0-3,x0-2,y0-2])[:3]
    inner=[];outer=[]
    for j in range(y0-3,math.ceil(y+h)+3):
        for i in range(x0-3,math.ceil(x+w)+3):
            pixel=region(image,[i,j,i+1,j+1])[:3]
            contrast=sum(abs(int(c)-int(bg)) for c,bg in zip(pixel,background))
            radius=math.hypot(i+.5-(x+w/2),j+.5-(y+h/2))
            if radius<=w/2:inner.append(contrast)
            if w/2+.75<=radius<=w/2+2.5:outer.append(contrast)
    return {'inner':sum(inner),'peak':max(inner),'outer':sum(outer)}
ink={'inferred':grade_ink(before[0]),'reported':grade_ink(after[0])}
require(ink['reported']['inner']>ink['inferred']['inner'], 'reported grade did not brighten the idle mark')
require(ink['reported']['peak']>ink['inferred']['peak'], 'reported grade lacks full-opacity ink')
require(ink['inferred']['outer']>ink['reported']['outer'], 'inferred dashed ring did not disappear on reported grade')
print(json.dumps({'glyph_rect':after[1],'grade_ink':ink,'states':[r['state'] for r in observations]}),flush=True)
for command in ['unknown','bad-id','oversize','version']:
    send(command);state=row()['state'];require(state['activity']['kind']=='idle' and state['grade']=='reported' and state['subagents']==[],command+' manufactured state or child');observations.append(row())
send('working');app.ctl('seat','wait',seat,'--state','active','--grade','reported','--timeout','30000');capture('t8-working')
send('child');app.ctl('seat','wait',seat,'--state','active','--subagent','t8-child','--grade','reported','--timeout','30000');capture('t8-child')
send('clear');app.ctl('seat','wait',seat,'--state','idle','--grade','reported','--timeout','30000');require(row()['state']['subagents']==[],'clear left a running child')
(app.work/'t8-report.json').write_text(json.dumps({'seat':seat,'observations':observations,'glyph_rect':after[1],'grade_ink':ink},indent=2)+'\n')
print('T8: real OSC state, silent inferred idle, reported idle/active/child/clear and invalid reports held',flush=True)
