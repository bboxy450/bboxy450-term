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
                "export PATH='{}:/usr/bin:/bin'\nexport LAUNCH_VALUE=from-profile\n",
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
        if std::thread::panicking() {
            let old = std::mem::replace(&mut self.directory, tempfile::tempdir().unwrap());
            eprintln!("failed private fixture retained: {}", old.keep().display());
        }
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
            if std::thread::panicking() {
                eprintln!("private UI exit before cleanup: {:?}", child.try_wait());
            }
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

#[test]
#[ignore = "night: private ctl-driven launcher/PTY/restart boundary; make test-spawn runs it"]
fn spawn_parameters_ctl_flow_runs_the_architect_and_remembers_at_each_home() {
    let mut fixture = Fixture::new();
    let script = r#"#!/bin/sh
printf '%s\000' "$0" "$@" > "$HOME/$BBOXY_SEAT_ID.argv"
printf '%s\000' "$PWD" "${LAUNCH_VALUE-}" > "$HOME/$BBOXY_SEAT_ID.context"
printf '\nCWD:%s\n' "$PWD"
exec /bin/cat
"#;
    for name in ["claude", "custom-claude"] {
        fs::write(fixture.root.join("bin").join(name), script).unwrap();
        fs::set_permissions(
            fixture.root.join("bin").join(name),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    }
    fixture.start();
    let initial = fixture.ctl(&["ui", "layout"]);
    let encoded = serde_json::to_string(&initial).unwrap();
    let swarm = regex::Regex::new(r"swarm-folder/([0-9A-Z]{26})/choose")
        .unwrap()
        .captures(&encoded)
        .unwrap()[1]
        .to_owned();
    let folder = fixture.root.join("project with spaces");
    fixture.ctl(&["ui", "swarm-workdir", &swarm, folder.to_str().unwrap()]);
    let first = fixture.spawn("zsh", &swarm, None);
    fixture.ctl(&["ui", "focus", &first]);
    fixture.ctl(&["ui", "key", "cmd-d"]);
    fixture.ctl(&["ui", "key", "c", "l", "a", "u", "d", "e"]);
    let level = fixture.ctl(&["ui", "key", "right"]);
    assert_eq!(level["launcher_params"]["fields"][0]["value"], "oracle");
    assert_eq!(
        level["launcher_params"]["fields"][1]["value"],
        folder.to_str().unwrap()
    );
    let edited = fixture.ctl(&["ui", "key", "right", "down", "enter"]);
    assert_eq!(edited["launcher_params"]["fields"][0]["value"], "architect");
    let shown = edited["launcher_params"]["runs"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(rendered_runs(&edited).as_deref(), Some(shown.as_str()));
    fixture.ctl(&["ui", "key", "enter"]);
    let architect = wait_harness(&fixture, "claude", 1);
    fixture.ctl(&[
        "seat",
        "wait",
        &architect,
        "--text",
        "CWD:",
        "--timeout",
        "10000",
    ]);
    assert_eq!(
        read_nul(fixture.root.join(format!("home/{architect}.context")))[0],
        folder.to_str().unwrap()
    );
    let seats = fixture.ctl(&["seat", "ls"]);
    let seat = seats
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == architect)
        .unwrap();
    assert_eq!(seat["record"]["role"]["name"], "architect");
    assert_eq!(seat["record"]["swarm"], swarm);
    let argv = read_nul(fixture.root.join(format!("home/{architect}.argv")));
    assert!(
        argv.iter().any(|v| v.contains("ROLE: ARCHITECT")),
        "{argv:?}"
    );
    // The executable and user argv shown by Runs are the exact prefix supplied
    // to the harness adapter; runtime MCP/hook arguments follow that prefix.
    let command = Path::new(&argv[0]).file_name().unwrap().to_str().unwrap();
    let actual = bboxy450_term::spawn_params::Resolved {
        role: Some("architect".into()),
        cwd: folder.to_string_lossy().into(),
        command: command.into(),
        args: vec![],
        env: vec![],
    };
    assert_eq!(shown, actual.runs());
    // Same parameter level edits command/arguments/env inline, Directory as a
    // typed value list. Enter accepts an edit; the next Enter launches.
    fixture.ctl(&[
        "ui",
        "key",
        "cmd-shift-d",
        "c",
        "l",
        "a",
        "u",
        "d",
        "e",
        "right",
    ]);
    set_parameter(&fixture, 2, "custom-claude");
    set_parameter(&fixture, 3, "--literal 'two words' '$HOME' ''");
    set_parameter(&fixture, 4, "LAUNCH_VALUE='two words'");
    fixture.ctl(&["ui", "key", "up", "up", "up", "right"]); // Environment -> Directory
    type_keys(&fixture, fixture.root.join("override").to_str().unwrap());
    fixture.ctl(&["ui", "key", "enter"]);
    let custom = fixture.ctl(&["ui", "layout"]);
    let runs = custom["launcher_params"]["runs"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(rendered_runs(&custom).as_deref(), Some(runs.as_str()));
    if let Some(directory) = std::env::var_os("BBOXY_SWARM_EVIDENCE") {
        fs::create_dir_all(&directory).unwrap();
        let capture = PathBuf::from(directory).join("spawn-parameters.png");
        fixture.ctl(&["seat", "capture", "window", capture.to_str().unwrap()]);
    }
    fixture.ctl(&["ui", "key", "enter"]);
    let second = wait_harness(&fixture, "claude", 2);
    fixture.ctl(&[
        "seat",
        "wait",
        &second,
        "--text",
        "CWD:",
        "--timeout",
        "10000",
    ]);
    let argv = read_nul(fixture.root.join(format!("home/{second}.argv")));
    assert_eq!(Path::new(&argv[0]).file_name().unwrap(), "custom-claude");
    assert_eq!(&argv[1..5], ["--literal", "two words", "$HOME", ""]);
    let context = read_nul(fixture.root.join(format!("home/{second}.context")));
    assert_eq!(
        context,
        [
            fixture.root.join("override").to_string_lossy().to_string(),
            "two words".into()
        ]
    );
    let actual = bboxy450_term::spawn_params::Resolved {
        role: None,
        cwd: context[0].clone(),
        command: "custom-claude".into(),
        args: argv[1..5].to_vec(),
        env: vec![("LAUNCH_VALUE".into(), context[1].clone())],
    };
    assert_eq!(runs, actual.runs());
    // A successful launch, and only that launch, updates the shared homes.
    let until = Instant::now() + Duration::from_secs(10);
    loop {
        let text = fs::read_to_string(fixture.root.join("account/config.toml")).unwrap_or_default();
        let layout = fixture.ctl(&["ui", "layout"]);
        if text.contains("custom-claude")
            && layout["swarm_workdirs"][&swarm] == fixture.root.join("override").to_str().unwrap()
        {
            break;
        }
        assert!(Instant::now() < until, "choices were not persisted");
        std::thread::sleep(Duration::from_millis(20));
    }
    // The ctl layout reply fences the latest durable window state. Kill only
    // this fixture's UI; the exact backend owners survive the abrupt loss.
    let owners = fixture.owners();
    let mut old = fixture.app.take().unwrap();
    old.kill().unwrap();
    old.wait().unwrap();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    fixture.ctl(&["ui", "focus", &second]);
    let next = fixture.ctl(&["ui", "key", "cmd-d", "c", "l", "a", "u", "d", "e", "right"]);
    let fields = next["launcher_params"]["fields"].as_array().unwrap();
    assert_eq!(
        fields[0]["value"], "oracle",
        "Role is a setting, not the last launch"
    );
    assert_eq!(
        fields[1]["value"],
        fixture.root.join("override").to_str().unwrap()
    );
    assert_eq!(fields[2]["value"], "custom-claude");
    assert_eq!(fields[3]["value"], "--literal 'two words' '$HOME' ''");
    assert_eq!(fields[4]["value"], "'LAUNCH_VALUE=two words'");
    assert!(fields[1..].iter().all(|f| f["source"] == "last choice"));
    fixture.ctl(&["ui", "key", "esc", "esc"]);
    // The common Enter path uses these same defaults without opening the form.
    fixture.ctl(&["ui", "key", "cmd-t", "c", "l", "a", "u", "d", "e", "enter"]);
    let third = wait_harness(&fixture, "claude", 3);
    fixture.ctl(&[
        "seat",
        "wait",
        &third,
        "--text",
        "CWD:",
        "--timeout",
        "10000",
    ]);
    assert_eq!(
        read_nul(fixture.root.join(format!("home/{third}.context"))),
        context
    );
    let argv = read_nul(fixture.root.join(format!("home/{third}.argv")));
    assert_eq!(&argv[1..5], ["--literal", "two words", "$HOME", ""]);
    assert!(argv.iter().any(|v| v.contains("ROLE: ORACLE")));
}
fn read_nul(path: PathBuf) -> Vec<String> {
    let mut fields: Vec<_> = fs::read(path)
        .unwrap()
        .split(|b| *b == 0)
        .map(|b| String::from_utf8(b.to_vec()).unwrap())
        .collect();
    assert_eq!(fields.pop().as_deref(), Some(""));
    fields
}
fn wait_harness(fixture: &Fixture, kind: &str, count: usize) -> String {
    let until = Instant::now() + Duration::from_secs(15);
    loop {
        let seats = fixture.ctl(&["seat", "ls"]);
        let rows: Vec<_> = seats
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["record"]["harness_kind"] == kind)
            .collect();
        if rows.len() == count {
            return rows.last().unwrap()["record"]["id"]
                .as_str()
                .unwrap()
                .into();
        }
        assert!(
            Instant::now() < until,
            "spawn missing: {seats}\n{}",
            fixture.log()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn type_keys(fixture: &Fixture, text: &str) {
    // Bounded ctl chrome keys, never PTY input or OS key injection.
    let chars: Vec<_> = text.chars().map(|c| c.to_string()).collect();
    for chunk in chars.chunks(32) {
        let mut args = vec!["ui", "key"];
        args.extend(chunk.iter().map(String::as_str));
        fixture.ctl(&args);
    }
}
fn set_parameter(fixture: &Fixture, index: usize, text: &str) {
    // Return to a known row using a unique typed label, then edit its selected value.
    fixture.ctl(&["ui", "key", "cmd-a"]);
    let label = ["Role", "Directory", "Command", "Arguments", "Environment"][index];
    type_keys(fixture, label);
    fixture.ctl(&["ui", "key", "right"]);
    type_keys(fixture, text);
    fixture.ctl(&["ui", "key", "enter"]);
}

fn rendered_runs(layout: &Value) -> Option<String> {
    fn text(node: &Value) -> String {
        node["text"].as_str().map(str::to_owned).unwrap_or_else(|| {
            node["children"]
                .as_array()
                .into_iter()
                .flatten()
                .map(text)
                .collect()
        })
    }
    fn find(node: &Value) -> Option<String> {
        if node["kind"]
            .as_str()
            .is_some_and(|kind| kind.ends_with("picker/div/cmdrow/cmdline"))
        {
            return Some(text(node));
        }
        node["children"].as_array()?.iter().find_map(find)
    }
    layout["tree"].as_array()?.iter().find_map(find)
}

fn click_region(fixture: &Fixture, key: &str) -> Value {
    let layout = fixture.ctl(&["ui", "layout"]);
    let r = layout["regions"][key]
        .as_array()
        .unwrap_or_else(|| panic!("missing {key}: {layout}"));
    let x = (r[0].as_f64().unwrap() + r[2].as_f64().unwrap() / 2.)
        .round()
        .to_string();
    let y = (r[1].as_f64().unwrap() + r[3].as_f64().unwrap() / 2.)
        .round()
        .to_string();
    fixture.ctl(&["ui", "click", &x, &y])
}

#[test]
#[ignore = "private ctl pointer/keyboard/PTY fixture; make test-navigation"]
fn native_navigation_clicks_splits_goto_and_directory_validation() {
    let mut fixture = Fixture::new();
    fixture.start();
    let initial = fixture.ctl(&["ui", "layout"]);
    let swarm = initial["regions"]
        .as_object()
        .unwrap()
        .keys()
        .find_map(|key| key.strip_prefix("swarm:"))
        .unwrap()
        .to_string();
    let first = fixture.spawn("claude", &swarm, None);
    let second = fixture.spawn("zsh", &swarm, None);
    assert_eq!(
        click_region(&fixture, &format!("seat:{first}"))["focused"],
        first
    );
    assert_eq!(
        click_region(&fixture, &format!("seat:{second}"))["focused"],
        second
    );
    fixture.ctl(&["ui", "key", "cmd-d"]);
    capture_navigation(&fixture, "split-right");
    type_keys(&fixture, "claude");
    fixture.ctl(&["ui", "key", "enter"]);
    let right = wait_harness(&fixture, "claude", 2);
    let layout = fixture.ctl(&["ui", "layout"]);
    assert_eq!(layout["panes"].as_array().unwrap().len(), 2, "{layout}");
    let panes = layout["panes"].as_array().unwrap();
    assert_eq!(panes[0]["rect"][1], panes[1]["rect"][1]);
    fixture.ctl(&["ui", "key", "cmd-shift-d"]);
    capture_navigation(&fixture, "split-down");
    type_keys(&fixture, "claude");
    fixture.ctl(&["ui", "key", "enter"]);
    let below = wait_harness(&fixture, "claude", 3);
    let layout = fixture.ctl(&["ui", "layout"]);
    assert_eq!(layout["panes"].as_array().unwrap().len(), 3, "{layout}");
    let split_tab = layout["tab"].as_str().unwrap().to_owned();
    assert_eq!(
        click_region(&fixture, &format!("pane:{below}"))["focused"],
        below
    );
    assert_eq!(
        click_region(&fixture, &format!("mini:{second}"))["focused"],
        second
    );
    assert_eq!(
        click_region(&fixture, &format!("seat:{first}"))["focused"],
        first
    );
    assert_eq!(
        click_region(&fixture, &format!("tab:{split_tab}"))["tab"],
        split_tab
    );
    let seats = fixture.ctl(&["seat", "ls"]);
    let name = seats
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["record"]["id"] == right)
        .unwrap()["record"]["name"]
        .as_str()
        .unwrap();
    fixture.ctl(&["ui", "key", "cmd-t"]);
    capture_navigation(&fixture, "new-tab");
    let bottom = fixture.ctl(&[
        "ui", "key", "down", "down", "down", "down", "down", "down", "down", "down", "down",
    ]);
    assert!(
        bottom["regions"]["launcher:item:settings"][3]
            .as_f64()
            .unwrap()
            >= 40.,
        "selected row must be fully visible"
    );
    fixture.ctl(&["ui", "launcher-scroll", "-100"]);
    assert_eq!(
        click_region(&fixture, "launcher:item:settings")["page"],
        "settings"
    );
    fixture.ctl(&["ui", "key", "cmd-t"]);
    type_keys(&fixture, &format!("goto {name}"));
    let jumped = fixture.ctl(&["ui", "key", "enter"]);
    assert_eq!(jumped["focused"], right, "{jumped}");
    assert_eq!(jumped["launcher_open"], false);
    fixture.ctl(&["ui", "key", "cmd-d"]);
    type_keys(&fixture, "claude");
    fixture.ctl(&["ui", "key", "right", "down", "right"]);
    let invalid = fixture.root.join("missing");
    type_keys(&fixture, invalid.to_str().unwrap());
    fixture.ctl(&["ui", "key", "enter"]);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let level = fixture.ctl(&["ui", "layout"]);
        if level["launcher_params"]["runs"]
            .as_str()
            .unwrap()
            .starts_with("Directory does not exist")
        {
            break;
        }
        assert!(Instant::now() < until, "{level}");
        std::thread::sleep(Duration::from_millis(10));
    }
    fixture.ctl(&["ui", "key", "cmd-a"]);
    type_keys(&fixture, fixture.root.join("override").to_str().unwrap());
    fixture.ctl(&["ui", "key", "enter"]);
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let level = fixture.ctl(&["ui", "layout"]);
        if level["launcher_params"]["choosing"].is_null() {
            assert_eq!(
                level["launcher_params"]["fields"][1]["value"],
                fixture.root.join("override").to_str().unwrap()
            );
            break;
        }
        assert!(Instant::now() < until, "{level}");
        std::thread::sleep(Duration::from_millis(10));
    }
    let closed = fixture.ctl(&["ui", "key", "esc", "esc"]);
    assert_eq!(closed["launcher_open"], false);
    let empty = click_region(&fixture, "new-swarm");
    let empty_id = empty["swarm"].as_str().unwrap().to_owned();
    assert_ne!(
        empty_id, swarm,
        "page={} focused={} launcher={}",
        empty["page"], empty["focused"], empty["launcher_open"]
    );
    assert!(empty["focused"].is_null(), "{empty}");
    assert_eq!(
        click_region(&fixture, &format!("new-seat:{empty_id}"))["launcher_open"],
        true
    );
    type_keys(&fixture, "claude");
    let params = fixture.ctl(&["ui", "key", "right"]);
    assert_eq!(params["launcher_params"]["fields"][0]["label"], "Role");
    assert_eq!(
        click_region(&fixture, "launcher:item:param:0")["launcher_params"]["choosing"],
        "Role"
    );
    assert_eq!(
        click_region(&fixture, "launcher:item:value:1")["launcher_params"]["fields"][0]["value"],
        "architect"
    );
    fixture.ctl(&["ui", "key", "esc", "esc"]);
    let owners = fixture.owners();
    let mut old = fixture.app.take().unwrap();
    old.kill().unwrap();
    old.wait().unwrap();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    assert_eq!(fixture.ctl(&["ui", "layout"])["swarm"], empty_id);
    fixture.ctl(&["ui", "key", "cmd-t"]);
    type_keys(&fixture, &format!("goto swarm:{swarm}"));
    assert_eq!(fixture.ctl(&["ui", "key", "enter"])["swarm"], swarm);
    let settings = click_region(&fixture, "settings");
    assert_eq!(settings["page"], "settings");
    fixture.ctl(&["app", "stop"]);
    assert!(fixture.app.as_mut().unwrap().wait().unwrap().success());
}

fn capture_navigation(fixture: &Fixture, name: &str) {
    if let Some(directory) = std::env::var_os("BBOXY_SWARM_EVIDENCE") {
        fs::create_dir_all(&directory).unwrap();
        let path = PathBuf::from(directory).join(format!("{name}.png"));
        fixture.ctl(&["seat", "capture", "window", path.to_str().unwrap()]);
        fs::write(
            path.with_extension("json"),
            serde_json::to_vec_pretty(&fixture.ctl(&["ui", "layout"])).unwrap(),
        )
        .unwrap();
    }
}
