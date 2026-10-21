#!/usr/bin/env python3
"""Bounded PTY fixture: retain only exact OSC 7501 frames, never terminal text."""
import argparse, errno, fcntl, hashlib, json, os, pty, re, select, signal, struct, termios, time, tty
from pathlib import Path

PROBE=b"\x1b]7501;?\x1b\\"
ST=b"\x1b\\"
ROOT=Path(__file__).resolve().parents[2]
RUNS=ROOT/".kb/runs"
PROMPTS={
 "claude": "This is the operator-authorized OSC 7501 live-wire experiment, not implementation work. Use the Agent tool exactly once with subagent_type wire_probe, description OSC7501 child fixture, prompt 'Reply exactly CAPTURE_CHILD_DONE. Use no tools.', and run_in_background false. Wait for that child, then reply exactly CAPTURE_PARENT_DONE. Do not read files, run commands, edit anything, or send messages. One child only.",
 "codex": "This is an operator-authorized terminal-protocol fixture. Do not use tools, read files, send messages or modify anything. Think briefly, then reply exactly CAPTURE_CODEX_DONE."
}

def command(agent):
 if agent=="claude":
  agents={"wire_probe":{"description":"Protocol-capture fixture child; no implementation or tools.","prompt":"You are a fixture subagent in an operator-authorized OSC capture. Reply exactly CAPTURE_CHILD_DONE. Do not use tools, read files, write anything or send messages.","tools":[]}}
  return ["/opt/homebrew/bin/claude",PROMPTS[agent],"--tools","Agent","--allowedTools","Agent","--agents",json.dumps(agents),"--setting-sources","","--settings",json.dumps({"disableAllHooks":True}),"--strict-mcp-config","--mcp-config",json.dumps({"mcpServers":{}}),"--disable-slash-commands","--no-chrome"]
 return ["/opt/homebrew/bin/codex","--no-alt-screen","--no-daemon","--disable","hooks","--sandbox","read-only","--ask-for-approval","never",PROMPTS[agent]]

class Filter:
 def __init__(self,on_osc,respond):
  self.buf=b""; self.on_osc=on_osc; self.respond=respond; self.other={}
 def feed(self,data):
  self.buf+=data
  while self.buf:
   positions=[x for x in (self.buf.find(b"\x1b"),) if x>=0]
   if not positions: self.buf=b""; return
   self.buf=self.buf[min(positions):]
   if self.buf.startswith(b"\x9d") or self.buf.startswith(b"\x1b]"):
    start=1 if self.buf[0]==0x9d else 2
    ends=[(x,n) for x,n in ((self.buf.find(b"\x07",start),1),(self.buf.find(ST,start),2)) if x>=0]
    if not ends:
     if len(self.buf)>65536: self.buf=b""
     return
    end,size=min(ends)
    raw=self.buf[:end+size]; payload=self.buf[start:end]; self.buf=self.buf[end+size:]
    code,sep,value=payload.partition(b";")
    if code==b"7501": self.on_osc(raw,value)
    elif code in (b"10",b"11") and value==b"?":
     rgb=b"eeee/eeee/eeee" if code==b"10" else b"1111/1111/1111"
     self.respond(b"\x1b]"+code+b";rgb:"+rgb+ST)
    else:
     key=code.decode("ascii","ignore")
     if key in ("9","21337"): self.other[key]=self.other.get(key,0)+1
   elif self.buf.startswith(b"\x1b["):
    match=re.match(rb"\x1b\[([0-?]*[ -/]*[@-~])",self.buf)
    if not match:
     if len(self.buf)>256: self.buf=self.buf[2:]
     else: return
     continue
    c=match.group(1); self.buf=self.buf[match.end():]
    if c in (b"6n",b"?6n"): self.respond(b"\x1b[1;1R" if c==b"6n" else b"\x1b[?1;1R")
    elif c==b"c": self.respond(b"\x1b[?1;2c")
    elif c in (b">c",b">0c"): self.respond(b"\x1b[>0;1;0c")
    elif c==b">q": self.respond(b"\x1bP>|bboxy-osc7501-capture(1)"+ST)
    elif c==b"?u": self.respond(b"\x1b[?0u")
    elif c in (b"?2026$p",b"?1016$p"):
     mode=c[1:-2]; self.respond(b"\x1b[?"+mode+b";2$y")
   elif self.buf.startswith(b"\x1bP"):
    end=self.buf.find(ST,2)
    if end<0:
     if len(self.buf)>65536: self.buf=b""
     return
    self.buf=self.buf[end+2:]
   elif len(self.buf)==1: return
   else: self.buf=self.buf[2:]

def self_test():
 frames=[]; replies=[]; f=Filter(lambda raw,payload:frames.append((raw,payload)),replies.append)
 raw=PROBE+"private ordinary text ❯\n".encode()+b"\x1b[31m"+b"\x1b]7501;state=working:app=fixture:id=a:title=VGVzdA==\x07"+b"\x1b[6n"
 for byte in raw: f.feed(bytes([byte]))
 assert len(frames)==2 and b"private" not in b"".join(x[0] for x in frames)
 assert replies==[b"\x1b[1;1R"]
 print(json.dumps({"parser_self_test":"passed","chunk_size":1,"frames":2,"plaintext_retained":False}))

def run(agent,timeout,term_program):
 binary=RUNS/f"osc7501-{agent}.bin"; meta=RUNS/f"osc7501-{agent}-timing.json"
 assert not binary.exists() and not meta.exists(),"Refusing to overwrite earlier evidence"
 argv=command(agent); env=os.environ.copy()
 for key in list(env):
  if key in {"TERM_SESSION_ID","ITERM_SESSION_ID","TMUX","TMUX_PANE","IT2_SOCKET","CLAUDECODE","CLAUDE_PID","CLAUDE_CODE_SESSION_ID","CLAUDE_CODE_MESSAGING_SOCKET","CLAUDE_CODE_MESSAGING_TOKEN","CLAUDE_CODE_CHILD_SESSION","CODEX_THREAD_ID","CODEX_SESSION_ID","CODEX_CI","CODEX_TOOL_CALL_ID","CODEX_VERSION"} or key.startswith("ICQ_"):
   env.pop(key,None)
 env.update(TERM="xterm-256color",COLORTERM="truecolor",TERM_PROGRAM=term_program,TERM_PROGRAM_VERSION=("3.7.6" if term_program=="iTerm.app" else "capture-1"))
 start=time.monotonic_ns(); start_utc=time.strftime("%Y-%m-%dT%H:%M:%SZ",time.gmtime()); records=[]; events=[]; probes=0; report_count=0; task_ids=set(); worked=False; finished_at=None; shutdown_at=None; reaped=None; total=0; sent=0; last_state=None
 pid,master=pty.fork()
 if pid==0:
  os.chdir(ROOT)
  tty.setraw(0)
  os.execvpe(argv[0],argv,env)
 os.set_blocking(master,False)
 fcntl.ioctl(master,termios.TIOCSWINSZ,struct.pack("HHHH",36,120,0,0))
 out=binary.open("xb")
 def elapsed(): return round((time.monotonic_ns()-start)/1000000,3)
 def respond(data):
  nonlocal sent
  try: sent+=os.write(master,data)
  except OSError: pass
 def on_osc(raw,payload):
  nonlocal probes,report_count,worked,finished_at,last_state
  if len(raw)>4096: return
  fields={}
  if payload!=b"?":
   for pair in payload.split(b":"):
    k,eq,v=pair.partition(b"=")
    if eq: fields[k.decode("ascii","ignore")]=v.decode("ascii","replace")
   if fields.get("state") not in {"idle","working","blocked","done","error","clear"}: return
  offset=out.tell(); out.write(raw); out.flush()
  records.append({"direction":"harness_to_terminal","t_ms":elapsed(),"offset":offset,"length":len(raw)})
  if payload==b"?":
   probes+=1; respond(PROBE)
   events.append({"event":"probe_echo","t_ms":elapsed(),"hex":PROBE.hex()})
   print(json.dumps({"agent":agent,"event":"probe_echo","t_ms":elapsed()}),flush=True)
   return
  report_count+=1
  state=fields["state"]
  if "id" in fields: task_ids.add(fields["id"])
  else:
   if state=="working": worked=True; finished_at=None
   elif worked and state in {"done","idle"}: finished_at=time.monotonic() if finished_at is None else finished_at
   last_state=state
  if report_count<15 or "id" in fields:
   print(json.dumps({"agent":agent,"event":"report","t_ms":elapsed(),"state":state,"has_task_id":"id" in fields}),flush=True)
 parser=Filter(on_osc,respond)
 recent_text=""; diagnostics=set()
 diagnostic_patterns={"config_error":"error:","trust_prompt":"trust the files", "trust_question":"trust this","directory_trust":"directory","hook_trust":"hooks","yes_option":"yes,","no_option":"no,","proceed_option":"proceed", "auth_prompt":"sign in", "update_prompt":"update available", "press_enter":"press enter", "codex_fixture_text":"CAPTURE_CODEX_DONE","hook_notice":"hook","trust_notice":"trust","migration_notice":"migrat","update_notice":"update","confirmation_notice":"confirm","mcp_notice":"mcp","error_notice":"error","model_notice":"model","terminal_notice":"terminal","question_notice":"?","upgrade_notice":"upgrade","config_warning":"config.toml","trusted_project_warning":"trusted project","initial_model_dialog":"new model","new_model_dialog":"newest model","try_model_dialog":"try the","escape_hint":"esc","enter_hint":"enter","continue_hint":"continue","select_hint":"select","proceed_hint":"proceed","untrusted_warning":"untrusted","sign_in_hint":"sign","blocked_prompt":"approval"}
 try:
  while True:
   now=time.monotonic(); duration=(time.monotonic_ns()-start)/1e9
   if shutdown_at is None and finished_at is not None and now-finished_at>3 and (agent=="codex" or task_ids):
    respond(b"\x04"); shutdown_at=now
    events.append({"event":"graceful_exit_requested","t_ms":elapsed()})
   if shutdown_at is None and duration>=timeout:
    shutdown_at=now; events.append({"event":"capture_deadline","t_ms":elapsed()})
    os.killpg(pid,signal.SIGTERM)
   if shutdown_at is not None and now-shutdown_at>5:
    try: os.killpg(pid,signal.SIGTERM)
    except ProcessLookupError: pass
   if shutdown_at is not None and now-shutdown_at>8:
    try: os.killpg(pid,signal.SIGKILL)
    except ProcessLookupError: pass
   ready,_,_=select.select([master],[],[],0.05)
   if ready:
    try: data=os.read(master,65536)
    except OSError as e:
     if e.errno==errno.EIO: break
     if e.errno in (errno.EAGAIN,errno.EWOULDBLOCK): continue
     raise
    if not data: break
    total+=len(data); parser.feed(data)
    recent_text=(recent_text+data.decode("utf-8","replace"))[-32768:]
    clean=re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]","",recent_text).lower()
    restricted=re.search(r"(?<![0-9])([1-9])[.)]\s+open restricted",clean)
    if agent=="codex" and restricted and not any(e["event"]=="restricted_startup_choice" for e in events):
     respond(restricted.group(1).encode()+b"\r")
     events.append({"event":"restricted_startup_choice","t_ms":elapsed(),"option_number":restricted.group(1)})
     print(json.dumps({"agent":agent,"event":"restricted_startup_choice","t_ms":elapsed(),"option_number":restricted.group(1)}),flush=True)
    for label,pattern in diagnostic_patterns.items():
     if pattern.lower() in clean and label not in diagnostics:
      diagnostics.add(label); print(json.dumps({"agent":agent,"event":"diagnostic_flag","flag":label,"t_ms":elapsed()}),flush=True)
   waited,status=os.waitpid(pid,os.WNOHANG)
   if waited:
    reaped=status
    # Drain output after exit until the PTY returns EOF/EIO.
    while True:
     try:
      data=os.read(master,65536)
      if not data: break
      total+=len(data); parser.feed(data)
     except OSError: break
    break
 finally:
  out.close(); os.close(master)
  if reaped is None:
   try:
    waited,status=os.waitpid(pid,os.WNOHANG)
    if not waited:
     try: os.killpg(pid,signal.SIGTERM)
     except ProcessLookupError: pass
     waited,status=os.waitpid(pid,0)
    reaped=status
   except ChildProcessError: pass
  content=binary.read_bytes()
  static_matches=[]
  if agent=="codex" and report_count==0:
   image=Path(argv[0]).resolve().read_bytes()
   display=re.sub(r"\x1b\[[0-?]*[ -/]*[@-~]","",recent_text)
   for text in re.findall(r"[ -~]{20,}",display):
    text=text.strip().encode("ascii")
    for size in (60,40,24,12):
     for at in range(0,max(1,len(text)-size+1),4):
      fragment=text[at:at+size]
      if len(fragment)<size or sum(chr(c).isalnum() for c in fragment)<size//2: continue
      position=image.find(fragment)
      if position>=0 and {"offset":position,"length":size} not in static_matches:
       static_matches.append({"offset":position,"length":size})
       break
     else: continue
     break
    if len(static_matches)>=12: break
  result={"agent":agent,"start_utc":start_utc,"duration_ms":elapsed(),"pty_columns":120,"pty_rows":36,"term":"xterm-256color","term_program":term_program,"probe_echo_hex":PROBE.hex(),"argv":argv,"records":records,"events":events,"probe_count":probes,"report_count":report_count,"task_ids":sorted(task_ids),"last_main_state":last_state,"observed_working":worked,"pty_bytes_read_and_discarded_except_frames":total,"retained_bytes":len(content),"sha256":hashlib.sha256(content).hexdigest(),"exit_code":os.waitstatus_to_exitcode(reaped) if reaped is not None else None,"other_osc_counts":parser.other,"terminal_reply_bytes":sent,"diagnostic_flags":sorted(diagnostics),"startup_static_matches":static_matches,"startup_control_words":[word for word in re.findall(r"[a-z]+",clean) if word in {"config","settings","files","folders","load","project","trusted","models","update","select","new","try","keep","esc","enter","continue","next","beta","codex","trust","welcome","plan","subscription","limit","upgrade","skip","login","sign","in","out","approval","read","only","error","setup","hook","hooks"}][:100]}
  meta.write_text(json.dumps(result,indent=2)+"\n")
  print(json.dumps({key:result[key] for key in ("agent","probe_count","report_count","task_ids","retained_bytes","exit_code","duration_ms","other_osc_counts")}),flush=True)

if __name__=="__main__":
 ap=argparse.ArgumentParser(); ap.add_argument("--agent",choices=("claude","codex")); ap.add_argument("--timeout",type=int,default=120); ap.add_argument("--self-test",action="store_true"); ap.add_argument("--term-program",choices=("bboxy450-term","iTerm.app"),default="bboxy450-term"); args=ap.parse_args()
 if args.self_test: self_test()
 elif args.agent: run(args.agent,args.timeout,args.term_program)
 else: ap.error("--agent or --self-test required")
