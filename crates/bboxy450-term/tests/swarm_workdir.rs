#![allow(
    clippy::disallowed_methods,
    reason = "private unattended app/PTY fixture"
)]
use serde_json::Value;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};
const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");

struct Fixture {
    directory: tempfile::TempDir,
    root: PathBuf,
    app: Option<Child>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        for name in [
            "home",
            "bin",
            "repository",
            "project with spaces",
            "override",
            "other swarm",
        ] {
            fs::create_dir(root.join(name)).unwrap();
        }
        fs::write(
            root.join("home/.zprofile"),
            format!(
                "export PATH='{}:/usr/bin:/bin'\n",
                root.join("bin").display()
            ),
        )
        .unwrap();
        fs::write(root.join("home/.zshrc"), "PROMPT='%# '\n").unwrap();
        fs::write(
            root.join("bin/claude"),
            "#!/bin/sh\nprintf '\\nCWD:%s\\n' \"$PWD\"\nexec /bin/cat\n",
        )
        .unwrap();
        fs::set_permissions(root.join("bin/claude"), fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory,
            root,
            app: None,
        }
    }
    fn command(&self) -> Command {
        let mut command = Command::new(BIN);
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("ZDOTDIR", self.root.join("home"))
            .env("CODEX_HOME", self.root.join("home/.codex"))
            .env("CLAUDE_CONFIG_DIR", self.root.join("home/.claude"))
            .env("PATH", "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin")
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }
    fn ctl(&self, args: &[&str]) -> Value {
        let output = self
            .command()
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(self.root.join("run"))
            .output()
            .unwrap();
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
    fn start(&mut self) {
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
                "{}",
                self.log()
            );
            if self.root.join("run/gui.endpoint").exists() {
                let output = self
                    .command()
                    .args(["ctl", "app", "status", "--run-dir"])
                    .arg(self.root.join("run"))
                    .output()
                    .unwrap();
                if output.status.success() {
                    let status: Value = serde_json::from_slice(&output.stdout).unwrap();
                    if status["data"]["gui_ready"] == true
                        && status["data"]["backend_ready"] == true
                    {
                        break;
                    }
                }
            }
            assert!(Instant::now() < until, "{}", self.log());
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    fn spawn(&self, kind: &str, swarm: &str, folder: Option<&Path>) -> String {
        let mut args = vec!["seat", "spawn", kind, "--place", "tab", "--swarm", swarm];
        let path = folder.map(|path| path.to_str().unwrap());
        if let Some(path) = path {
            args.extend(["--cwd", path]);
        }
        self.ctl(&args)["seat"].as_str().unwrap().into()
    }
    fn expect_cwd(&self, seat: &str, folder: &Path) {
        let text = format!("(?m)^CWD:{}$", regex::escape(folder.to_str().unwrap()));
        self.ctl(&["seat", "wait", seat, "--text", &text, "--timeout", "10000"]);
    }
    fn owners(&self) -> Vec<u64> {
        ["supervisor", "logic"]
            .map(|name| {
                let value: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).unwrap(),
                )
                .unwrap();
                value["process"]["pid"].as_u64().unwrap()
            })
            .into()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _keep_alive = &self.directory;
        let owners: Vec<u32> = ["supervisor", "logic"]
            .into_iter()
            .filter_map(|name| {
                let value: Value = serde_json::from_slice(
                    &fs::read(self.root.join(format!("run/{name}.owner.json"))).ok()?,
                )
                .ok()?;
                value["process"]["pid"].as_u64().map(|pid| pid as u32)
            })
            .collect();
        let _ = self
            .command()
            .args(["ctl", "app", "stop", "--run-dir"])
            .arg(self.root.join("run"))
            .output();
        if let Some(mut child) = self.app.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        for pid in owners {
            // Only backend owners recorded in this fixture's private run directory.
            unsafe {
                libc::kill(pid as i32, libc::SIGTERM);
            }
        }
    }
}

#[test]
#[ignore = "night: real app/PTY/window-journal boundary; make test-swarm runs it"]
fn swarm_workdir_is_inherited_overridden_and_retained_without_restarting_seats() {
    let mut fixture = Fixture::new();
    fixture.start();
    // An empty swarm exposes the folder control before its first harness.
    let layout = fixture.ctl(&["ui", "layout"]);
    let encoded = serde_json::to_string(&layout).unwrap();
    let swarm = regex::Regex::new(r"swarm-folder/([0-9A-Z]{26})/choose")
        .unwrap()
        .captures(&encoded)
        .expect("empty swarm has a real folder button")[1]
        .to_owned();
    let project = fixture.root.join("project with spaces");
    let set = fixture.ctl(&["ui", "swarm-workdir", &swarm, project.to_str().unwrap()]);
    assert_eq!(set["swarm_workdirs"][&swarm], project.to_str().unwrap());
    let first = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&first, &project);
    let before = fixture.ctl(&["seat", "ls"]);
    let old = before
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == first)
        .unwrap()
        .clone();
    let other = fixture.root.join("other swarm");
    fixture.ctl(&["ui", "swarm-workdir", &swarm, other.to_str().unwrap()]);
    let second = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&second, &other);
    let override_path = fixture.root.join("override");
    let explicit = fixture.spawn("claude", &swarm, Some(&override_path));
    fixture.expect_cwd(&explicit, &override_path);
    let after = fixture.ctl(&["seat", "ls"]);
    let retained = after
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["record"]["id"] == first)
        .unwrap();
    assert_eq!(old["state"]["process"], retained["state"]["process"]);
    assert_eq!(
        old["record"]["generation"],
        retained["record"]["generation"]
    );
    let new_space = fixture.ctl(&[
        "seat",
        "spawn",
        "claude",
        "--place",
        "space",
        "--cwd",
        override_path.to_str().unwrap(),
    ]);
    let other_swarm = new_space["swarm"].as_str().unwrap();
    assert_ne!(other_swarm, swarm);
    fixture.ctl(&[
        "ui",
        "swarm-workdir",
        other_swarm,
        override_path.to_str().unwrap(),
    ]);
    let isolated = fixture.spawn("claude", other_swarm, None);
    fixture.expect_cwd(&isolated, &override_path);
    let still_original = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&still_original, &other);
    // Wait for the actual durable window-state write before an abrupt UI loss.
    std::thread::sleep(Duration::from_millis(500));
    let owners = fixture.owners();
    let mut ui = fixture.app.take().unwrap();
    ui.kill().unwrap();
    ui.wait().unwrap();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    let restored = fixture.ctl(&["ui", "layout"]);
    assert_eq!(restored["swarm_workdirs"][&swarm], other.to_str().unwrap());
    assert_eq!(
        restored["swarm_workdirs"][other_swarm],
        override_path.to_str().unwrap()
    );
    if let Some(directory) = std::env::var_os("BBOXY_SWARM_EVIDENCE") {
        fs::create_dir_all(&directory).unwrap();
        let capture = PathBuf::from(directory).join("swarm-workdir.png");
        fixture.ctl(&["seat", "capture", "window", capture.to_str().unwrap()]);
    }
    let third = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&third, &other);
    fixture.ctl(&["ui", "swarm-workdir", &swarm, "default"]);
    let default = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&default, &fixture.root.join("repository"));
    fixture.ctl(&["app", "stop"]);
    assert!(fixture.app.as_mut().unwrap().wait().unwrap().success());
}
