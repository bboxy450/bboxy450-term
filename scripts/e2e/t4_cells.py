"""T4: actual PTY style/colour/wide-grid and cursor captures."""
import json
import math
import os
from pathlib import Path
import shlex
import sys
import termios
import time
import tty

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts/e2e'))
from ui import App, pane, require

BASE = [7, 17, 27]
WHITE = [240, 240, 240]
# bbt-core term/mod.rs:54, the public Screen.modes bitfield.
SHOW_CURSOR = 1
COLOURS = [
    ('31', '41', {'Named': 'Red'}, [204, 0, 0]),
    ('32', '42', {'Named': 'Green'}, [78, 154, 6]),
    ('34', '44', {'Named': 'Blue'}, [52, 101, 164]),
    ('38;5;196', '48;5;196', {'Indexed': 196}, [255, 0, 0]),
    ('38;2;17;203;145', '48;2;17;203;145', {'Spec': {'r':17,'g':203,'b':145}}, [17, 203, 145]),
]
NORMAL = '\x1b[0m\x1b[38;2;240;240;240m\x1b[48;2;7;17;27m'


def program(directory):
    require(os.isatty(0) and os.isatty(1), 'real PTY required')
    saved = termios.tcgetattr(0)
    directory = Path(directory)
    count = 0
    def cursor(kind):
        nonlocal count
        seq = {'b':'\x1b[?25h\x1b[2 q', 'u':'\x1b[?25h\x1b[4 q',
               's':'\x1b[?25h\x1b[6 q', 'h':'\x1b[?25l\x1b[2 q'}[kind]
        os.write(1, (NORMAL + '\x1b[6;25H' + seq).encode())
        count += 1
        p=directory/'t4-state.tmp';p.write_text(json.dumps({'phase':count,'cursor':kind})+'\n')
        p.replace(directory/'t4-state.json')
    try:
        tty.setraw(0)
        text = NORMAL + '\x1b[?7l\x1b[2J\x1b[H'
        text += 'M ' + '\x1b[1mM' + NORMAL + ' ' + '\x1b[3mM' + NORMAL + ' ' + '\x1b[4mM' + NORMAL
        text += '\x1b[2;1H'
        for fg,bg,_,_ in COLOURS:
            text += NORMAL+'\x1b['+fg+'m█'+NORMAL+'\x1b['+bg+'m '
        text += NORMAL+'\x1b[3;1H漢x 😀y'
        text += '\x1b[4;1H漢\x1b[4;3Hx\x1b[4;5H😀\x1b[4;7Hy'
        data=memoryview(text.encode())
        while data:data=data[os.write(1,data):]
        cursor('h')
        while True:
            command=os.read(0,128)
            if not command or b'q' in command:break
            for value in command.decode():
                if value in 'bush':cursor(value)
    finally:termios.tcsetattr(0,termios.TCSANOW,saved)


def observe(app,seat,phase,previous_sequence):
    deadline=time.monotonic()+45
    while True:
        path=app.work/'t4-state.json'
        state=json.loads(path.read_text()) if path.exists() else None
        screen=app.ctl('seat','screen',seat,'--cells')
        if state and state['phase']>phase:
            layout=app.ctl('ui','layout');p=pane(layout,seat);source=p.get('rendered')
            expected={'b':'Block','u':'Underline','s':'Beam','h':'Block'}[state['cursor']]
            if (source and source.get('pixel_size') and screen['sequence']>previous_sequence and
                source['sequence']>=screen['sequence'] and source['lines'][:4]==screen['rows'][:4] and
                screen['rows'][0]=='M M M M' and screen['rows'][2]==screen['rows'][3]=='漢x 😀y' and
                screen['cursor_style']['shape']==expected and
                bool(screen['modes'] & SHOW_CURSOR) == (state['cursor'] != 'h')):
                return layout,screen,state
        text='\n'.join(screen['rows'])
        require('Traceback (most recent call last)' not in text,'fixture crash: '+text)
        require(time.monotonic()<deadline,'T4 did not become observable: '+str(state))
        time.sleep(.01)


def capture(app,seat,layout,label,screen):
    from rim_clock import png,region
    path=app.capture('window',label,layout['frame_seq']);image=png(path)
    (app.work/(label+'-source.json')).write_text(json.dumps({
        'layout':layout,'screen':screen,
        'capture':json.loads((app.work/(label+'.jsonl')).read_text())},indent=2)+'\n')
    p=pane(layout,seat);x,y,_,_=p['rect'];w,h=p['rendered']['pixel_size']
    require(list(image[:2])==list(layout['size']),'expected headless 1:1 frame')
    require(w%p['cols']==h%p['rows']==0,'non-integral cell size')
    cw,ch=w//p['cols'],h//p['rows'];x,y=math.floor(x+.5),math.floor(y+.5)
    def cell(col,row,columns=1):return region(image,[x+col*cw,y+row*ch,x+(col+columns)*cw,y+(row+1)*ch])
    return cell,cw,ch


def expand_single_width(runs):
    require(all(not run['flags'] & 32 for run in runs),'single-width control row contains a wide run')
    return [dict(run,text=c) for run in runs for c in run['text']]


def main():
    app=App();app.ctl('ui','clock','freeze');seat=app.spawn('T4 actual cells')
    app.line(seat,shlex.join(['exec','python3','-I','-B',str(Path(__file__).resolve()),'--program',str(app.work)]))
    layout,screen,state=observe(app,seat,0,0)
    cells=expand_single_width(screen['cells'][0])
    require(cells[0]['flags']&14==0,'normal M gained a style')
    for column,flag in ((2,2),(4,4),(6,8)):require(cells[column]['flags']&flag,'missing style flag')
    colours=expand_single_width(screen['cells'][1])
    for n,(_,_,wire,_) in enumerate(COLOURS):
        require(colours[2*n]['fg']==wire and colours[2*n+1]['bg']==wire,'wrong parsed colour')
    for rune in ('漢','😀'):
        require(next(run for run in screen['cells'][2] if rune in run['text'])['flags']&32,'missing wide flag')
    cell,cw,ch=capture(app,seat,layout,'t4-cells',screen)
    normal,bold,italic,underline=[cell(c,0) for c in (0,2,4,6)]
    mass=lambda data:sum(max(0,data[i+c]-BASE[c]) for i in range(0,len(data),4) for c in range(3))
    stats={'normal_mass':mass(normal),'bold_mass':mass(bold),'italic_different_pixels':sum(normal[i:i+4]!=italic[i:i+4] for i in range(0,len(normal),4))}
    require(stats['bold_mass']>stats['normal_mass'],'bold did not gain ink')
    require(stats['italic_different_pixels']>0,'italic is the normal glyph')
    extra=[(i//4%cw,i//4//cw) for i in range(0,len(normal),4) if normal[i:i+4]!=underline[i:i+4]]
    require(extra and len(set(y for _,y in extra))<=3 and min(y for _,y in extra)>=ch//2 and
            max(x for x,_ in extra)-min(x for x,_ in extra)+1>=cw*.75,'underline is not a lower horizontal rule')
    stats['underline_pixels']=len(extra)
    stats['underline_rows']=sorted(set(y for _,y in extra))
    colour_stats=[]
    for n,(_,_,wire,rgb) in enumerate(COLOURS):
        foreground=cell(2*n,1);background=cell(2*n+1,1)
        matches=lambda data:sum(all(abs(data[i+c]-rgb[c])<=2 for c in range(3)) and data[i+3]==255 for i in range(0,len(data),4))
        fg,bg=matches(foreground),matches(background)
        require(fg>0 and bg==cw*ch,'missing or partial rendered fg/bg colour '+str(rgb))
        colour_stats.append({'wire':wire,'rgb':rgb,'foreground_pixels':fg,'background_pixels':bg})
    require(cell(0,2,8)==cell(0,3,8),'automatic wide placement differs from explicit grid columns')
    for col in (0,4):
        data=cell(col,2,2)
        for half in (cell(col,2),cell(col+1,2)):
            require(any(any(abs(half[i+c]-BASE[c])>15 for c in range(3)) for i in range(0,len(half),4)),'wide glyph has no ink in both cells')
    cursor_stats=[]
    hidden=cell(24,5)
    for kind in ('b','u','s'):
        app.ctl('seat','type',seat,kind)
        layout,screen,state=observe(app,seat,state['phase'],screen['sequence'])
        current,_,_=capture(app,seat,layout,'t4-cursor-'+kind,screen);visible=current(24,5)
        points=[(i//4%cw,i//4//cw) for i in range(0,len(hidden),4) if hidden[i:i+4]!=visible[i:i+4]]
        require(points,'cursor did not appear')
        width=max(x for x,_ in points)-min(x for x,_ in points)+1;height=max(y for _,y in points)-min(y for _,y in points)+1
        if kind=='b':require(len(points)>=cw*ch*.75,'block is not solid')
        elif kind=='u':require(width>=cw*.75 and height<=3 and min(y for _,y in points)>=ch-3,'underline cursor has wrong geometry')
        else:require(width<=3 and height>=ch*.75 and max(x for x,_ in points)<=2,'beam cursor has wrong geometry')
        cursor_stats.append({'kind':kind,'pixels':len(points),'width':width,'height':height})
        app.ctl('seat','type',seat,'h')
        layout,screen,state=observe(app,seat,state['phase'],screen['sequence'])
        current,_,_=capture(app,seat,layout,'t4-hidden-'+kind,screen);hidden=current(24,5)
    result={'styles':stats,'colours':colour_stats,'wide':{'automatic_equals_explicit':True,'han_cells':2,'emoji_cells':2,'both_halves_have_ink':True},'cursor':cursor_stats,'cell':[cw,ch]}
    (app.work/'t4-observations.json').write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps(result))

if __name__=='__main__':
    if len(sys.argv)==3 and sys.argv[1]=='--program':program(sys.argv[2])
    else:main()
