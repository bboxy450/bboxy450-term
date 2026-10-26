"""App-only SIGKILL: retain real PTYs and restore the first window from its journal."""
import json,os,re,signal,subprocess,sys,time
from pathlib import Path
sys.path.insert(0,str(Path(__file__).resolve().parent))
from ui import App,require
app=App();run=Path(os.environ['BBOXY_RUN_DIR']).resolve();account=app.work.parent/'account'
require(account.is_dir(),'runner file-account directory is missing')
app.ctl('app','resize','160x50')
seats=[]
for index,place in enumerate(['tab','right','down']):
    seat=app.ctl('seat','spawn','zsh','--place',place,'--cwd',app.work,'--name',f'T5 pane {index+1}')['seat'];seats.append(seat)
    app.ctl('ui','focus',seat)
program=app.work/'t5-program.py'
program.write_text("import os,sys\nindex=int(sys.argv[1]);cookie=55119+index\nprint('\\x1bc',end='')\nfor n in range(2000):print(f'T5_{index}_{n:05}')\nprint(f'T5_READY_{index} {os.getpid()}',flush=True)\nfor line in sys.stdin: print(f'T5_KEPT_{index} {cookie} {os.getpid()}',flush=True)\n")
import shlex
for index,seat in enumerate(seats):
    app.line(seat,'exec python3 -I -B '+shlex.quote(str(program))+' '+str(index))
    app.wait(seat,f'(?m)^T5_READY_{index} [0-9]+$')
for seat,row in zip(seats[:2],[23,61]):app.ctl('ui','scroll',seat,'row',row)
layout=app.ctl('ui','focus',seats[2],label='t5-before-layout')
def shape(layout):
    return {'size':layout['size'],'swarm':layout['swarm'],'tab':layout['tab'],'focused':layout['focused'],
            'panes':[dict(seat=p['seat'],rect=p['rect'],cols=p['cols'],rows=p['rows'],focused=p['focused'],scroll=p['scroll']) for p in layout['panes']]}
def roster():return {r['record']['id']:r for r in app.ctl('seat','ls')}
before=roster();previous=shape(layout);require(len(previous['panes'])==3,'fixture does not have three real panes')
app.capture('window','t5-before',layout['frame_seq'])
owners={name:json.loads((run/(name+'.owner.json')).read_text())['process']['pid'] for name in ['logic','supervisor']}
old=int(os.environ['E2E_APP']);query=['ps','-p',str(old),'-o','lstart=,args=']
fingerprint=subprocess.check_output(query,text=True)
require(str(run) in fingerprint and '--headless' in fingerprint,'fault PID is not this runner-owned app')
require(subprocess.check_output(query,text=True)==fingerprint,'app identity changed before fault')
os.kill(old,signal.SIGKILL)
end=time.monotonic()+10
while subprocess.run(['ps','-p',str(old),'-o','pid='],capture_output=True).returncode==0:
    require(time.monotonic()<end,'old app did not exit');time.sleep(.02)
start=app.ctl('app','start','--headless','--ctl','--unattended','--test-account',account,label='t5-relaunch')
os.environ['E2E_APP']=str(start['pid'])
require(start['frame_seq']==1,'readiness did not name the first restored frame')
first=app.capture('window','t5-first-after',start['frame_seq'])
restored=app.ctl('ui','layout',label='t5-after-layout');actual=shape(restored)
require(actual==previous,'restored layout/focus/scroll differs: '+json.dumps({'before':previous,'after':actual}))
after=roster()
for seat in seats:
    require(after[seat]['record']['generation']==before[seat]['record']['generation'],'app crash changed a process generation')
    require(after[seat]['state']['process']==before[seat]['state']['process'],'app crash replaced a PTY/program')
for name,pid in owners.items():require(json.loads((run/(name+'.owner.json')).read_text())['process']['pid']==pid,'relaunch replaced '+name)
for index,seat in enumerate(seats):
    app.line(seat,'check')
    app.wait(seat,f'(?m)^T5_KEPT_{index} {55119+index} '+str(before[seat]['state']['process']['pid'])+'$')
report={'before':previous,'after':actual,'owners':owners,'old_app':old,'new_app':start['pid'],'ready_frame':start['frame_seq'],'first_capture':str(first),'pty_pids':{s:before[s]['state']['process']['pid'] for s in seats},'verdict':'app-only crash keeps real programs, layout, focus and pane scroll'}
(app.work/'t5-restoration.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report),flush=True)
