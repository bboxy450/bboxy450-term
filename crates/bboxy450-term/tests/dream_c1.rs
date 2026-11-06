// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private headless app and actual ctl process boundary; no operator or inference.
//! Adapted from this project's tests/swarm_workdir.rs private account/owner fixture:
//! https://github.com/bboxy450/bboxy450-term/blob/main/crates/bboxy450-term/tests/swarm_workdir.rs
#![cfg(target_os = "macos")]
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended app process fixture"
)]
use serde_json::{Value, json};
use std::{
    fs,
    os::unix::fs::{DirBuilderExt, PermissionsExt},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    time::{Duration, Instant},
};
const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");
struct Fixture {
    directory: tempfile::TempDir,
    root: PathBuf,
    app: Option<Child>,
    fault_mode: Option<String>,
}
impl Fixture {
    fn new() -> Self {
        // The producer binds its canonical HOME/run session socket before its
        // host alias. Darwin's104-byte sockaddr_un bound excludes macOS's long
        // default per-user temporary directory; use our own fresh short root.
        let directory = tempfile::Builder::new()
            .prefix("dc1-")
            .tempdir_in("/tmp")
            .unwrap();
        let root = directory.path().canonicalize().unwrap();
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
        for name in [
            "home",
            "repository",
            "bin",
            "seat-bin",
            "repository/.bboxy",
            "home/.codex",
            "home/.claude",
            "home/.bboxy450-harness",
            "home/.bboxy450-harness/profiles",
            "home/.bboxy450-harness/run",
            "home/.bboxy450-harness/sessions",
        ] {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(root.join(name))
                .unwrap();
        }
        private_bytes(&root.join("home/.zshrc"), b"PROMPT='%# '\n");
        Self {
            directory,
            root,
            app: None,
            fault_mode: None,
        }
    }
    fn scripted() -> Self {
        let f = Self::new();
        let runner = PathBuf::from(
            std::env::var_os("DREAM_C1_RUNNER")
                .expect("joined C1 test requires the deliberately pinned absolute real runner"),
        );
        assert!(
            runner.is_absolute() && runner.is_file(),
            "missing real C1 producer"
        );
        let seat_runner = PathBuf::from(
            std::env::var_os("DREAM_C1_SEAT_RUNNER")
                .expect("C1 source fixtures require an explicit feature-built seat runner"),
        );
        assert!(
            seat_runner.is_absolute() && seat_runner.is_file(),
            "missing real source-seat producer"
        );
        fs::copy(BIN, f.root.join("bin/bboxy450-term")).unwrap();
        // C1 starts the exact ordinary worker directly. The scripted managed
        // seat launcher uses PATH separately and never prefixes a C1 birth.
        fs::copy(&runner, f.root.join("bin/bboxy450-harness")).unwrap();
        let sse = f.root.join("scripted.sse");
        // Sixteen bounded deltas make a 128 KiB completed item, below the canonical
        // 256 KiB envelope limit. Sixteen real turns below produce a multi-MiB
        // durable source, leaving a measured opportunity to hold its real worker.
        let delta = "private C1 data ".repeat(512);
        let mut response = String::new();
        for _ in 0..16 {
            response.push_str(&format!(
                "data: {}\n\n",
                json!({"choices":[{"index":0,"delta":{"content":delta},"finish_reason":null}]})
            ));
        }
        response.push_str(&format!(
            "data: {}\n\ndata: [DONE]\n\n",
            json!({"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]})
        ));
        private_bytes(&sse, response.as_bytes());
        let profiles = f.root.join("home/.bboxy450-harness/profiles");
        private_bytes(&profiles.join("private-c1.md"), b"---\nname: private-c1\nmodel: scripted\nendpoint: http://127.0.0.1:1/v1\non_unattended: deny\nauto_compact: false\n---\n");
        // Only managed seats use this deterministic fixture launcher. Runtime's
        // absolute sibling worker remains the ordinary executable above.
        let shim = f.root.join("seat-bin/bboxy450-harness");
        fs::write(&shim,format!("#!/usr/bin/python3\nimport json,os,sys\nworker={worker}\nseat={seat}\nargs=sys.argv[1:]\nreal=worker if args[:2]==['dream','inspect-c1'] else seat\nif args[:2]!=['dream','inspect-c1'] and 'BBOXY_SEAT_ID' in os.environ:\n  keys=['BBOXY_SOCK','BBOXY_SEAT_ID','BBOXY_GENERATION','BBOXY_CAPABILITY_FILE']\n  path={root}+'/admission-'+os.environ['BBOXY_SEAT_ID']+'.json'\n  fd=os.open(path,os.O_WRONLY|os.O_CREAT|os.O_EXCL,0o600)\n  os.write(fd,json.dumps({{k:os.environ[k] for k in keys}}).encode())\n  os.close(fd)\n  args.extend(['--profile','private-c1'])\n  for _ in range(32): args.extend(['--scripted',{script}])\nif '--dream-c1-fixture-next-session' in args:\n  allowed={{'HOME','ZDOTDIR','CODEX_HOME','CLAUDE_CONFIG_DIR','PATH','TERM','LANG','LC_ALL','TMPDIR','TZ','BBOXY_SEAT_ID','BBOXY_GENERATION','BBOXY_ROLE','BBOXY_SOCK','BBOXY_CAPABILITY_FILE','TERM_PROGRAM','TERM_PROGRAM_VERSION'}}\n  os.execve(real,[real,*args],{{k:v for k,v in os.environ.items() if k in allowed}})\nos.execv(real,[real,*args])\n",
            worker=serde_json::to_string(&runner).unwrap(),seat=serde_json::to_string(&seat_runner).unwrap(),script=serde_json::to_string(&sse).unwrap(),root=serde_json::to_string(&f.root).unwrap())).unwrap();
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o700)).unwrap();
        let git = Command::new("/usr/bin/git")
            .args(["init", "-q"])
            .arg(f.root.join("repository"))
            .env_clear()
            .env("HOME", f.root.join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(
            git.status.success(),
            "private source repository setup failed"
        );
        f
    }
    fn scripted_fault(mode: &str) -> Self {
        assert!(matches!(mode, "hold-source" | "omit-rest" | "hot-owner"));
        let mut f = Self::scripted();
        f.fault_mode = Some(mode.to_owned());
        f
    }
    fn scripted_progress_flood() -> Self {
        let f = Self::scripted();
        let corpus = PathBuf::from(
            std::env::var_os("DREAM_C1_FIXTURES")
                .expect("approved producer corpus required for protocol fake templates"),
        );
        let template = |name: &str| {
            let raw = fs::read(corpus.join(format!("valid/{name}.json"))).unwrap();
            serde_json::to_string(&String::from_utf8(raw).unwrap()).unwrap()
        };
        // Adapted from this project's wake-up transport Script fixture. These
        // canonical templates define framing only; this child is explicitly a
        // protocol fake, never production-worker/corpus conformance evidence.
        let fault = r#"
def protocol_flood(job):
  import collections,select,time
  os.nice(19)
  hello=json.loads(__HELLO__)
  progress=json.loads(__PROGRESS__)
  done=json.loads(__DONE__)
  def frame(value):
    raw=json.dumps(value,ensure_ascii=False,sort_keys=True,separators=(',',':')).encode()
    assert len(raw)<=262144
    return raw+b'\n'
  pending=bytearray(frame(hello))
  incoming=bytearray()
  started=None
  seq=0
  generated=0
  completed_progress=0
  progress_ends=collections.deque()
  sent=0
  max_pending=len(pending)
  cancellation=False
  eof=False
  until=time.monotonic()+8
  os.set_blocking(0,False)
  os.set_blocking(1,False)
  while time.monotonic()<until:
    readable,writable,_=select.select([0],[1] if pending else [],[],0.001)
    if readable:
      chunk=os.read(0,4096)
      if not chunk:
        eof=True
        break
      incoming.extend(chunk)
      assert len(incoming)<=262145
      while b'\n' in incoming:
        length=incoming.index(b'\n')
        assert 0<length<=262144
        value=json.loads(incoming[:length])
        del incoming[:length+1]
        if value['type']=='start':
          assert started is None
          started=value
        elif value['type']=='cancel':
          assert started and value['run_id']==started['run_id'] and value['nonce']==started['nonce']
          if not cancellation:
            cancellation=True
            seq+=1
            done.update(run_id=started['run_id'],nonce=started['nonce'],seq=seq,outcome='cancelled',pages=0,result_sha256=None,error={'code':'cancelled','source':None})
            pending.extend(frame(done))
        elif value['type']=='activity':
          assert started and value['run_id']==started['run_id'] and value['nonce']==started['nonce']
        else: raise AssertionError('unexpected worker input')
    if started and not cancellation:
      for _ in range(32):
        candidate=dict(progress,run_id=started['run_id'],nonce=started['nonce'],seq=seq+1,phase='replay')
        encoded=frame(candidate)
        if len(pending)+len(encoded)>65536: break
        pending.extend(encoded)
        progress_ends.append(sent+len(pending))
        seq+=1
        generated+=1
    max_pending=max(max_pending,len(pending))
    assert max_pending<=66048
    if writable and pending:
      try:
        count=os.write(1,pending[:4096])
        sent+=count
        del pending[:count]
        while progress_ends and progress_ends[0]<=sent:
          progress_ends.popleft()
          completed_progress+=1
      except BlockingIOError: pass
      except BrokenPipeError:
        eof=True
        break
    if cancellation and not pending: break
  receipt={'protocol_fake':True,'generated_progress_frames':generated,'fully_written_progress_frames':completed_progress,'sent_framed_bytes':sent,'max_pending_bytes':max_pending,'cancel_received':cancellation,'eof_received':eof,'bounded_lifetime_seconds':8}
  fd=os.open(os.path.join(job,'protocol-flood.json'),os.O_WRONLY|os.O_CREAT|os.O_EXCL|os.O_NOFOLLOW,0o600)
  os.write(fd,json.dumps(receipt,sort_keys=True,separators=(',',':')).encode())
  os.close(fd)
  sys.exit(0 if cancellation or eof else 3)
if args[:2]==['dream','inspect-c1']:
  job=args[args.index('--job-dir')+1]
  name=os.path.basename(job)
  if len(name)==26 and set(name)<=set('0123456789ABCDEFGHJKMNPQRSTVWXYZ'):
    protocol_flood(job)
"#
        .replace("__HELLO__", &template("hello"))
        .replace("__PROGRESS__", &template("progress"))
        .replace("__DONE__", &template("done"));
        let path = f.root.join("bin/bboxy450-harness");
        let worker = PathBuf::from(std::env::var_os("DREAM_C1_RUNNER").unwrap());
        let code = format!(
            "#!/usr/bin/python3\nimport json,os,sys\nworker={}\nargs=sys.argv[1:]\nos.execv(worker,[worker,*args])\n",
            serde_json::to_string(&worker).unwrap()
        );
        fs::write(
            path,
            code.replace(
                "args=sys.argv[1:]\n",
                &format!("args=sys.argv[1:]\n{fault}\n"),
            ),
        )
        .unwrap();
        f
    }
    fn scripted_blob(bytes: u64) -> Self {
        let f = Self::scripted();
        let program = format!(
            "import os\nleft={bytes}\nchunk=b'z'*65536\nwhile left:\n n=os.write(1,chunk[:min(left,len(chunk))]); left-=n\n"
        );
        let arguments = json!({"program":"/usr/bin/nice","args":["-n","10","/usr/bin/python3","-I","-c",program],"timeout":30}).to_string();
        let first = sse(
            json!({"tool_calls":[{"index":0,"id":"private-owned-source-generator","type":"function","function":{"name":"exec","arguments":arguments}}]}),
            "tool_calls",
        );
        let done = sse(
            json!({"content":"Private bounded source complete."}),
            "stop",
        );
        f.provision_rounds(vec![first, done]);
        f
    }
    fn provision_rounds(&self, rounds: Vec<String>) {
        let mut args = Vec::new();
        for (index, round) in rounds.into_iter().enumerate() {
            let path = self.root.join(format!("source-round-{index}.sse"));
            private_bytes(&path, round.as_bytes());
            args.push("--scripted".to_owned());
            args.push(path.to_str().unwrap().to_owned());
        }
        let path = self.root.join("seat-bin/bboxy450-harness");
        let code = fs::read_to_string(&path).unwrap();
        let original = format!(
            "for _ in range(32): args.extend(['--scripted',{}])",
            serde_json::to_string(&self.root.join("scripted.sse")).unwrap()
        );
        assert!(code.contains(&original));
        fs::write(
            path,
            code.replace(
                &original,
                &format!("args.extend({})", serde_json::to_string(&args).unwrap()),
            ),
        )
        .unwrap();
    }
    fn scripted_retained(next_session: &str) -> Self {
        let f = Self::scripted();
        let program = "import os\nleft=8*1024*1024\nchunk=b'z'*65536\nwhile left:\n n=os.write(1,chunk[:min(left,len(chunk))]); left-=n\n";
        let args=json!({"program":"/usr/bin/nice","args":["-n","10","/usr/bin/python3","-I","-c",program],"timeout":30}).to_string();
        let blob = sse(
            json!({"tool_calls":[{"index":0,"id":"private-retained-source-blob","type":"function","function":{"name":"exec","arguments":args}}]}),
            "tool_calls",
        );
        let done_a = sse(
            json!({"content":"A is ready for authoritative close."}),
            "stop",
        );
        let sleep = json!({"program":"/bin/sleep","args":["1"],"timeout":3}).to_string();
        let busy_b = sse(
            json!({"tool_calls":[{"index":0,"id":"private-retained-foreground","type":"function","function":{"name":"exec","arguments":sleep}}]}),
            "tool_calls",
        );
        let done_b = sse(json!({"content":"B foreground complete."}), "stop");
        f.provision_rounds(vec![blob, done_a, busy_b, done_b]);
        let path = f.root.join("seat-bin/bboxy450-harness");
        let code = fs::read_to_string(&path).unwrap();
        let extra = serde_json::to_string(&vec![
            "--dream-c1-fixture-next-session".to_owned(),
            next_session.to_owned(),
            "--sessions".to_owned(),
            f.root
                .join("home/.bboxy450-harness/sessions")
                .to_str()
                .unwrap()
                .to_owned(),
        ])
        .unwrap();
        let anchor = "  args.extend(['--profile','private-c1'])";
        assert!(code.contains(anchor));
        fs::write(path,code.replace(anchor,&format!("{anchor}\n  args.extend({extra})\n  if '--json' not in args: args.append('--json')"))).unwrap();
        f
    }
    fn command(&self) -> Command {
        let private = self.root.join("bin/bboxy450-term");
        let mut command = Command::new(if private.is_file() {
            private
        } else {
            PathBuf::from(BIN)
        });
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("ZDOTDIR", self.root.join("home"))
            .env("CODEX_HOME", self.root.join("home/.codex"))
            .env("CLAUDE_CONFIG_DIR", self.root.join("home/.claude"))
            .env(
                "PATH",
                format!(
                    "{}:{}:/usr/bin:/bin:/usr/sbin:/sbin",
                    self.root.join("seat-bin").display(),
                    self.root.join("bin").display()
                ),
            )
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }
    fn raw(&self, args: &[&str]) -> Output {
        self.command()
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(self.root.join("run"))
            .output()
            .unwrap()
    }
    fn ctl(&self, args: &[&str]) -> Value {
        let output = self.raw(args);
        assert!(
            output.status.success(),
            "{args:?}: {} {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
            self.log()
        );
        let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(reply["ok"], true, "{reply}");
        reply["data"].clone()
    }
    fn log(&self) -> String {
        fs::read_to_string(self.root.join("app.log")).unwrap_or_default()
    }
    fn prepare_wrapper_provision(&self) {
        let wrapper = self.root.join("bin/bboxy450-harness");
        if !wrapper.is_file() {
            return;
        }
        let worker = PathBuf::from(
            std::env::var_os("DREAM_C1_RUNNER").expect("canonical ordinary worker required"),
        );
        assert!(worker.is_absolute());
        let receipt_path = worker.with_extension("dream-c1.json");
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::symlink_metadata(&receipt_path)
            .expect("canonical worker provision is required; no fixture fallback");
        assert!(
            metadata.is_file()
                && metadata.len() <= 8192
                && metadata.mode() & 0o077 == 0
                && metadata.nlink() == 1
        );
        assert_eq!(metadata.uid(), unsafe { libc::geteuid() });
        let receipt: Value = serde_json::from_slice(&fs::read(&receipt_path).unwrap()).unwrap();
        assert_eq!(receipt["schema"], "bboxy.dream.c1.provision/1");
        assert_eq!(receipt["role"], "worker");
        assert_eq!(receipt["metrics_profile"], "c1-native-debt/1");
        assert_eq!(receipt["bootstrap_profile"], "dream-bootstrap/1");
        assert_eq!(receipt["build_profile"], "release");
        assert_eq!(receipt["default_features"], true);
        assert_eq!(receipt["explicit_features"], json!([]));
        assert_eq!(receipt["effective_features"], json!(["acp-agent"]));
        assert_eq!(receipt["test_fixtures"], false);
        let launch_hash = digest(&fs::read(&wrapper).unwrap());
        let provision = if receipt["binary_sha256"].as_str() == Some(launch_hash.as_str()) {
            // Decision66076: a copy is another inode/path, so admit that exact
            // file through the canonical provisioning-only Make seam. It does
            // --version once and never Hello/Start; old admission cannot move.
            let workspace=Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap();
            let output=Command::new("make").arg("-C").arg(workspace)
                .arg("admit-dream-c1-file")
                .arg(format!("DREAM_C1_ADMIT_FILE={}",wrapper.display()))
                .output().unwrap();
            assert!(output.status.success(),"copy provisioning failed: {} {}",String::from_utf8_lossy(&output.stdout),String::from_utf8_lossy(&output.stderr));
            let admitted:Value=serde_json::from_slice(&fs::read(wrapper.with_extension("dream-c1.json")).unwrap()).unwrap();
            assert_eq!(admitted["binary_sha256"],receipt["binary_sha256"]);
            assert_eq!(admitted["first_launch_admission"]["image"]["path"],json!(wrapper));
            assert_eq!(admitted["first_launch_admission"]["image"]["inode"],fs::metadata(&wrapper).unwrap().ino());
            assert_eq!(admitted["first_launch_admission"]["state"],"admitted");
            admitted
        } else {
            // Explicit protocol-fault transport only; this role can never
            // authorize positive bootstrap/native-profile completion.
            json!({"schema":"bboxy.dream.c1.provision/1","role":"private_wrapper","metrics_profile":"c1-native-debt/1","bootstrap_profile":"dream-bootstrap/1",
                "binary_sha256":launch_hash,"actual_worker":worker,"actual_worker_sha256":receipt["binary_sha256"]})
        };
        let bytes = bbt_proto::dream::canonical(&provision).unwrap();
        assert!(bytes.len() <= 8192);
        let path = wrapper.with_extension("dream-c1.json");
        if path.exists() {
            assert_eq!(
                fs::read(path).unwrap(),
                bytes,
                "running fixture provision may not be rewritten"
            );
        } else {
            private_bytes(&path, &bytes);
        }
    }
    fn start(&mut self) {
        self.prepare_wrapper_provision();
        let log = fs::File::create(self.root.join("app.log")).unwrap();
        self.app = Some(
            self.command()
                .args(["--headless", "--ctl", "--unattended", "--test-account"])
                .arg(self.root.join("account"))
                .arg("--run-dir")
                .arg(self.root.join("run"))
                .arg("--repository")
                .arg(self.root.join("repository"))
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        let until = Instant::now() + Duration::from_secs(30);
        loop {
            assert!(
                self.app.as_mut().unwrap().try_wait().unwrap().is_none(),
                "private app exited: {}",
                self.log()
            );
            if self.root.join("run/gui.endpoint").is_file() {
                let output = self.raw(&["app", "status"]);
                if output.status.success() {
                    let reply: Value = serde_json::from_slice(&output.stdout).unwrap();
                    if reply["data"]["gui_ready"] == true && reply["data"]["backend_ready"] == true
                    {
                        break;
                    }
                }
            }
            assert!(
                Instant::now() < until,
                "private app did not become ready: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(20));
        }
        if self.root.join("bin/bboxy450-harness").is_file() {
            // A real managed launch must follow the asynchronous matching Hello
            // probe. Otherwise that launch honestly keeps legacy ownership and
            // cannot become this fixture's later host-owned C1 source.
            let probe = self.ctl(&[
                "seat",
                "spawn",
                "zsh",
                "--place",
                "tab",
                "--name",
                "DreamCapability",
            ])["seat"]
                .as_str()
                .unwrap()
                .to_owned();
            wait(
                "verified matching C1 capabilities before managed launch",
                || {
                    let value =
                        self.ctl(&["dream", "inspect", &probe, "00000000000000000000000003:1"]);
                    let code = value["error"]["code"].as_str();
                    assert!(
                        matches!(
                            code,
                            Some(
                                "source_unavailable"
                                    | "runner_unavailable"
                                    | "stale_context"
                                    | "busy"
                            )
                        ),
                        "unexpected capability readiness outcome: {value}"
                    );
                    (code == Some("source_unavailable")).then_some(())
                },
            );
        }
    }
    fn ready(&self, seat: &str, run: &str) {
        let until = Instant::now() + Duration::from_secs(10);
        loop {
            let reply = self.ctl(&["dream", "status", seat, run]);
            if reply["error"]["code"] == "not_found" {
                return;
            }
            assert_eq!(
                reply["error"]["code"], "stale_context",
                "unexpected C1 authority error: {reply}"
            );
            assert!(
                Instant::now() < until,
                "Dream current context unavailable: {reply}\n{}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        // Read only the backend identities created in this fixture's private run.
        let owners: Vec<u32> = ["supervisor", "logic"]
            .into_iter()
            .filter_map(|name| {
                let data: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).ok()?,
                )
                .ok()?;
                data["process"]["pid"].as_u64().map(|pid| pid as u32)
            })
            .collect();
        let _ = self.raw(&["app", "stop"]);
        if let Some(mut child) = self.app.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        for pid in owners {
            // SAFETY: these are exact private backend PIDs started by this app.
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
        if std::thread::panicking() {
            let directory = std::mem::replace(&mut self.directory, tempfile::tempdir().unwrap());
            eprintln!(
                "failed private Dream fixture retained: {}",
                directory.keep().display()
            );
        }
    }
}
fn missing(reply: Value) -> Value {
    assert_eq!(reply["v"], 1, "wrong control reply: {reply}");
    assert_eq!(reply["ok"], false, "invented run: {reply}");
    assert_eq!(
        reply["error"]["code"], "not_found",
        "wrong missing-run refusal: {reply}"
    );
    assert!(reply.get("run").is_none(), "run metadata leaked: {reply}");
    reply
}
#[test]
fn dream_c1_real_ctl_selects_one_seat_and_preserves_missing_outcomes_and_sources() {
    let mut f = Fixture::new();
    f.start();
    let own = f.ctl(&[
        "seat", "spawn", "zsh", "--place", "tab", "--name", "DreamOwn",
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let peer = f.ctl(&[
        "seat",
        "spawn",
        "zsh",
        "--place",
        "tab",
        "--name",
        "DreamPeer",
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let run = "00000000000000000000000001";
    f.ready(&own, run);
    f.ready(&peer, run);
    for seat in [&own, &peer] {
        missing(f.ctl(&["dream", "status", seat, run]));
        missing(f.ctl(&["dream", "status", seat, run, "0"]));
        missing(f.ctl(&[
            "dream",
            "cancel",
            seat,
            run,
            "--op",
            "00000000000000000000000002",
        ]));
    }
    let unknown = f.ctl(&[
        "dream",
        "inspect",
        &own,
        "00000000000000000000000003:1",
        "--op",
        "00000000000000000000000004",
    ]);
    assert_eq!(unknown["v"], 1);
    assert_eq!(unknown["ok"], false);
    assert!(
        matches!(
            unknown["error"]["code"].as_str(),
            Some("runner_unavailable" | "source_unavailable")
        ),
        "unknown source/runner was not refused honestly: {unknown}"
    );
    let original = f.root.join("unregistered-source.jsonl");
    fs::write(&original, b"PRIVATE UNREGISTERED SOURCE\n").unwrap();
    let before = fs::read(&original).unwrap();
    for args in [
        vec!["dream", "inspect", &own, "00000000000000000000000003:0"],
        vec![
            "dream",
            "inspect",
            &own,
            "00000000000000000000000003:1",
            "00000000000000000000000003:2",
        ],
        vec!["dream", "status", &own, run, "-1"],
        vec![
            "dream",
            "status",
            &own,
            run,
            "--root",
            original.to_str().unwrap(),
        ],
        vec!["dream", "status", &own, run, "--seat", &peer],
    ] {
        let output = f.raw(&args);
        assert_eq!(
            output.status.code(),
            Some(2),
            "malformed control was not a usage refusal: {}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    assert_eq!(
        fs::read(original).unwrap(),
        before,
        "control touched unregistered source"
    );
    let status = f.ctl(&["app", "status"]);
    assert_eq!(status["backend_ready"], json!(true));
}

fn sse(delta: Value, finish: &str) -> String {
    format!(
        "data: {}\n\ndata: {}\n\ndata: [DONE]\n\n",
        json!({"choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
        json!({"choices":[{"index":0,"delta":{},"finish_reason":finish}]})
    )
}
fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
fn wait<T>(description: &str, mut read: impl FnMut() -> Option<T>) -> T {
    let until = Instant::now() + Duration::from_secs(35);
    loop {
        if let Some(value) = read() {
            return value;
        }
        assert!(
            Instant::now() < until,
            "private C1 witness timed out: {description}"
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
fn row(f: &Fixture, seat: &str) -> Value {
    f.ctl(&["seat", "ls"])
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == seat)
        .unwrap()
        .clone()
}
fn session(f: &Fixture, seat: &str) -> (String, u64, PathBuf) {
    wait("retained harness session", || {
        let value = row(f, seat);
        assert_ne!(
            value["state"]["process"]["kind"],
            "exited",
            "private harness exited before publishing its native session: state={}\n{}",
            value["state"],
            f.log()
        );
        let session = value["record"]["harness_session_id"].as_str()?;
        let generation = value["record"]["generation"].as_u64()?;
        let path = f
            .root
            .join(format!("home/.bboxy450-harness/sessions/{session}.jsonl"));
        path.is_file()
            .then(|| (session.to_owned(), generation, path))
    })
}
fn runtime_path(f: &Fixture) -> PathBuf {
    let locator = f.root.join("run/c1-runtime-path.json");
    if locator.is_file() {
        let value: Value = serde_json::from_slice(&fs::read(locator).unwrap()).unwrap();
        let path = PathBuf::from(value["path"].as_str().unwrap());
        assert!(path.canonicalize().unwrap().starts_with(&f.root));
        path
    } else {
        f.root.join("run")
    }
}
fn private_bytes(path: &Path, bytes: &[u8]) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
fn private_json(path: &Path, value: &Value) {
    let mut bytes = serde_json::to_vec(value).unwrap();
    bytes.push(b'\n');
    private_bytes(path, &bytes);
}
fn journal_worker(f: &Fixture, seat: &str, run: &str) -> Option<(u32, String)> {
    // This is the private fixture's exact retained run, not an account/source scan.
    // No raw transcript is printed or extracted from the journal.
    let output = Command::new("/usr/bin/python3")
        .args(["-I", "-c", "import json,sqlite3,sys\nc=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True)\nr=c.execute('SELECT record FROM runs WHERE id=?',(sys.argv[2],)).fetchone()\nv=json.loads(r[0]) if r else {}\nprint(json.dumps([v.get('worker_pid'),v.get('worker_start')]))"])
        .arg(runtime_path(f).join(format!("dream-c1/journals/{seat}.sqlite")))
        .arg(run)
        .env_clear()
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value: Value = serde_json::from_slice(&output.stdout).ok()?;
    Some((
        u32::try_from(value[0].as_u64()?).ok()?,
        value[1].as_str()?.to_owned(),
    ))
}
fn journal_record(f: &Fixture, seat: &str, run: &str) -> (Value, u64) {
    let output=Command::new("/usr/bin/python3").args(["-I","-c",
        "import json,sqlite3,sys\nc=sqlite3.connect('file:'+sys.argv[1]+'?mode=ro',uri=True)\nr=c.execute('SELECT record FROM runs WHERE id=?',(sys.argv[2],)).fetchone()\nprint(json.dumps([json.loads(r[0]),c.execute('SELECT count(*) FROM runs').fetchone()[0]]))"])
        .arg(runtime_path(f).join(format!("dream-c1/journals/{seat}.sqlite"))).arg(run).env_clear().output().unwrap();
    assert!(
        output.status.success(),
        "private retained run record unavailable"
    );
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    (value[0].clone(), value[1].as_u64().unwrap())
}
fn owner_pid(f: &Fixture, name: &str) -> u32 {
    let value: Value =
        serde_json::from_slice(&fs::read(f.root.join(format!("run/{name}.owner.json"))).unwrap())
            .unwrap();
    value["process"]["pid"]
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .unwrap()
}
struct HeldWorker {
    pid: u32,
    start: String,
}
impl HeldWorker {
    fn stop(pid: u32, start: String) -> Self {
        assert_eq!(
            bbt_mux::adapters::dream_c1::process_stamp(pid).unwrap(),
            start
        );
        // SAFETY: this exact private worker was started by this fixture's app,
        // recorded by its process owner, and its start stamp was revalidated.
        assert_eq!(unsafe { libc::kill(pid as i32, libc::SIGSTOP) }, 0);
        Self { pid, start }
    }
    fn current(&self) -> bool {
        bbt_mux::adapters::dream_c1::process_stamp(self.pid).is_ok_and(|value| value == self.start)
    }
}
impl Drop for HeldWorker {
    fn drop(&mut self) {
        if self.current() {
            // Releasing the exact owned stop prevents a failed fixture from leaving
            // an uninterruptible private worker for the app's normal cancel lane.
            unsafe {
                libc::kill(self.pid as i32, libc::SIGCONT);
            }
        }
    }
}

/// Kills and reaps only this fixture's exact native child, including panic paths.
struct OwnedChild {
    process: Child,
    root: PathBuf,
    c1_worker: Option<(u32, String)>,
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if self.process.try_wait().ok().flatten().is_some() {
            return;
        }
        // The private hook converts this into Native::cancel, retaining the real
        // Core through ordinary worker/PTY cancellation and actual reap.
        let _ = fs::write(self.root.join("run/c1-abort"), b"abort\n");
        let until = Instant::now() + Duration::from_secs(5);
        while Instant::now() < until {
            if self.process.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut identities = Vec::new();
        if let Ok(raw) = fs::read(self.root.join("native-frame-run/owned-pids.json"))
            && let Ok(pids) = serde_json::from_slice::<Vec<u32>>(&raw)
        {
            for pid in pids {
                if let Ok(start) = bbt_mux::adapters::native::process_start(pid) {
                    identities.push((pid, start, false));
                }
            }
        }
        if let Some(worker) = &self.c1_worker {
            identities.push((worker.0, worker.1.clone(), true));
        }
        let _ = self.process.kill();
        let _ = self.process.wait();
        for (pid, start, kernel) in identities {
            let current = if kernel {
                bbt_mux::adapters::dream_c1::process_stamp(pid).ok()
            } else {
                bbt_mux::adapters::native::process_start(pid).ok()
            };
            if current.as_ref() == Some(&start) {
                // SAFETY: exact PIDs recorded by this fixture's Core, revalidated
                // against their retained incarnation; no process-name/group kill.
                unsafe {
                    libc::kill(pid as i32, libc::SIGKILL);
                }
            }
        }
    }
}

/// A persistent connection to the same admitted ctl GUI door. It supplies no actor.
struct DreamPeer {
    reader: std::io::BufReader<std::os::unix::net::UnixStream>,
    writer: std::os::unix::net::UnixStream,
    capability: [u8; 32],
}
impl DreamPeer {
    fn connect(root: &Path) -> Self {
        let endpoint = bboxy450_term::ctl::Endpoint::read(&root.join("run/gui.endpoint")).unwrap();
        let writer = std::os::unix::net::UnixStream::connect(endpoint.path).unwrap();
        writer
            .set_read_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        writer
            .set_write_timeout(Some(Duration::from_millis(250)))
            .unwrap();
        Self {
            reader: std::io::BufReader::new(writer.try_clone().unwrap()),
            writer,
            capability: endpoint.capability,
        }
    }
    fn ask(&mut self, seat: bbt_mux::Ulid, request: bbt_proto::dream::Control) -> Value {
        use std::io::{BufRead, Read, Write};
        let id = match &request {
            bbt_proto::dream::Control::Cancel(value) => value.op_id.0.parse().unwrap(),
            bbt_proto::dream::Control::Inspect(value) => value.op_id.0.parse().unwrap(),
            _ => bbt_mux::Ulid::new(),
        };
        let mut bytes = serde_json::to_vec(&bbt_proto::GuiRequest {
            version: bbt_proto::gui::VERSION,
            capability: self.capability,
            action: bbt_proto::GuiAction::Dream { id, seat, request },
        })
        .unwrap();
        bytes.push(b'\n');
        self.writer.write_all(&bytes).unwrap();
        loop {
            let mut line = Vec::new();
            let n = std::io::Read::by_ref(&mut self.reader)
                .take(64 * 1024 + 1)
                .read_until(b'\n', &mut line)
                .unwrap();
            assert!(
                n > 0 && line.len() <= 64 * 1024,
                "unbounded/closed private ctl reply"
            );
            let reply: Value = serde_json::from_slice(&line).unwrap();
            if reply["body"]["op"] == json!(id) {
                assert_eq!(
                    reply["kind"], "Dream",
                    "private ctl refused request: {reply}"
                );
                return reply["body"]["response"].clone();
            }
        }
    }
}
// Adapted from this project's tests/mcp.rs admitted bridge fixture. This reads
// only our source child's private launch paths; admission bytes never enter logs.
fn mcp(f: &Fixture, seat: &str) -> bbt_mux::mcp::bridge::Bridge {
    use bbt_mux::mcp::bridge::{Bridge, Config};
    let context: Value =
        serde_json::from_slice(&fs::read(f.root.join(format!("admission-{seat}.json"))).unwrap())
            .unwrap();
    assert_eq!(context["BBOXY_SEAT_ID"], seat);
    let path = Path::new(context["BBOXY_CAPABILITY_FILE"].as_str().unwrap());
    assert!(path.canonicalize().unwrap().starts_with(&f.root));
    let raw = fs::read(path).unwrap();
    assert_eq!(raw.len(), 65, "private admission format changed");
    assert_eq!(raw[64], b'\n');
    let mut capability = [0u8; 32];
    for (index, byte) in capability.iter_mut().enumerate() {
        *byte = u8::from_str_radix(
            std::str::from_utf8(&raw[index * 2..index * 2 + 2]).unwrap(),
            16,
        )
        .unwrap();
    }
    let socket = PathBuf::from(context["BBOXY_SOCK"].as_str().unwrap());
    // Logic uses a fresh private /tmp socket namespace to meet sockaddr_un.
    assert!(socket.is_absolute());
    let mut peer = Bridge::new(Config {
        socket,
        seat: seat.parse().unwrap(),
        generation: context["BBOXY_GENERATION"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap(),
        capability,
    });
    let reply = rpc(
        &mut peer,
        1,
        "initialize",
        json!({
        "protocolVersion":"2025-11-25","capabilities":{},
        "clientInfo":{"name":"private-real-C1-source","version":"1"}}),
    );
    assert_eq!(reply["result"]["protocolVersion"], "2025-11-25");
    assert!(
        peer.handle(br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
    let oriented = rpc(
        &mut peer,
        2,
        "tools/call",
        json!({"name":"orient","arguments":{}}),
    );
    assert_eq!(
        oriented["result"]["structuredContent"]["actor"]["seat_id"],
        seat
    );
    peer
}
fn rpc(peer: &mut bbt_mux::mcp::bridge::Bridge, id: u64, method: &str, params: Value) -> Value {
    peer.handle(
        &serde_json::to_vec(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .unwrap(),
    )
    .expect("missing actual admitted MCP reply")
}
fn mcp_control(
    peer: &mut bbt_mux::mcp::bridge::Bridge,
    id: u64,
    op: &str,
    arguments: Value,
) -> Value {
    let reply = rpc(
        peer,
        id,
        "tools/call",
        json!({"name":"dream_c1","arguments":arguments,"_meta":{"bboxy450/op_id":op}}),
    );
    assert!(
        reply.get("error").is_none(),
        "actual MCP transport error: {reply}"
    );
    let data = reply["result"]["structuredContent"].clone();
    assert_eq!(data["v"], 1, "missing actual MCP C1 reply: {reply}");
    data
}
fn status_request(run: &str, page: Option<u32>) -> bbt_proto::dream::Control {
    bbt_proto::dream::Control::Status(bbt_proto::dream::Status {
        v: bbt_proto::dream::Version,
        run_id: bbt_proto::dream::Ulid::try_from(run.to_owned()).unwrap(),
        page,
    })
}
fn summary(mut times: Vec<f64>) -> Value {
    assert_eq!(times.len(), 200, "missing actual round trips");
    times.sort_by(f64::total_cmp);
    let p99 = times[times.len() * 99 / 100];
    let maximum = *times.last().unwrap();
    assert!(p99 <= 100., "C1 round-trip p99={p99} ms exceeds100ms");
    assert!(
        maximum <= 250.,
        "C1 round-trip maximum={maximum} ms exceeds250ms"
    );
    json!({"samples":times.len(),"p99_ms":p99,"max_ms":maximum})
}

// The producer's raw compute maximum includes the two specifically admitted
// native classes. Its signed, disaggregated profile must qualify that value;
// ordinary application, control and cleanup retain the unchanged1ms bound.
fn assert_validated_producer_compute(metrics: &Value) {
    assert_eq!(metrics["native_metrics_authorized"], true);
    assert_eq!(metrics["producer_native_profile_verified"], true);
    assert_eq!(metrics["producer_cost_known"], 1);
    assert_eq!(metrics["producer_terminal_remaining_debt_ns"], 0);
    for key in [
        "producer_terminal_app_max_cpu_ns",
        "producer_terminal_control_max_cpu_ns",
        "producer_max_cleanup_unit_cpu_ns",
    ] {
        assert!(
            metrics[key].as_u64().unwrap() <= 1_000_000,
            "{key}: {metrics}"
        );
    }
    let captured = metrics["producer_terminal_captured_max_cpu_ns"]
        .as_u64()
        .unwrap();
    let raw = metrics["producer_max_unit_cpu_ns"].as_u64().unwrap();
    let inclusive = metrics["producer_terminal_inclusive_max_cpu_ns"]
        .as_u64()
        .unwrap();
    assert!(
        captured <= raw && raw <= inclusive && inclusive <= 3_000_000,
        "{metrics}"
    );
}
fn write_evidence(name: &str, value: &Value) {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let worker = PathBuf::from(
        std::env::var_os("DREAM_C1_RUNNER").expect("explicit ordinary release worker required"),
    );
    let seat = PathBuf::from(
        std::env::var_os("DREAM_C1_SEAT_RUNNER")
            .expect("explicit feature-built seat runner required"),
    );
    let seat_profile = std::env::var("DREAM_C1_SEAT_PROFILE")
        .expect("explicit seat artifact build profile required; no canonical profile inference");
    assert!(matches!(
        seat_profile.as_str(),
        "release" | "dream-c1-fixture"
    ));
    let mut payload = value.clone();
    payload["artifact_roles"] = json!({
        "worker":{"path":worker,"sha256":value["runner_sha256"],"profile":"release","explicit_features":[],"default_features":true,"test_fixtures":false},
        "seat":{"path":seat,"sha256":digest(&fs::read(&seat).unwrap()),"profile":seat_profile,"explicit_features":["test-fixtures"],"default_features":true,"test_fixtures":true},
    });
    let root = PathBuf::from(
        std::env::var_os("DREAM_C1_EVIDENCE")
            .expect("canonical C1 gate must supply a private evidence directory"),
    );
    let path = root.join(name);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    serde_json::to_writer(&mut file, &payload).unwrap();
    file.write_all(b"\n").unwrap();
    file.sync_all().unwrap();
}

struct ClosedSource {
    seat: String,
    session: String,
    generation: u64,
    selected: String,
    path: PathBuf,
    sha256: String,
    bytes: usize,
    mcp: Option<bbt_mux::mcp::bridge::Bridge>,
    close_requested_at: Instant,
}

/// Read-only kernel accounting for an exact fixture child, not heap or thread CPU.
fn live_process_usage(pid: u32, expected_start: &str) -> Value {
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(pid).unwrap(),
        expected_start
    );
    let mut usage = std::mem::MaybeUninit::<libc::rusage_info_v0>::zeroed();
    // SAFETY: flavor V0 names this exact initialized-size output buffer. The
    // read-only kernel call neither controls another process nor takes ownership.
    assert_eq!(
        unsafe {
            libc::proc_pid_rusage(
                i32::try_from(pid).unwrap(),
                libc::RUSAGE_INFO_V0,
                usage.as_mut_ptr().cast::<libc::rusage_info_t>(),
            )
        },
        0,
        "actual private child CPU accounting unavailable: {}",
        std::io::Error::last_os_error()
    );
    // SAFETY: proc_pid_rusage returned success for the matching V0 buffer.
    let usage = unsafe { usage.assume_init() };
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(pid).unwrap(),
        expected_start
    );
    json!({"user_cpu_ns":usage.ri_user_time,"system_cpu_ns":usage.ri_system_time,"resident_bytes":usage.ri_resident_size,"kernel_start_abstime":usage.ri_proc_start_abstime})
}
struct OpenSource {
    seat: String,
    session: String,
    generation: u64,
    selected: String,
    path: PathBuf,
    mcp: bbt_mux::mcp::bridge::Bridge,
}
fn close_source(f: &Fixture, name: &str, turns: usize) -> ClosedSource {
    close_prepared(f, prepare_source(f, name, turns))
}
fn prepare_source(f: &Fixture, name: &str, turns: usize) -> OpenSource {
    let own = f.ctl(&[
        "seat",
        "spawn",
        "bboxy450-harness",
        "--place",
        "tab",
        "--name",
        name,
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let (source_session, source_generation, source_path) = session(f, &own);
    if let Some(mode) = &f.fault_mode {
        // The authenticated private parent creates the exact job's test marker
        // before worker birth. No interpreter runs ahead of the bootstrap gate.
        let directory = runtime_path(f).join("dream-c1");
        assert!(directory.is_dir());
        private_bytes(
            &directory.join(format!(".fixture-{mode}-{own}-{source_generation}")),
            b"",
        );
    }
    let selected = format!("{source_session}:{source_generation}");
    wait("owned open source catalogue", || {
        let value = f.ctl(&["dream", "inspect", &own, &selected]);
        (value["error"]["code"] == "source_open").then_some(())
    });
    // Keep this launch's admitted door before EOF can remove launch files.
    // No new actor or generation is substituted after closure.
    let peer = mcp(f, &own);
    for turn in 1..=turns {
        f.ctl(&[
            "seat",
            "type",
            &own,
            "Create the bounded post-session witness",
        ]);
        f.ctl(&["seat", "key", &own, "enter"]);
        wait("real scripted turn completion", || {
            let source = fs::read_to_string(&source_path).ok()?;
            (source
                .lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .filter(|value| value["event"]["type"] == "turn_ended")
                .count()
                == turn)
                .then_some(())
        });
        let open = f.ctl(&["dream", "inspect", &own, &selected]);
        assert_eq!(
            open["error"]["code"], "source_open",
            "turn end admitted Dream: {open}"
        );
    }
    OpenSource {
        seat: own,
        session: source_session,
        generation: source_generation,
        selected,
        path: source_path,
        mcp: peer,
    }
}
fn close_prepared(f: &Fixture, prepared: OpenSource) -> ClosedSource {
    close_prepared_mode(f, prepared, false)
}
fn close_prepared_mode(f: &Fixture, prepared: OpenSource, retained: bool) -> ClosedSource {
    let OpenSource {
        seat: own,
        session: source_session,
        generation: source_generation,
        selected,
        path: source_path,
        mcp: peer,
    } = prepared;
    let close_requested_at = Instant::now();
    if retained {
        native_close(f, &source_session);
    } else {
        f.ctl(&["seat", "key", &own, "ctrl-d"]);
    }
    let closed = wait("authoritative durable SessionClosed", || {
        let bytes = fs::read(&source_path).ok()?;
        let value: Value = serde_json::from_slice(
            bytes
                .split(|b| *b == b'\n')
                .rfind(|line| !line.is_empty())?,
        )
        .ok()?;
        (value["event"]["type"] == "session_closed").then_some(bytes)
    });
    ClosedSource {
        seat: own,
        session: source_session,
        generation: source_generation,
        selected,
        path: source_path,
        sha256: digest(&closed),
        bytes: closed.len(),
        mcp: Some(peer),
        close_requested_at,
    }
}
fn inspect_closed(f: &Fixture, source: &ClosedSource, op: &str) -> Value {
    wait("post-close automatic/manual coalesced admission", || {
        let value = f.ctl(&[
            "dream",
            "inspect",
            &source.seat,
            &source.selected,
            "--op",
            op,
        ]);
        if value["ok"] == true {
            return Some(value);
        }
        assert!(
            matches!(
                value["error"]["code"].as_str(),
                Some("source_open" | "source_unavailable" | "busy" | "stale_context")
            ),
            "C1 admission failed: {value}\n{}",
            f.log()
        );
        None
    })
}

fn running_worker_proof(f: &Fixture, seat: &str, run: &str) -> Value {
    let mut receipt = wait("owned Hello child receipt before Start", || {
        serde_json::from_slice::<Value>(
            &fs::read(runtime_path(f).join(format!("dream-c1/{run}/worker-started.json"))).ok()?,
        )
        .ok()
    });
    assert_eq!(receipt["run_id"], run);
    assert_eq!(receipt["seq"], 0);
    assert_eq!(receipt["phase"], "spawned_hello_validated");
    assert_eq!(receipt["effect_stage"], "spawned_hello_validated");
    assert_eq!(receipt["work_started"], false);
    // The immutable receipt proves ownership/Hello only. Production sets public
    // Running solely after a sequence/identity-valid Progress, so every replay,
    // CPU, pause and GPU-overlap claim independently waits for that actual API.
    let running = wait("actual public Running after validated Progress", || {
        let value = f.ctl(&["dream", "status", seat, run]);
        assert_ne!(
            value["run"]["settled"], true,
            "worker settled before actual running witness: {value}"
        );
        (value["ok"] == true && value["run"]["state"] == "running").then_some(value)
    });
    let (pid, start) = journal_worker(f, seat, run).unwrap();
    assert_eq!(receipt["worker_pid"], pid);
    assert_eq!(receipt["worker_start"], start);
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(pid).unwrap(),
        start
    );
    receipt["public_running_after_validated_progress"] = running["run"].clone();
    receipt
}

/// Ordinary producer Unix gateway, confined to this actual managed source.
/// A sent Close can lose its ACK when the durable close unlinks its address;
/// the caller resolves that outcome from SessionClosed and the fresh B header.
fn native_close(f: &Fixture, session: &str) {
    use std::io::{BufRead, Read, Write};
    let path = f
        .root
        .join(format!("home/.bboxy450-harness/run/{session}.sock"));
    let mut socket = std::os::unix::net::UnixStream::connect(path).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_millis(250)))
        .unwrap();
    let mut reader = std::io::BufReader::new(socket.try_clone().unwrap());
    let mut subscribe=serde_json::to_vec(&json!({"v":{"major":1,"minor":3},"subscribe":{"session":session,"after":null,"ephemeral":false}})).unwrap();
    subscribe.push(b'\n');
    socket.write_all(&subscribe).unwrap();
    let mut line = Vec::new();
    Read::by_ref(&mut reader)
        .take(256 * 1024 + 1)
        .read_until(b'\n', &mut line)
        .unwrap();
    assert!(line.len() <= 256 * 1024);
    let hello: Value = serde_json::from_slice(&line).unwrap();
    assert_eq!(
        hello["meta"]["session"], session,
        "native source identity changed before Close"
    );
    let op = bbt_mux::Ulid::new();
    let mut close = serde_json::to_vec(
        &json!({"v":{"major":1,"minor":3},"op_id":op,"session":session,"op":{"type":"close"}}),
    )
    .unwrap();
    close.push(b'\n');
    socket.write_all(&close).unwrap();
}

fn measured_controls(f: &Fixture, probe: &str, own: &str, run: &str) -> Value {
    measured_controls_with_gap(f, probe, own, run, Duration::ZERO)
}
fn measured_controls_with_gap(
    f: &Fixture,
    probe: &str,
    own: &str,
    run: &str,
    gap: Duration,
) -> Value {
    let endpoint = bboxy450_term::ctl::Endpoint::read(&f.root.join("run/gui.endpoint")).unwrap();
    let probe_row = row(f, probe);
    let probe_id = probe.parse::<bbt_mux::Ulid>().unwrap();
    let probe_generation = probe_row["record"]["generation"].as_u64().unwrap();
    let swarm = probe_row["record"]["swarm"]
        .as_str()
        .unwrap()
        .parse::<bbt_mux::Ulid>()
        .unwrap();
    let own_id = own.parse::<bbt_mux::Ulid>().unwrap();
    std::thread::scope(|scope| {
        let input_endpoint = endpoint.clone();
        let inputs = scope.spawn(move || {
            let mut peer = bboxy450_term::ctl::Client::connect(&input_endpoint).unwrap();
            summary(
                (0..200)
                    .map(|_| {
                        std::thread::sleep(gap);
                        let at = Instant::now();
                        peer.input(probe_id, probe_generation, b"\x1b[C".to_vec())
                            .unwrap();
                        at.elapsed().as_secs_f64() * 1000.
                    })
                    .collect(),
            )
        });
        let message_endpoint = endpoint.clone();
        let messages = scope.spawn(move || {
            let mut peer = bboxy450_term::ctl::Client::connect(&message_endpoint).unwrap();
            summary(
                (0..200)
                    .map(|index| {
                        std::thread::sleep(gap);
                        let at = Instant::now();
                        let delivered = peer
                            .messaging(
                                swarm,
                                bbt_proto::messaging::Request::Send {
                                    message: bbt_proto::messaging::SendRequest::text(
                                        format!("seats/{probe_id}/inbox"),
                                        format!("private C1 sample{index}"),
                                    ),
                                },
                            )
                            .unwrap();
                        assert!(
                            matches!(delivered, bbt_proto::messaging::Response::Stored { .. }),
                            "private message was not stored"
                        );
                        at.elapsed().as_secs_f64() * 1000.
                    })
                    .collect(),
            )
        });
        let statuses = scope.spawn(|| {
            let mut peer = DreamPeer::connect(&f.root);
            summary(
                (0..200)
                    .map(|_| {
                        std::thread::sleep(gap);
                        let at = Instant::now();
                        let reply = peer.ask(own_id, status_request(run, None));
                        assert_eq!(reply["ok"], true, "{reply}");
                        assert_eq!(
                            reply["run"]["settled"], false,
                            "held worker certified settled"
                        );
                        at.elapsed().as_secs_f64() * 1000.
                    })
                    .collect(),
            )
        });
        json!({"input":inputs.join().unwrap(),"message":messages.join().unwrap(),"status":statuses.join().unwrap()})
    })
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_real_closed_job_owns_controls_dedup_and_native_frame_overlap() {
    use bbt_proto::dream as w;
    let f = Fixture::scripted();
    f.prepare_wrapper_provision();
    let frame_report = f.root.join("native-frames.json");
    let frame_log = fs::File::create(f.root.join("native-frames.log")).unwrap();
    let mut native = OwnedChild {
        process: f
            .command()
            .env("BBOXY_TEST_CUSTODY", "memory")
            .env("BBOXY_DREAM_C1_NATIVE_JOIN", f.root.join("run"))
            .args([
                "--stall-test",
                "--stall-headless",
                "--stall-real",
                "--stall-frame-gate",
                "--stall-seats",
                "1",
                "--stall-watched",
                "1",
                "--stall-seconds",
                "5",
                "--stall-idle",
                "1",
                "--stall-report",
            ])
            .arg(&frame_report)
            .arg("--stall-run-dir")
            .arg(f.root.join("native-frame-run"))
            .stdout(frame_log.try_clone().unwrap())
            .stderr(frame_log)
            .spawn()
            .unwrap(),
        root: f.root.clone(),
        c1_worker: None,
    };
    wait("same Native/Logic C1 door", || {
        assert!(
            native.process.try_wait().unwrap().is_none(),
            "private native join exited: {}",
            fs::read_to_string(f.root.join("native-frames.log")).unwrap_or_default()
        );
        f.root.join("run/gui.endpoint").is_file().then_some(())
    });
    // Prepare a second private root before the first job is admitted; the
    // measured ready interval below starts at its actual authoritative close.
    let mut independent_fixture = Fixture::scripted();
    independent_fixture.start();
    let probe = f.ctl(&[
        "seat",
        "spawn",
        "zsh",
        "--place",
        "tab",
        "--name",
        "DreamInput",
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    wait("verified C1 capability", || {
        let value = f.ctl(&["dream", "inspect", &probe, "00000000000000000000000003:1"]);
        (value["error"]["code"] == "source_unavailable").then_some(())
    });
    let mut source = close_source(&f, "DreamClosed", 16);
    let own = source.seat.clone();
    let source_session = source.session.clone();
    let source_generation = source.generation;
    let source_path = source.path.clone();
    let selected = source.selected.clone();
    let original_hash = source.sha256.clone();
    let source_bytes = source.bytes;
    assert!(
        source_bytes >= 2 * 1024 * 1024,
        "fixture did not create a real multi-MiB source"
    );
    let inspect_op = "00000000000000000000000020";
    let admitted = inspect_closed(&f, &source, inspect_op);
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let started = running_worker_proof(&f, &own, &run);
    assert_eq!(started["run_id"], run);
    let (pid, start) = wait("owned real worker identity before replay", || {
        journal_worker(&f, &own, &run)
    });
    assert_eq!(started["worker_pid"], pid);
    assert_eq!(started["worker_start"], start);
    native.c1_worker = Some((pid, start.clone()));
    let held = HeldWorker::stop(pid, start);
    let independent_source = close_source(&independent_fixture, "DreamIndependentRoot", 1);
    let independent_admitted = inspect_closed(
        &independent_fixture,
        &independent_source,
        "00000000000000000000000028",
    );
    let independent_run = independent_admitted["run"]["run_id"]
        .as_str()
        .unwrap()
        .to_owned();
    let independent_terminal = wait(
        "independent ready root completes while first worker is held",
        || {
            let value = independent_fixture.ctl(&[
                "dream",
                "status",
                &independent_source.seat,
                &independent_run,
            ]);
            (value["run"]["settled"] == true).then_some(value)
        },
    );
    let independent_ready_ms = independent_source
        .close_requested_at
        .elapsed()
        .as_secs_f64()
        * 1000.;
    assert!(
        independent_ready_ms <= 2000.,
        "independent root exceeded2s: {independent_ready_ms}ms"
    );
    assert_eq!(
        independent_terminal["run"]["state"], "completed",
        "{independent_terminal}"
    );
    assert!(
        held.current(),
        "independent work replaced the first held worker"
    );
    let (main_record, _) = journal_record(&f, &own, &run);
    let (other_record, _) = journal_record(
        &independent_fixture,
        &independent_source.seat,
        &independent_run,
    );
    assert_ne!(
        main_record["context"]["root"], other_record["context"]["root"],
        "independent fixture reused one root"
    );
    let (other_pid, other_stamp) = journal_worker(
        &independent_fixture,
        &independent_source.seat,
        &independent_run,
    )
    .unwrap();
    assert!(
        !bbt_mux::adapters::dream_c1::process_stamp(other_pid)
            .is_ok_and(|value| value == other_stamp),
        "independent Completed preceded reap"
    );
    assert_eq!(
        digest(&fs::read(&independent_source.path).unwrap()),
        independent_source.sha256
    );
    let independent = json!({"run_id":independent_run,"ready_ms":independent_ready_ms,
        "root":other_record["context"]["root"],"held_root":main_record["context"]["root"],
        "completed":true,"actual_reap":true,"held_worker_unchanged":true,"source_unchanged":true});
    drop(independent_fixture);
    let mut mcp_peer = source.mcp.take().unwrap();
    let mcp_inspect = mcp_control(
        &mut mcp_peer,
        3,
        inspect_op,
        json!({
        "v":1,"method":"inspect","sources":[{"session":source_session,"source_generation":source_generation}]}),
    );
    assert_eq!(
        mcp_inspect["run"]["run_id"], run,
        "MCP Inspect duplicated an admitted close"
    );
    let mcp_status = mcp_control(
        &mut mcp_peer,
        4,
        "00000000000000000000000023",
        json!({"v":1,"method":"status","run_id":run}),
    );
    assert_eq!(mcp_status["ok"], true, "{mcp_status}");
    assert_eq!(mcp_status["run"]["settled"], false);
    let own_id = own.parse::<bbt_mux::Ulid>().unwrap();
    private_json(
        &f.root.join("run/c1-ready.json"),
        &json!({
        "run_id":run,"worker_pid":pid,"worker_start":held.start,"source_sha256":original_hash}),
    );
    let roundtrips = measured_controls(&f, &probe, &own, &run);
    wait(
        "240 actual same-owner GPU completions during the held C1 job",
        || {
            let error = f.root.join("run/c1-overlap-error.json");
            assert!(
                !error.exists(),
                "private C1 overlap failed: {}",
                fs::read_to_string(error).unwrap_or_default()
            );
            let value: Value = serde_json::from_slice(
                &fs::read(f.root.join("run/c1-overlap-progress.json")).ok()?,
            )
            .ok()?;
            (value["gpu_frames"].as_u64()? >= 240).then_some(())
        },
    );
    assert!(
        held.current(),
        "real C1 worker ended before the GPU overlap cut"
    );
    private_json(
        &f.root.join("run/c1-finish-overlap"),
        &json!({"run_id":run}),
    );
    let overlap = wait("same-owner C1 frame overlap receipt", || {
        serde_json::from_slice::<Value>(&fs::read(f.root.join("run/c1-overlap.json")).ok()?).ok()
    });
    assert_eq!(overlap["run_id"], run);
    assert_eq!(overlap["same_logic_owner"], true);
    assert_eq!(overlap["worker_pid"], pid);
    assert_eq!(overlap["worker_start"], held.start);
    assert_eq!(overlap["source_sha256"], original_hash);
    assert_eq!(overlap["overflow"], false);
    assert!(overlap["gpu_frames"].as_u64().unwrap() >= 240);
    assert!(overlap["cpu_p99_ms"].as_f64().unwrap() <= 4.);
    assert!(overlap["input_to_gpu_ms"].as_f64().unwrap() <= 16.7);
    let mut peer = DreamPeer::connect(&f.root);
    let cancel_op = w::Ulid::try_from("00000000000000000000000021".to_owned()).unwrap();
    let cancel = w::Control::Cancel(w::Cancel {
        v: w::Version,
        op_id: cancel_op,
        run_id: w::Ulid(run.clone()),
    });
    let mcp_cancel = mcp_control(
        &mut mcp_peer,
        5,
        "00000000000000000000000021",
        json!({"v":1,"method":"cancel","run_id":run}),
    );
    assert_eq!(mcp_cancel["ok"], true, "{mcp_cancel}");
    let cancels = summary(
        (0..200)
            .map(|_| {
                let at = Instant::now();
                let reply = peer.ask(own_id, cancel.clone());
                assert_eq!(reply["ok"], true, "{reply}");
                at.elapsed().as_secs_f64() * 1000.
            })
            .collect(),
    );
    let terminal = wait("cancelled only after actual process settlement", || {
        let value = f.ctl(&["dream", "status", &own, &run]);
        (value["run"]["settled"] == true).then_some(value)
    });
    assert_eq!(terminal["run"]["state"], "cancelled", "{terminal}");
    assert!(
        !held.current(),
        "Cancelled exposed before actual worker exit/reap"
    );
    let host_metrics: Value = serde_json::from_slice(
        &fs::read(runtime_path(&f).join(format!("dream-c1/{run}/metrics.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(host_metrics["schema"], "bboxy.dream.c1.host-metrics/1");
    assert_eq!(host_metrics["run_id"], run);
    assert_eq!(host_metrics["child_usage_observed"], true);
    assert!(
        host_metrics["max_unit_cpu_ns"].as_u64().unwrap() <= 1_000_000,
        "host compute unit exceeded1ms: {host_metrics}"
    );
    assert_eq!(host_metrics["unit_overruns"], 0);
    assert_eq!(
        digest(&fs::read(&source_path).unwrap()),
        original_hash,
        "Dream changed original source"
    );
    let repeated = f.ctl(&["dream", "inspect", &own, &selected, "--op", inspect_op]);
    assert_eq!(
        repeated["run"]["run_id"], run,
        "same Inspect restarted/changed run"
    );
    let alias = f.ctl(&[
        "dream",
        "inspect",
        &own,
        &selected,
        "--op",
        "00000000000000000000000022",
    ]);
    assert_eq!(alias["run"]["run_id"], run, "SourceKey duplicated replay");
    let until = Instant::now() + Duration::from_secs(45);
    let native_exit = loop {
        if let Some(exit) = native.process.try_wait().unwrap() {
            break exit;
        }
        if Instant::now() >= until {
            let _ = native.process.kill();
            let _ = native.process.wait();
            panic!("private native C1 frame witness timed out");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        native_exit.success(),
        "native frame witness failed: {}",
        fs::read_to_string(f.root.join("native-frames.log")).unwrap_or_default()
    );
    let frames: Value = serde_json::from_slice(&fs::read(&frame_report).unwrap()).unwrap();
    assert_eq!(frames["timing_gated"], true);
    assert!(
        frames["failures"].as_array().unwrap().is_empty(),
        "{frames}"
    );
    assert!(
        frames["metrics"]["frames"].as_u64().unwrap() >= 240,
        "insufficient actual native GPU frame samples: {frames}"
    );
    assert!(frames["metrics"]["cpu_p99_ms"].as_f64().unwrap() <= 4.);
    assert!(
        frames["metrics"]["input_to_gpu_complete_ms"]
            .as_f64()
            .unwrap()
            <= 16.7
    );
    write_evidence(
        "runtime-native-ctl.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["closed_only_real_ctl_admission","source_key_manual_dedup","own_seat_real_mcp_status_cancel","held_real_worker_control_latency","native_frame_overlap","same_app_native_frame_pipeline","independent_root_progress","cancel_actual_exit"],
            "not_established":["held_source_read","cpu_heavy_progress_flood","reactor_cpu_units","attributed_parent_child_heap","hot_owner_busy_wait_controls",],
            "run":run,"source_sha256":original_hash,"source_bytes":source_bytes,"worker_pid":pid,"roundtrips":roundtrips,"cancel_roundtrips":cancels,
            "owned_hello_receipt":started,"public_running_before_hold":started["public_running_after_validated_progress"],
            "native":{"process_exit":native_exit.code(),"frames":frames["metrics"]["frames"],"cpu_p99_ms":frames["metrics"]["cpu_p99_ms"],"input_to_gpu_ms":frames["metrics"]["input_to_gpu_complete_ms"],"fixture_report_sha256":digest(&fs::read(frame_report).unwrap()),"held_worker_entire_interval":true,"same_app_pipeline":true,"overlap":overlap},
            "settled_state":terminal["run"]["state"],"source_unchanged":true,"actual_original_process_absent":true,
            "independent_root":independent,"host_metrics":host_metrics,"host_metrics_scope":"tracked requested buffer capacity and parent compute units; child wait4 process CPU/RSS; total attributed heap and thread CPU remain unproven",
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_real_completed_status_rejects_valid_json_page_tamper() {
    let mut f = Fixture::scripted();
    f.start();
    let prepared = prepare_source(&f, "DreamImmutable", 1);
    let source_pid = row(&f, &prepared.seat)["state"]["process"]["pid"]
        .as_u64()
        .and_then(|pid| u32::try_from(pid).ok())
        .unwrap();
    let source_start = bbt_mux::adapters::dream_c1::process_stamp(source_pid).unwrap();
    let mut source = close_prepared(&f, prepared);
    wait(
        "durable ordinary close followed by actual exact source-process absence",
        || {
            (!bbt_mux::adapters::dream_c1::process_stamp(source_pid)
                .is_ok_and(|stamp| stamp == source_start))
            .then_some(())
        },
    );
    let inspect_op = "00000000000000000000000024";
    let admitted = inspect_closed(&f, &source, inspect_op);
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let terminal = wait("real worker completed and settled", || {
        let value = f.ctl(&["dream", "status", &source.seat, &run]);
        (value["run"]["settled"] == true).then_some(value)
    });
    assert_eq!(
        terminal["run"]["state"],
        "completed",
        "{terminal}\n{}",
        f.log()
    );
    let (pid, start) = journal_worker(&f, &source.seat, &run)
        .expect("completed real worker lacks retained identity");
    assert!(
        !bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == start),
        "Completed preceded actual worker reap"
    );
    let page = f.ctl(&["dream", "status", &source.seat, &run, "--page", "0"]);
    assert_eq!(page["ok"], true, "{page}");
    assert!(
        !page["page"]["rows"].as_array().unwrap().is_empty(),
        "missing actual completed rows"
    );
    let path = f
        .root
        .join(format!("run/dream-c1/{run}/output/page-000000.json"));
    let original = fs::read(&path).unwrap();
    let result_path = path.parent().unwrap().join("result.json");
    let result_hash = digest(&fs::read(&result_path).unwrap());
    let mut altered: Value = serde_json::from_slice(&original).unwrap();
    altered["rows"] = json!([]);
    let bytes = serde_json::to_vec(&altered).unwrap();
    assert_ne!(digest(&bytes), digest(&original));
    fs::write(&path, &bytes).unwrap();
    // Run/index/manifest remain exact and the page still has a valid wire shape.
    // The actual public API must check the retained descriptor/page hashes.
    let ctl = f.ctl(&["dream", "status", &source.seat, &run, "--page", "0"]);
    assert_eq!(
        ctl["error"]["code"], "protocol_error",
        "edited rows escaped ctl: {ctl}"
    );
    let mut peer = source.mcp.take().unwrap();
    let mcp = mcp_control(
        &mut peer,
        3,
        "00000000000000000000000025",
        json!({"v":1,"method":"status","run_id":run,"page":0}),
    );
    assert_eq!(
        mcp["error"]["code"], "protocol_error",
        "edited rows escaped MCP: {mcp}"
    );
    assert_eq!(
        digest(&fs::read(result_path).unwrap()),
        result_hash,
        "fixture changed result descriptor"
    );
    fs::write(&path, &original).unwrap();
    let restored = f.ctl(&["dream", "status", &source.seat, &run, "--page", "0"]);
    assert_eq!(
        restored["page"], page["page"],
        "exact restored immutable bytes did not recover"
    );
    assert_eq!(
        digest(&fs::read(&source.path).unwrap()),
        source.sha256,
        "Dream changed original source"
    );
    let retry = inspect_closed(&f, &source, inspect_op);
    assert_eq!(
        retry["run"]["run_id"], run,
        "completed original operation replayed"
    );
    write_evidence(
        "runtime-page-tamper.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["immutable_page_tamper_api"],"run":run,
            "original_page_sha256":digest(&original),"altered_page_sha256":digest(&bytes),
            "result_sha256":result_hash,"source_sha256":source.sha256,
            "ctl_error":ctl["error"]["code"],"mcp_error":mcp["error"]["code"],
            "restored_exact_page":true,"source_unchanged":true,"actual_original_process_absent":true,
            "ordinary_closed_source_exited":true,"source_process_pid":source_pid,"source_process_start":source_start,
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_real_daemon_loss_quarantines_predecessor_without_replay_or_adoption() {
    let mut f = Fixture::scripted();
    f.start();
    let source = close_source(&f, "DreamRestart", 16);
    let original_op = "00000000000000000000000026";
    let admitted = inspect_closed(&f, &source, original_op);
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    let started = running_worker_proof(&f, &source.seat, &run);
    let (pid, stamp) = journal_worker(&f, &source.seat, &run).unwrap();
    assert_eq!(started["run_id"], run);
    assert_eq!(started["worker_pid"], pid);
    assert_eq!(started["worker_start"], stamp);
    let held = HeldWorker::stop(pid, stamp);
    let (before, count) = journal_record(&f, &source.seat, &run);
    assert_eq!(count, 1, "private source already duplicated a run");
    assert_eq!(
        before["started"], true,
        "run lacked a durable pre-effect fence"
    );
    assert_eq!(before["nonce"], started["nonce"]);
    assert_eq!(before["worker_pid"], pid);
    assert!(before["view"]["manifest_sha256"].as_str().is_some());
    assert_eq!(before["view"]["settled"], false);
    let original_manifest = digest(&fs::read(job.join("input/manifest.json")).unwrap());
    let logic = owner_pid(&f, "logic");
    let supervisor = owner_pid(&f, "supervisor");
    let logic_stamp = bbt_mux::adapters::dream_c1::process_stamp(logic).unwrap();
    // Stop only our recorded frontend first: it must not race the deliberate
    // daemon-loss cut by automatically starting a replacement logic process.
    let mut frontend = f.app.take().unwrap();
    frontend.kill().unwrap();
    frontend.wait().unwrap();
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(logic).unwrap(),
        logic_stamp
    );
    unsafe {
        assert_eq!(libc::kill(logic as i32, libc::SIGKILL), 0);
    }
    wait("exact private logic daemon exit", || {
        (!bbt_mux::adapters::dream_c1::process_stamp(logic)
            .is_ok_and(|current| current == logic_stamp))
        .then_some(())
    });
    assert!(
        held.current(),
        "daemon-loss witness did not retain the predecessor child"
    );
    f.start();
    let next_logic = owner_pid(&f, "logic");
    assert_ne!(
        next_logic, logic,
        "UI restart substituted for actual daemon loss"
    );
    assert_eq!(
        owner_pid(&f, "supervisor"),
        supervisor,
        "logic-loss fixture replaced supervisor"
    );
    let old = f.ctl(&[
        "dream",
        "inspect",
        &source.seat,
        &source.selected,
        "--op",
        original_op,
    ]);
    assert_eq!(
        old["error"]["code"], "stale_context",
        "new daemon presented old-context result: {old}"
    );
    assert!(old.get("run").is_none());
    let new = f.ctl(&[
        "dream",
        "inspect",
        &source.seat,
        &source.selected,
        "--op",
        "00000000000000000000000027",
    ]);
    assert_eq!(
        new["error"]["code"], "stale_context",
        "new operation adopted predecessor: {new}"
    );
    assert!(new.get("run").is_none());
    let status = f.ctl(&["dream", "status", &source.seat, &run]);
    assert!(
        matches!(
            status["error"]["code"].as_str(),
            Some("not_found" | "stale_context")
        ),
        "new authority received predecessor status: {status}"
    );
    assert!(status.get("run").is_none());
    let (after, count) = journal_record(&f, &source.seat, &run);
    assert_eq!(
        count, 1,
        "restart admitted another run for the same SourceKey"
    );
    assert_eq!(after["view"]["state"], "quarantined");
    assert_eq!(
        after["view"]["settled"], false,
        "unverified predecessor certified settled"
    );
    assert_eq!(after["started"], true);
    for key in [
        "worker_pid",
        "worker_start",
        "owner_incarnation",
        "identity_fence",
        "op_id",
        "nonce",
        "request",
    ] {
        assert_eq!(after[key], before[key], "restart changed retained {key}");
    }
    assert!(held.current(), "retry replaced or adopted original child");
    assert_eq!(
        digest(&fs::read(&source.path).unwrap()),
        source.sha256,
        "retry replay changed source"
    );
    assert_eq!(
        digest(&fs::read(job.join("input/manifest.json")).unwrap()),
        original_manifest,
        "restart rebuilt original manifest"
    );
    // Explicit fixture cleanup is not product settlement or restart adoption.
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(pid).unwrap(),
        held.start
    );
    unsafe {
        assert_eq!(libc::kill(pid as i32, libc::SIGKILL), 0);
    }
    wait("exact private orphan fixture cleanup", || {
        (!held.current()).then_some(())
    });
    write_evidence(
        "runtime-restart.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["durable_close_restart_dedup","restart_predecessor_quarantine"],
            "run":run,"worker_pid":pid,"worker_start":held.start,"owned_hello_receipt":started,
            "public_running_before_restart":started["public_running_after_validated_progress"],
            "source_sha256":source.sha256,"source_unchanged":true,"manifest_unchanged":true,
            "old_logic_pid":logic,"new_logic_pid":next_logic,"supervisor_unchanged":true,
            "original_op_error":old["error"]["code"],"new_op_error":new["error"]["code"],
            "status_error":status["error"]["code"],"runs_before":1,"runs_after":count,
            "retained_nonce":after["nonce"],"retained_owner_incarnation":after["owner_incarnation"],
            "predecessor_state":after["view"]["state"],"predecessor_settled":false,
            "no_new_worker":true,"no_new_context_result":true,"fixture_exact_orphan_cleanup":true,
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_four_live_real_jobs_keep_independent_controls_and_exact_settlement() {
    let mut f = Fixture::scripted();
    f.start();
    let mut prepared = Vec::new();
    for index in 0..4 {
        let repository = f.root.join(format!("parallel-root-{index}"));
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(repository.join(".bboxy"))
            .unwrap();
        let git = Command::new("/usr/bin/git")
            .args(["init", "-q"])
            .arg(&repository)
            .env_clear()
            .env("HOME", f.root.join("home"))
            .env("PATH", "/usr/bin:/bin")
            .output()
            .unwrap();
        assert!(git.status.success());
        let layout = if index == 0 {
            f.ctl(&["ui", "layout"])
        } else {
            let layout = f.ctl(&["ui", "layout"]);
            let region = layout["regions"]["new-swarm"]
                .as_array()
                .expect("private headless new-swarm control absent");
            let x = (region[0].as_f64().unwrap() + region[2].as_f64().unwrap() / 2.)
                .round()
                .to_string();
            let y = (region[1].as_f64().unwrap() + region[3].as_f64().unwrap() / 2.)
                .round()
                .to_string();
            f.ctl(&["ui", "click", &x, &y])
        };
        let swarm = layout["swarm"].as_str().unwrap();
        f.ctl(&["ui", "swarm-workdir", swarm, repository.to_str().unwrap()]);
        prepared.push(prepare_source(&f, &format!("DreamParallel{index}"), 16));
    }
    // All real sessions are prepared before any close. Admit all four before
    // waiting for a paced snapshot/worker Start, so the fixed30s wall budgets
    // overlap rather than serialising four preparations behind the first job.
    let sources = prepared
        .into_iter()
        .map(|source| close_prepared(&f, source))
        .collect::<Vec<_>>();
    let mut runs = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        let op = bbt_mux::Ulid::from(40u128 + index as u128).to_string();
        let admitted = inspect_closed(&f, source, &op);
        runs.push(admitted["run"]["run_id"].as_str().unwrap().to_owned());
    }
    let mut held = Vec::new();
    let mut contexts = Vec::new();
    let mut work_proofs = Vec::new();
    for (source, run) in sources.iter().zip(&runs) {
        let (pid, stamp) = wait("four actual independent worker owners", || {
            journal_worker(&f, &source.seat, run)
        });
        let acknowledged = running_worker_proof(&f, &source.seat, run);
        assert_eq!(acknowledged["worker_pid"], pid);
        assert_eq!(acknowledged["worker_start"], stamp);
        assert_eq!(acknowledged["run_id"], run.as_str());
        work_proofs.push(acknowledged);
        held.push(HeldWorker::stop(pid, stamp));
        let (record, count) = journal_record(&f, &source.seat, run);
        assert_eq!(count, 1);
        contexts.push(record["context"].clone());
    }
    assert_eq!(held.len(), 4);
    let accounts = contexts
        .iter()
        .map(|v| v["account"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    let roots = contexts
        .iter()
        .map(|v| v["root"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        accounts.len(),
        1,
        "four-job witness used different accounts"
    );
    assert_eq!(
        roots.len(),
        4,
        "four-job witness did not isolate four roots"
    );
    assert_eq!(
        held.iter()
            .map(|v| v.pid)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
    assert!(
        held.iter().all(HeldWorker::current),
        "first worker expired before all four were live"
    );
    let mut rows = Vec::new();
    for index in 0..4 {
        let source = &sources[index];
        let run = &runs[index];
        let seat = source.seat.parse().unwrap();
        let mut peer = DreamPeer::connect(&f.root);
        let statuses = summary(
            (0..200)
                .map(|_| {
                    let at = Instant::now();
                    let reply = peer.ask(seat, status_request(run, None));
                    assert_eq!(reply["ok"], true, "{reply}");
                    assert_eq!(reply["run"]["settled"], false);
                    at.elapsed().as_secs_f64() * 1000.
                })
                .collect(),
        );
        assert!(
            held.iter().skip(index).all(HeldWorker::current),
            "one active owner was released by another root's status"
        );
        let op = bbt_mux::Ulid::from(50u128 + index as u128).to_string();
        let request = bbt_proto::dream::Control::Cancel(bbt_proto::dream::Cancel {
            v: bbt_proto::dream::Version,
            op_id: bbt_proto::dream::Ulid(op),
            run_id: bbt_proto::dream::Ulid(run.clone()),
        });
        let cancels = summary(
            (0..200)
                .map(|_| {
                    let at = Instant::now();
                    let reply = peer.ask(seat, request.clone());
                    assert_eq!(reply["ok"], true, "{reply}");
                    at.elapsed().as_secs_f64() * 1000.
                })
                .collect(),
        );
        let terminal = wait(
            "independent four-job cancellation settles after exit",
            || {
                let value = f.ctl(&["dream", "status", &source.seat, run]);
                (value["run"]["settled"] == true).then_some(value)
            },
        );
        assert_eq!(terminal["run"]["state"], "cancelled");
        assert!(
            !held[index].current(),
            "four-job Cancelled preceded actual process exit"
        );
        assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
        let metrics: Value = serde_json::from_slice(
            &fs::read(runtime_path(&f).join(format!("dream-c1/{run}/metrics.json"))).unwrap(),
        )
        .unwrap();
        rows.push(json!({"run_id":run,"worker_pid":held[index].pid,"worker_start":held[index].start,
            "root":contexts[index]["root"],"account":contexts[index]["account"],
            "owned_hello_receipt":work_proofs[index],"public_running_before_hold":work_proofs[index]["public_running_after_validated_progress"],
            "status_roundtrips":statuses,"cancel_roundtrips":cancels,"actual_reap":true,"source_unchanged":true,"host_metrics":metrics}));
    }
    write_evidence(
        "runtime-four-jobs.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["four_live_jobs_controls"],"four_simultaneous_worker_incarnations":true,"jobs":rows,
            "heap_scope":"per-job tracked metrics only; no assertion of total parent live heap from these counters",
        }),
    );
}

struct SourceReadHold {
    path: PathBuf,
    released: bool,
}
impl SourceReadHold {
    fn release(&mut self) {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        if self.released {
            return;
        }
        if let Ok(mut writer) = fs::OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
            .open(&self.path)
            && writer.write_all(b"r").is_ok()
        {
            self.released = true;
        }
    }
}
impl Drop for SourceReadHold {
    fn drop(&mut self) {
        self.release();
    }
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_actual_source_read_hold_keeps_controls_and_cancel_owner_until_release() {
    let mut f = Fixture::scripted_fault("hold-source");
    f.start();
    let probe = f.ctl(&[
        "seat",
        "spawn",
        "zsh",
        "--place",
        "tab",
        "--name",
        "DreamHeldInput",
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let source = close_source(&f, "DreamHeldSource", 1);
    let admitted = inspect_closed(&f, &source, "00000000000000000000000060");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    let mut hold = SourceReadHold {
        path: job.join("test-source-read.fifo"),
        released: false,
    };
    let witness = wait(
        "actual validated source owner blocked on private FIFO",
        || serde_json::from_slice::<Value>(&fs::read(job.join("source-read-held.json")).ok()?).ok(),
    );
    assert_eq!(witness["owner"], "snapshot");
    assert_eq!(witness["source_fd_validated"], true);
    assert_eq!(witness["fifo_read"], true);
    let (pid, stamp) = journal_worker(&f, &source.seat, &run).unwrap();
    let roundtrips = measured_controls(&f, &probe, &source.seat, &run);
    let mut peer = DreamPeer::connect(&f.root);
    let seat = source.seat.parse().unwrap();
    let cancel = bbt_proto::dream::Control::Cancel(bbt_proto::dream::Cancel {
        v: bbt_proto::dream::Version,
        op_id: bbt_proto::dream::Ulid("00000000000000000000000061".into()),
        run_id: bbt_proto::dream::Ulid(run.clone()),
    });
    let cancels = summary(
        (0..200)
            .map(|_| {
                let at = Instant::now();
                let value = peer.ask(seat, cancel.clone());
                assert_eq!(value["ok"], true, "{value}");
                assert_eq!(
                    value["run"]["settled"], false,
                    "held source owner was released early"
                );
                at.elapsed().as_secs_f64() * 1000.
            })
            .collect(),
    );
    let blocked = peer.ask(seat, status_request(&run, None));
    assert_eq!(
        blocked["run"]["settled"], false,
        "cancel detached the blocked source owner"
    );
    hold.release();
    assert!(
        hold.released,
        "private source FIFO read owner disappeared before explicit release"
    );
    let terminal = wait(
        "actual held source owner joins before cancellation settlement",
        || {
            let value = f.ctl(&["dream", "status", &source.seat, &run]);
            (value["run"]["settled"] == true).then_some(value)
        },
    );
    assert_eq!(terminal["run"]["state"], "cancelled");
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == stamp));
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    let metrics: Value =
        serde_json::from_slice(&fs::read(job.join("metrics.json")).unwrap()).unwrap();
    assert_eq!(metrics["rest_skips"], 0);
    assert_eq!(metrics["rest_violations"], 0);
    assert!(metrics["max_unit_cpu_ns"].as_u64().unwrap() <= 1_000_000);
    write_evidence(
        "runtime-source-hold.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["held_source_read"],"run":run,"source_read_held":witness,"roundtrips":roundtrips,"cancel_roundtrips":cancels,
            "retained_unsettled_until_source_release":true,"actual_reap":true,"source_unchanged":true,"host_metrics":metrics,
            "foreground_pause_established":false,
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_real_omit_rest_control_is_rejected_by_measured_schedule_invariant() {
    let mut f = Fixture::scripted_fault("omit-rest");
    f.start();
    let source = close_source(&f, "DreamBrokenRest", 1);
    let admitted = inspect_closed(&f, &source, "00000000000000000000000062");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let terminal = wait("actual REST mutant process and owners settle", || {
        let value = f.ctl(&["dream", "status", &source.seat, &run]);
        (value["run"]["settled"] == true).then_some(value)
    });
    let (pid, stamp) = journal_worker(&f, &source.seat, &run).unwrap();
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == stamp));
    let metrics: Value = serde_json::from_slice(
        &fs::read(runtime_path(&f).join(format!("dream-c1/{run}/metrics.json"))).unwrap(),
    )
    .unwrap();
    assert!(
        metrics["rest_skips"].as_u64().unwrap() > 0,
        "fault did not exercise actual compute owner: {metrics}"
    );
    assert!(
        metrics["rest_violations"].as_u64().unwrap() > 0,
        "omitted rest escaped independent gap accounting: {metrics}"
    );
    assert!(
        metrics["min_inter_unit_gap_ns"].as_u64().unwrap() < 9_000_000,
        "mutant did not remove the prescribed9ms rest"
    );
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    write_evidence(
        "runtime-busy-wait-control.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["busy_wait_causal_negative"],"run":run,"verdict":"schedule_budget_failed",
            "actual_reap":true,"source_unchanged":true,"terminal_state":terminal["run"]["state"],"host_metrics":metrics,
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_production_cpu_and_progress_keep_actual_controls_responsive() {
    let mut f = Fixture::scripted();
    f.start();
    let probe = f.ctl(&[
        "seat",
        "spawn",
        "zsh",
        "--place",
        "tab",
        "--name",
        "DreamCpuInput",
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let source = close_source(&f, "DreamProductionCpu", 16);
    assert!(source.bytes >= 2 * 1024 * 1024);
    let admitted = inspect_closed(&f, &source, "00000000000000000000000067");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    let started = running_worker_proof(&f, &source.seat, &run);
    let (pid, stamp) = journal_worker(&f, &source.seat, &run).unwrap();
    assert_eq!(started["worker_pid"], pid);
    assert_eq!(started["worker_start"], stamp);
    let before = live_process_usage(pid, &stamp);
    let controls_at = Instant::now();
    // This explicit sampling gap increases the observed load interval; each
    // round-trip timer still encloses only the actual input/message/Status call.
    let controls =
        measured_controls_with_gap(&f, &probe, &source.seat, &run, Duration::from_millis(1));
    let controls_wall_ns = controls_at.elapsed().as_nanos() as u64;
    let after = live_process_usage(pid, &stamp);
    assert_eq!(
        before["kernel_start_abstime"],
        after["kernel_start_abstime"]
    );
    let cpu = |value: &Value| {
        value["user_cpu_ns"].as_u64().unwrap() + value["system_cpu_ns"].as_u64().unwrap()
    };
    let observed_cpu_ns = cpu(&after).checked_sub(cpu(&before)).unwrap();
    assert!(
        observed_cpu_ns > 0,
        "production child did no measured CPU work during the200 control round trips"
    );
    let terminal = wait(
        "production CPU witness completes and actually reaps",
        || {
            let value = f.ctl(&["dream", "status", &source.seat, &run]);
            (value["run"]["settled"] == true).then_some(value)
        },
    );
    assert_eq!(terminal["run"]["state"], "completed", "{terminal}");
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == stamp));
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    let metrics: Value =
        serde_json::from_slice(&fs::read(job.join("metrics.json")).unwrap()).unwrap();
    assert_eq!(metrics["producer_metrics_observed"], true);
    assert!(metrics["producer_compute_cpu_ns"].as_u64().unwrap() > 0);
    assert!(metrics["producer_envelopes"].as_u64().unwrap() > 0);
    assert!(metrics["max_unit_cpu_ns"].as_u64().unwrap() <= 1_000_000);
    assert_validated_producer_compute(&metrics);
    assert_eq!(metrics["unit_overruns"], 0);
    assert_eq!(metrics["producer_overruns"], 0);
    assert_eq!(metrics["rest_skips"], 0);
    assert_eq!(metrics["rest_violations"], 0);
    write_evidence(
        "runtime-production-cpu.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["production_cpu_progress_controls"],"protocol_fake":false,"run":run,
            "worker_pid":pid,"worker_start":stamp,"owned_hello_receipt":started,"source_bytes":source.bytes,
            "public_running_before_controls":started["public_running_after_validated_progress"],
            "controls":controls,"control_sample_gap_ns":1_000_000,"controls_wall_ns":controls_wall_ns,
            "process_usage_before":before,"process_usage_after":after,"child_cpu_during_controls_ns":observed_cpu_ns,
            "actual_reap":true,"source_unchanged":true,"host_metrics":metrics,
            "claim":"genuine production process CPU overlap and bounded progress, not an unbounded flood or thread CPU measurement",
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_protocol_fake_progress_flood_keeps_controls_and_cancel_independent() {
    use bbt_proto::dream as w;
    let mut f = Fixture::scripted_progress_flood();
    let fault_sha256 = digest(&fs::read(f.root.join("bin/bboxy450-harness")).unwrap());
    f.start();
    let probe = f.ctl(&[
        "seat",
        "spawn",
        "zsh",
        "--place",
        "tab",
        "--name",
        "DreamFloodInput",
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    let source = close_source(&f, "DreamFaultFlood", 1);
    let admitted = inspect_closed(&f, &source, "00000000000000000000000068");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    let started = running_worker_proof(&f, &source.seat, &run);
    let (pid, stamp) = journal_worker(&f, &source.seat, &run).unwrap();
    assert_eq!(started["worker_pid"], pid);
    assert_eq!(started["worker_start"], stamp);
    let before = live_process_usage(pid, &stamp);
    let controls =
        measured_controls_with_gap(&f, &probe, &source.seat, &run, Duration::from_millis(1));
    let after = live_process_usage(pid, &stamp);
    let cpu = |value: &Value| {
        value["user_cpu_ns"].as_u64().unwrap() + value["system_cpu_ns"].as_u64().unwrap()
    };
    let fault_cpu_ns = cpu(&after).checked_sub(cpu(&before)).unwrap();
    assert!(
        fault_cpu_ns > 0,
        "protocol-fake process was inactive during the measured controls"
    );
    let own = source.seat.parse::<bbt_mux::Ulid>().unwrap();
    let cancel = w::Control::Cancel(w::Cancel {
        v: w::Version,
        op_id: w::Ulid::try_from("00000000000000000000000069".to_owned()).unwrap(),
        run_id: w::Ulid(run.clone()),
    });
    let mut peer = DreamPeer::connect(&f.root);
    let cancel_controls = summary(
        (0..200)
            .map(|_| {
                let at = Instant::now();
                let value = peer.ask(own, cancel.clone());
                assert_eq!(value["ok"], true, "{value}");
                at.elapsed().as_secs_f64() * 1000.
            })
            .collect(),
    );
    let terminal = wait("protocol-fake flood Cancel/EOF and actual reap", || {
        let value = f.ctl(&["dream", "status", &source.seat, &run]);
        (value["run"]["settled"] == true).then_some(value)
    });
    assert_eq!(terminal["run"]["state"], "cancelled", "{terminal}");
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == stamp));
    let fault: Value =
        serde_json::from_slice(&fs::read(job.join("protocol-flood.json")).unwrap()).unwrap();
    assert_eq!(fault["protocol_fake"], true);
    assert!(fault["generated_progress_frames"].as_u64().unwrap() >= 200);
    assert!(fault["fully_written_progress_frames"].as_u64().unwrap() >= 200);
    assert!(fault["sent_framed_bytes"].as_u64().unwrap() > 0);
    assert!(fault["max_pending_bytes"].as_u64().unwrap() <= 66_048);
    assert!(fault["cancel_received"] == true || fault["eof_received"] == true);
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    write_evidence(
        "runtime-protocol-progress-flood.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["protocol_fake_progress_flood_controls"],"protocol_fake":true,"production_child_executed":false,
            "fault_sha256":fault_sha256,"fault_path":f.root.join("bin/bboxy450-harness"),"fault_receipt":fault,
            "run":run,"worker_pid":pid,"worker_start":stamp,"owned_hello_receipt":started,
            "public_running_before_controls":started["public_running_after_validated_progress"],
            "controls":controls,"cancel_controls":cancel_controls,"process_usage_before":before,"process_usage_after":after,
            "fault_process_cpu_during_controls_ns":fault_cpu_ns,"actual_reap":true,"source_unchanged":true,
            "claim":"bounded protocol-fake valid-progress traffic control witness; no production worker conformance/CPU/heap claim",
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_hot_owner_mutant_refuses_snapshot_custody_before_source_effect() {
    let mut f = Fixture::scripted_fault("hot-owner");
    f.start();
    let source = close_source(&f, "DreamBrokenOwner", 1);
    let admitted = inspect_closed(&f, &source, "00000000000000000000000066");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let terminal = wait(
        "actual snapshot custody refusal and Hello child reap",
        || {
            let value = f.ctl(&["dream", "status", &source.seat, &run]);
            (value["run"]["settled"] == true).then_some(value)
        },
    );
    assert_eq!(terminal["run"]["state"], "failed", "{terminal}");
    assert_eq!(
        terminal["run"]["error"]["code"], "resource_limit",
        "{terminal}"
    );
    assert!(terminal["run"]["manifest_sha256"].is_null());
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    let marker = fs::symlink_metadata(job.join("test-hot-owner")).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert!(marker.is_file());
    assert_eq!(marker.len(), 0);
    assert_eq!(marker.nlink(), 1);
    assert_eq!(marker.mode() & 0o777, 0o600);
    assert_eq!(marker.uid(), unsafe { libc::geteuid() });
    assert!(!job.join("input/manifest.json").exists());
    assert!(!job.join("worker-started.json").exists());
    let (pid, stamp) = journal_worker(&f, &source.seat, &run)
        .expect("custody fault must retain the real Hello child's PID/start");
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == stamp));
    let metrics: Value =
        serde_json::from_slice(&fs::read(job.join("metrics.json")).unwrap()).unwrap();
    assert!(
        metrics["source_owner_refusals"].as_u64().unwrap() > 0,
        "snapshot owner mismatch did not reach the actual custody guard: {metrics}"
    );
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    let retry = inspect_closed(&f, &source, "00000000000000000000000066");
    assert_eq!(retry["run"]["run_id"], run);
    assert_eq!(retry["run"]["state"], "failed");
    write_evidence(
        "runtime-hot-owner-control.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["hot_owner_custody_causal_negative"],"run":run,"verdict":"snapshot_owner_custody_refused",
            "terminal_state":terminal["run"]["state"],"error":"resource_limit","manifest_published":false,
            "worker_start_observed":false,"hello_child_pid":pid,"hello_child_start":stamp,"actual_reap":true,
            "source_unchanged":true,"original_op_retained":true,"host_metrics":metrics,
            "claim":"actual snapshot custody guard; no deliberate UI work or UI-latency counterexample",
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_real_owned_blob_over_limit_refuses_before_manifest_or_worker_start() {
    const LARGE: u64 = 65 * 1024 * 1024;
    let mut f = Fixture::scripted_blob(LARGE);
    f.start();
    let source = close_source(&f, "DreamOverLimit", 1);
    let blobs = f.root.join(format!(
        "home/.bboxy450-harness/sessions/{}/blobs",
        source.session
    ));
    let blob = fs::read_dir(&blobs)
        .unwrap()
        .map(|v| v.unwrap().path())
        .find(|p| fs::metadata(p).is_ok_and(|m| m.len() == LARGE))
        .expect("actual Store/exec did not publish the requested65MiB blob");
    let before = fs::metadata(&blob).unwrap();
    use std::os::unix::fs::MetadataExt;
    let stamp = (
        before.dev(),
        before.ino(),
        before.len(),
        before.mtime(),
        before.mtime_nsec(),
        before.ctime(),
        before.ctime_nsec(),
    );
    let admitted = inspect_closed(&f, &source, "00000000000000000000000063");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let terminal = wait(
        "real over-limit source refusal and actual worker reap",
        || {
            let value = f.ctl(&["dream", "status", &source.seat, &run]);
            (value["run"]["settled"] == true).then_some(value)
        },
    );
    assert_eq!(terminal["run"]["state"], "failed", "{terminal}");
    assert_eq!(
        terminal["run"]["error"]["code"], "resource_limit",
        "{terminal}"
    );
    assert!(
        terminal["run"]["manifest_sha256"].is_null(),
        "over-limit source published a manifest"
    );
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    assert!(
        !job.join("worker-started.json").exists(),
        "over-limit source reached framed worker Start"
    );
    assert!(!job.join("input/manifest.json").exists());
    let (pid, process) = journal_worker(&f, &source.seat, &run).unwrap();
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == process));
    let after = fs::metadata(&blob).unwrap();
    assert_eq!(
        stamp,
        (
            after.dev(),
            after.ino(),
            after.len(),
            after.mtime(),
            after.mtime_nsec(),
            after.ctime(),
            after.ctime_nsec()
        ),
        "consumer changed source blob"
    );
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    let metrics: Value =
        serde_json::from_slice(&fs::read(job.join("metrics.json")).unwrap()).unwrap();
    assert!(
        metrics["max_unit_cpu_ns"].as_u64().unwrap() <= 1_000_000,
        "refusal produced a compute burst: {metrics}"
    );
    write_evidence(
        "runtime-over-limit.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["large_input_over_limit"],"run":run,"actual_store_blob_bytes":LARGE,"bounded_ir_bytes":source.bytes,
            "error":"resource_limit","manifest_published":false,"worker_start_observed":false,"actual_reap":true,
            "source_unchanged":true,"blob_metadata_unchanged":true,"blob_digest":blob.file_name().unwrap().to_str().unwrap(),"host_metrics":metrics,
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_real_multimib_store_blob_completes_and_retains_exact_private_input() {
    const BYTES: u64 = 2 * 1024 * 1024;
    let mut f = Fixture::scripted_blob(BYTES);
    f.start();
    let source = close_source(&f, "DreamHeapBlob", 1);
    let blobs = f.root.join(format!(
        "home/.bboxy450-harness/sessions/{}/blobs",
        source.session
    ));
    let blob = fs::read_dir(&blobs)
        .unwrap()
        .map(|v| v.unwrap().path())
        .find(|p| fs::metadata(p).is_ok_and(|m| m.len() == BYTES))
        .expect("actual Store/exec did not publish the requested2MiB blob");
    let hash = digest(&fs::read(&blob).unwrap());
    assert_eq!(
        blob.file_name().unwrap().to_str().unwrap(),
        hash,
        "producer Store blob digest differs from actual bytes"
    );
    let admitted = inspect_closed(&f, &source, "00000000000000000000000064");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let terminal = wait("actual multi-MiB source and framed worker complete", || {
        let value = f.ctl(&["dream", "status", &source.seat, &run]);
        (value["run"]["settled"] == true).then_some(value)
    });
    assert_eq!(terminal["run"]["state"], "completed", "{terminal}");
    let (pid, stamp) = journal_worker(&f, &source.seat, &run).unwrap();
    assert!(!bbt_mux::adapters::dream_c1::process_stamp(pid).is_ok_and(|value| value == stamp));
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    assert_eq!(digest(&fs::read(&blob).unwrap()), hash);
    let job = runtime_path(&f).join(format!("dream-c1/{run}"));
    let metrics: Value =
        serde_json::from_slice(&fs::read(job.join("metrics.json")).unwrap()).unwrap();
    assert_eq!(
        metrics["producer_metrics_observed"], true,
        "real producer did not emit bounded measurement evidence"
    );
    assert_eq!(metrics["rest_skips"], 0);
    assert_eq!(metrics["rest_violations"], 0);
    assert!(metrics["max_unit_cpu_ns"].as_u64().unwrap() <= 1_000_000);
    assert_validated_producer_compute(&metrics);
    assert_eq!(metrics["producer_overruns"], 0);
    // Keep this exact producer-owned workload outside Git for source-local
    // isolated allocation/RSS/thread-CPU verification. The normal Drop still
    // stops only this fixture's actual private owners; it does not delete input.
    let replacement = tempfile::Builder::new()
        .prefix("dc1-cleanup-")
        .tempdir_in("/tmp")
        .unwrap();
    let retained = std::mem::replace(&mut f.directory, replacement).keep();
    assert_eq!(retained.canonicalize().unwrap(), f.root);
    write_evidence(
        "runtime-multimib-blob.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["real_multimib_blob_workload"],"run":run,"actual_reap":true,"source_unchanged":true,"blob_unchanged":true,
            "source_path":source.path,"session":source.session,"source_generation":source.generation,"source_sha256":source.sha256,
            "blob_path":blob,"blob_sha256":hash,"blob_bytes":BYTES,"fixture_root":retained,"job_path":job,"host_metrics":metrics,
            "heap_claim":"tracked capacity + observed producer metrics only; total parent attributed live heap remains separate",
        }),
    );
}

#[test]
#[ignore = "joined real-runner test: runs only via make test-dream-c1 (needs pinned DREAM_C1_RUNNER)"]
fn dream_c1_retained_same_generation_turn_pauses_real_worker_with_legacy_read_held() {
    let next = bbt_mux::Ulid::new().to_string();
    let mut f = Fixture::scripted_retained(&next);
    f.start();
    let prepared = prepare_source(&f, "DreamRetainedHost", 1);
    let seat = prepared.seat.clone();
    let before = row(&f, &seat);
    let generation = before["record"]["generation"].as_u64().unwrap();
    let host_pid = before["state"]["process"]["pid"]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .unwrap();
    let host_stamp = bbt_mux::adapters::dream_c1::process_stamp(host_pid).unwrap();
    let source = close_prepared_mode(&f, prepared, true);
    wait(
        "fresh nonfork B retains exact managed PID and generation",
        || {
            let current = row(&f, &seat);
            assert_eq!(current["record"]["generation"], generation);
            assert_eq!(
                current["state"]["process"]["kind"],
                "running",
                "retained fixture exited: {}",
                f.log()
            );
            assert_eq!(current["state"]["process"]["pid"], host_pid);
            (current["record"]["harness_session_id"] == next).then_some(())
        },
    );
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(host_pid).unwrap(),
        host_stamp
    );
    let next_path = f
        .root
        .join(format!("home/.bboxy450-harness/sessions/{next}.jsonl"));
    let header: Value = serde_json::from_slice(
        fs::read(&next_path)
            .unwrap()
            .split(|b| *b == b'\n')
            .next()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(header["event"]["type"], "session_started");
    assert_eq!(header["event"]["meta"]["session"], next);
    assert!(
        header["event"]["meta"]["fork"].is_null(),
        "B inherited a fork prefix"
    );
    let selected_b = format!("{next}:{generation}");
    wait(
        "fresh B open catalogue after retained Host transition",
        || {
            let value = f.ctl(&["dream", "inspect", &seat, &selected_b]);
            (value["error"]["code"] == "source_open").then_some(())
        },
    );
    let admitted = inspect_closed(&f, &source, "00000000000000000000000065");
    let run = admitted["run"]["run_id"].as_str().unwrap().to_owned();
    let started = running_worker_proof(&f, &seat, &run);
    let (worker_pid, worker_stamp) = journal_worker(&f, &seat, &run).unwrap();
    assert_eq!(started["run_id"], run);
    assert_eq!(started["worker_pid"], worker_pid);
    assert_eq!(started["worker_start"], worker_stamp);
    let fifo = runtime_path(&f).join(format!("dream-c1/test-legacy-{seat}.fifo"));
    use std::os::unix::ffi::OsStrExt;
    let native_path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    unsafe {
        assert_eq!(libc::mkfifo(native_path.as_ptr(), 0o600), 0);
    }
    let mut legacy = SourceReadHold {
        path: fifo,
        released: false,
    };
    let held_path = runtime_path(&f).join(format!("dream-c1/test-legacy-{seat}.held.json"));
    let held = wait("actual isolated legacy log read is held", || {
        serde_json::from_slice::<Value>(&fs::read(&held_path).ok()?).ok()
    });
    assert_eq!(held["owner"], "legacy-log-read");
    assert_eq!(held["real_fifo_read"], true);
    let own = seat.parse::<bbt_mux::Ulid>().unwrap();
    let mut peer = DreamPeer::connect(&f.root);
    let ready = peer.ask(own, status_request(&run, None));
    assert_eq!(ready["ok"], true, "{ready}");
    assert_eq!(
        ready["run"]["state"], "running",
        "A ended or deferred before foreground witness: {ready}"
    );
    assert_eq!(ready["run"]["settled"], false);
    f.ctl(&[
        "seat",
        "type",
        &seat,
        "Exercise retained same-seat foreground",
    ]);
    let at = Instant::now();
    let enter = f.raw(&["seat", "key", &seat, "enter"]);
    assert!(
        enter.status.success(),
        "admitted retained turn failed: {}",
        String::from_utf8_lossy(&enter.stderr)
    );
    let entered: Value = serde_json::from_slice(&enter.stdout).unwrap();
    assert_eq!(entered["ok"], true);
    let paused = loop {
        let value = peer.ask(own, status_request(&run, None));
        assert_eq!(value["ok"], true, "{value}");
        assert_eq!(
            value["run"]["settled"], false,
            "worker ended instead of pausing"
        );
        if value["run"]["state"] == "paused" {
            break value;
        }
        assert!(
            at.elapsed() <= Duration::from_millis(100),
            "actual foreground pause acknowledgement exceeded100ms"
        );
        std::thread::sleep(Duration::from_millis(1));
    };
    let pause_ms = at.elapsed().as_secs_f64() * 1000.;
    assert!(
        pause_ms <= 100.,
        "actual pause ack={pause_ms}ms exceeds100ms"
    );
    assert!(
        !legacy.released,
        "legacy source owner was not held through pause acknowledgement"
    );
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(host_pid).unwrap(),
        host_stamp
    );
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(worker_pid).unwrap(),
        worker_stamp
    );
    wait("B actual admitted TurnStarted", || {
        let text = fs::read_to_string(&next_path).ok()?;
        text.lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|v| v["event"]["type"] == "turn_started")
            .then_some(())
    });
    legacy.release();
    assert!(legacy.released);
    wait("real B foreground tool completes", || {
        let text = fs::read_to_string(&next_path).ok()?;
        text.lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .any(|v| v["event"]["type"] == "turn_ended")
            .then_some(())
    });
    let terminal = wait(
        "A real worker resumes and completes after B foreground",
        || {
            let value = f.ctl(&["dream", "status", &seat, &run]);
            (value["run"]["settled"] == true).then_some(value)
        },
    );
    assert_eq!(terminal["run"]["state"], "completed", "{terminal}");
    assert!(
        !bbt_mux::adapters::dream_c1::process_stamp(worker_pid)
            .is_ok_and(|value| value == worker_stamp)
    );
    assert_eq!(digest(&fs::read(&source.path).unwrap()), source.sha256);
    assert_eq!(
        bbt_mux::adapters::dream_c1::process_stamp(host_pid).unwrap(),
        host_stamp
    );
    let retry = inspect_closed(&f, &source, "00000000000000000000000065");
    assert_eq!(retry["run"]["run_id"], run);
    f.ctl(&["seat", "key", &seat, "ctrl-d"]);
    wait(
        "consumed retained transition restores ordinary B EOF exit",
        || {
            (!bbt_mux::adapters::dream_c1::process_stamp(host_pid)
                .is_ok_and(|value| value == host_stamp))
            .then_some(())
        },
    );
    write_evidence(
        "runtime-foreground-pause.json",
        &json!({
            "schema":"bboxy.dream.c1.runtime-evidence/1","driver_nonce":std::env::var("DREAM_C1_GATE_NONCE").unwrap(),
            "runner_sha256":digest(&fs::read(std::env::var_os("DREAM_C1_RUNNER").unwrap()).unwrap()),
            "established":["foreground_pause_100ms"],"run":run,"pause_ack_ms":pause_ms,"pause_state":paused["run"]["state"],
            "source_session":source.session,"next_session":next,"source_generation":generation,
            "same_pid":host_pid,"same_kernel_start":host_stamp,"same_generation":true,"fresh_nonfork_b":true,
            "admitted_enter_op":entered["op"],"actual_b_turn_started":true,"legacy_read_held":held,"legacy_retained_through_pause":true,
            "owned_hello_receipt":started,"worker_pid":worker_pid,"worker_start":worker_stamp,
            "public_running_before_b_turn":started["public_running_after_validated_progress"],
            "source_unchanged":true,"actual_worker_reap":true,"resumed_to_completed":true,"ordinary_eof_exit_after_b":true,
        }),
    );
}
