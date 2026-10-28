//! Origin: bboxy450-term, MIT OR Apache-2.0. Retain credit on reuse:
//! https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
#[ignore = "night: private ctl sidebar-drag/PTY/restart fixture; make test-sidebar-resize"]
fn sidebar_drag_reflows_and_persists_without_restarting_harnesses() {
    fn rect(node: &Value) -> [f64; 4] {
        let rect = node["rect"].as_array().unwrap();
        std::array::from_fn(|i| rect[i].as_f64().unwrap())
    }
    fn named<'a>(node: &'a Value, key: &str) -> Option<&'a Value> {
        if node["kind"].as_str() == Some(key) {
            return Some(node);
        }
        node["children"]
            .as_array()?
            .iter()
            .find_map(|child| named(child, key))
    }
    fn named_rect(frame: &Value, key: &str) -> [f64; 4] {
        rect(
            frame["tree"]
                .as_array()
                .unwrap()
                .iter()
                .find_map(|node| named(node, key))
                .unwrap_or_else(|| panic!("missing solved node {key}")),
        )
    }
    fn handle(frame: &Value) -> [f64; 4] {
        let region = frame["regions"]["sidebar-resize"]
            .as_array()
            .expect("sidebar exposes its retained resize handle");
        let region = std::array::from_fn(|i| region[i].as_f64().unwrap());
        assert_eq!(region[2], 8.);
        region
    }
    fn edge(frame: &Value) -> f64 {
        let h = handle(frame);
        h[0] + h[2] / 2.
    }
    fn drag(fixture: &Fixture, width: f64) -> Value {
        let frame = fixture.ctl(&["ui", "layout"]);
        let h = handle(&frame);
        // Grip two pixels left of the edge. The result must not jump by two.
        let from = h[0] + 2.;
        let to = width + from - edge(&frame);
        let y = (h[1] + h[3] / 2.).round().to_string();
        fixture.ctl(&[
            "ui",
            "drag",
            &from.round().to_string(),
            &y,
            &to.round().to_string(),
            &y,
        ])
    }
    fn process(fixture: &Fixture, seat: &str) -> (Value, Value) {
        let rows = fixture.ctl(&["seat", "ls"]);
        let row = rows
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["record"]["id"] == seat)
            .unwrap();
        (
            row["state"]["process"].clone(),
            row["record"]["generation"].clone(),
        )
    }
    fn aligned(frame: &Value, seat: &str, inset: f64) {
        let width = edge(frame);
        let dock = named_rect(frame, "named:conversation");
        assert_eq!(dock[0], 0.);
        assert_eq!(dock[2], width, "dock follows the sidebar edge");
        for key in [
            "named:composer/send",
            "named:composer/priority",
            "named:composer/kpick",
        ] {
            let control = named_rect(frame, key);
            assert!(
                control[0] >= 0. && control[0] + control[2] <= width,
                "{key} must fit inside the sidebar"
            );
        }
        let h = handle(frame);
        assert!(h[1] <= dock[1] && h[1] + h[3] >= dock[1] + dock[3]);
        let pane = frame["panes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|pane| pane["seat"] == seat)
            .unwrap();
        let pane = rect(pane);
        assert_eq!(pane[0], width + inset, "stage follows the same edge");
        assert!(pane[2] > 0. && pane[0] + pane[2] <= frame["size"][0].as_f64().unwrap());
        assert_eq!(frame["focused"], seat, "drag does not move focus");
    }
    fn capture(fixture: &Fixture, name: &str) {
        if let Some(directory) = std::env::var_os("BBOXY_SIDEBAR_EVIDENCE") {
            fs::create_dir_all(&directory).unwrap();
            let path = PathBuf::from(directory).join(name);
            fixture.ctl(&["seat", "capture", "window", path.to_str().unwrap()]);
        }
    }

    let mut fixture = Fixture::new();
    fixture.start();
    let initial = fixture.ctl(&["ui", "layout"]);
    let swarm = initial["swarm"].as_str().unwrap().to_owned();
    let seat = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&seat, &fixture.root.join("repository"));
    let initial = fixture.ctl(&["ui", "focus", &seat]);
    let before = process(&fixture, &seat);
    let pane = initial["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pane| pane["seat"] == seat)
        .unwrap();
    let inset = rect(pane)[0] - edge(&initial);
    for (requested, expected, name) in [
        (420., 420., "sidebar-420.png"),
        (20., 160., "sidebar-160.png"),
        (900., 520., "sidebar-520.png"),
    ] {
        let frame = drag(&fixture, requested);
        assert_eq!(edge(&frame), expected);
        aligned(&frame, &seat, inset);
        capture(&fixture, name);
    }

    // Unsupported drags must not fall through to the ordinary settings click.
    let before_refusal = fixture.ctl(&["ui", "layout"]);
    let gear = before_refusal["regions"]["settings"].as_array().unwrap();
    let x = (gear[0].as_f64().unwrap() + gear[2].as_f64().unwrap() / 2.)
        .round()
        .to_string();
    let y = (gear[1].as_f64().unwrap() + gear[3].as_f64().unwrap() / 2.)
        .round()
        .to_string();
    let refused = fixture
        .command()
        .args(["ctl", "ui", "drag", &x, &y, "700", &y, "--run-dir"])
        .arg(fixture.root.join("run"))
        .output()
        .unwrap();
    assert!(!refused.status.success(), "off-handle drag must be refused");
    let reply: Value = serde_json::from_slice(&refused.stdout).unwrap();
    assert_eq!(reply["ok"], false);
    let unchanged = fixture.ctl(&["ui", "layout"]);
    assert_eq!(edge(&unchanged), 520.);
    assert_eq!(unchanged["page"], before_refusal["page"]);
    assert_eq!(unchanged["focused"], before_refusal["focused"]);

    let settings = click_region(&fixture, "settings");
    assert_eq!(settings["page"], "settings");
    let page = settings["tree"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| named(node, "named:local/close").is_some())
        .unwrap();
    assert_eq!(rect(page)[0], 520.);
    assert_eq!(rect(page)[2], settings["size"][0].as_f64().unwrap() - 520.);
    assert_eq!(named_rect(&settings, "named:conversation")[2], 520.);
    capture(&fixture, "sidebar-settings-520.png");
    click_region(&fixture, &format!("seat:{seat}"));
    let narrow = fixture.ctl(&["app", "resize", "2x24"]);
    assert_eq!(narrow["size"][0], 320.);
    assert_eq!(edge(&narrow), 160.);
    aligned(&narrow, &seat, inset);
    let resized = fixture.ctl(&["app", "resize", "120x40"]);
    assert_eq!(edge(&resized), 520.);
    aligned(&resized, &seat, inset);
    fixture.ctl(&["app", "resize", "180x60"]);
    let final_width = drag(&fixture, 420.);
    assert_eq!(edge(&final_width), 420.);
    aligned(&final_width, &seat, inset);
    assert_eq!(process(&fixture, &seat), before);
    std::thread::sleep(Duration::from_millis(500));
    let owners = fixture.owners();
    let mut old = fixture.app.take().unwrap();
    old.kill().unwrap();
    old.wait().unwrap();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    assert_eq!(process(&fixture, &seat), before);
    let restored = fixture.ctl(&["ui", "layout"]);
    assert_eq!(edge(&restored), 420., "the chosen width survives UI loss");
    aligned(&restored, &seat, inset);
    capture(&fixture, "sidebar-restored.png");
    fixture.ctl(&["app", "stop"]);
    assert!(fixture.app.as_mut().unwrap().wait().unwrap().success());
}

#[test]
#[ignore = "night: private ctl new-swarm/cwd/PTY/restart fixture; make test-new-swarm"]
fn new_swarm_cwd_controls_stay_visible_and_launch_in_the_saved_folder() {
    let mut fixture = Fixture::new();
    fixture.start();
    let initial = fixture.ctl(&["ui", "layout"]);
    let original = initial["swarm"].as_str().unwrap();
    let original_folder = fixture.root.join("override");
    fixture.ctl(&[
        "ui",
        "swarm-workdir",
        original,
        original_folder.to_str().unwrap(),
    ]);
    let mut swarm = String::new();
    for _ in 0..8 {
        fixture.ctl(&["ui", "sidebar-scroll", "-1000"]);
        let created = click_region(&fixture, "new-swarm");
        let next = created["swarm"].as_str().unwrap().to_owned();
        assert_ne!(next, swarm, "each click creates a new swarm");
        swarm = next;
        assert_ne!(swarm, original);
        assert!(created["focused"].is_null());
        let key = format!("swarm-folder/{swarm}/choose");
        assert!(
            created["regions"][&key].as_array().is_some(),
            "new swarm cwd must be visible"
        );
    }
    let folder = fixture.root.join("project with spaces");
    let selected = fixture.ctl(&["ui", "swarm-workdir", &swarm, folder.to_str().unwrap()]);
    assert_eq!(selected["swarm_workdirs"][&swarm], folder.to_str().unwrap());
    assert_eq!(
        selected["swarm_workdirs"][original],
        original_folder.to_str().unwrap()
    );
    let reset = click_region(&fixture, &format!("swarm-folder/{swarm}/default"));
    assert!(reset["swarm_workdirs"][&swarm].is_null());
    assert_eq!(
        reset["swarm_workdirs"][original],
        original_folder.to_str().unwrap()
    );
    fixture.ctl(&["ui", "swarm-workdir", &swarm, folder.to_str().unwrap()]);
    let seat = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&seat, &folder);
    std::thread::sleep(Duration::from_millis(500));
    let owners = fixture.owners();
    let mut old = fixture.app.take().unwrap();
    old.kill().unwrap();
    old.wait().unwrap();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    assert_eq!(
        fixture.ctl(&["ui", "layout"])["swarm_workdirs"][&swarm],
        folder.to_str().unwrap()
    );
    let next = fixture.spawn("claude", &swarm, None);
    fixture.expect_cwd(&next, &folder);
    fixture.ctl(&["app", "stop"]);
    assert!(fixture.app.as_mut().unwrap().wait().unwrap().success());
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

#[test]
#[ignore = "private ctl swarm switching, messaging readiness and colored PTY GPU regression"]
fn swarm_switches_wait_for_membership_and_dim_real_inactive_seats() {
    fn capture(f: &Fixture, name: &str) -> (u32, Vec<u8>) {
        let path = f.root.join(name);
        f.ctl(&["seat", "capture", "window", path.to_str().unwrap()]);
        if let Some(directory) = std::env::var_os("BBOXY_SWITCH_EVIDENCE") {
            fs::create_dir_all(&directory).unwrap();
            fs::copy(&path, PathBuf::from(directory).join(name)).unwrap();
        }
        let mut reader = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).unwrap()))
            .read_info()
            .unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        bytes.truncate(info.buffer_size());
        (info.width, bytes)
    }
    fn clean(f: &Fixture) {
        let frame = f.ctl(&["ui", "layout"]);
        let errors = f.ctl(&["debug", "errors"]);
        for forbidden in [
            "unknown GUI stream",
            "unknown operator messaging swarm",
            "swarm changed; choose a destination",
        ] {
            assert!(!frame.to_string().contains(forbidden), "{frame}");
            assert!(!errors.to_string().contains(forbidden), "{errors}");
            assert!(!f.log().contains(forbidden), "{}", f.log());
        }
    }
    let mut f = Fixture::new();
    fs::write(f.root.join("bin/claude"), "#!/bin/sh\nprintf '\\033[2J\\033[HCWD:%s\\n' \"$PWD\"\nprintf '\\033[48;2;255;0;0m                \\033[0m\\nCOLOR-READY\\n'\nexec /bin/cat\n").unwrap();
    f.start();
    let first_swarm = f.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let second_swarm = click_region(&f, "new-swarm")["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let third_swarm = click_region(&f, "new-swarm")["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    for swarm in [
        &first_swarm,
        &third_swarm,
        &second_swarm,
        &first_swarm,
        &second_swarm,
    ] {
        assert_eq!(click_region(&f, &format!("swarm:{swarm}"))["swarm"], *swarm);
        std::thread::sleep(Duration::from_millis(40));
        clean(&f);
    }
    let first = f.spawn("claude", &second_swarm, None);
    f.ctl(&[
        "seat",
        "wait",
        &first,
        "--text",
        "COLOR-READY",
        "--timeout",
        "10000",
    ]);
    f.ctl(&[
        "ui",
        "compose",
        "--to",
        &first,
        "--text",
        "SWARM-READS-RESUMED",
    ]);
    let until = Instant::now() + Duration::from_secs(10);
    while !f
        .ctl(&["ui", "layout"])
        .to_string()
        .contains("SWARM-READS-RESUMED")
    {
        assert!(
            Instant::now() < until,
            "message reads did not resume\n{}",
            f.log()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
    f.ctl(&["ui", "key", "cmd-d"]);
    type_keys(&f, "claude");
    f.ctl(&["ui", "key", "enter"]);
    let second = wait_harness(&f, "claude", 2);
    f.ctl(&[
        "seat",
        "wait",
        &second,
        "--text",
        "COLOR-READY",
        "--timeout",
        "10000",
    ]);
    f.ctl(&["ui", "clock", "freeze"]);
    let active = f.ctl(&["ui", "focus", &first]);
    assert_eq!(
        active["panes"].as_array().unwrap().len(),
        2,
        "both seats must stay visible"
    );
    f.ctl(&["ui", "clock", "advance", "900"]);
    let (width, vivid) = capture(&f, "swarm-seat-active.png");
    let pane = active["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == first)
        .unwrap();
    let rect: Vec<_> = pane["rect"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap() as usize)
        .collect();
    let mut red = vec![];
    for y in rect[1] + 2..rect[1] + rect[3] - 2 {
        for x in rect[0] + 2..rect[0] + rect[2] - 2 {
            let i = (y * width as usize + x) * 4;
            if vivid[i] > 240 && vivid[i + 1] < 15 && vivid[i + 2] < 15 {
                red.push(i);
            }
        }
    }
    assert!(
        red.len() > 100,
        "actual focused PTY has no red background cells"
    );
    let inactive = f.ctl(&["ui", "focus", &second]);
    let inactive_pane = inactive["panes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["seat"] == first)
        .unwrap();
    assert_eq!(
        pane["rect"], inactive_pane["rect"],
        "focus changes paint, never pane geometry"
    );
    f.ctl(&["ui", "clock", "advance", "900"]);
    let (_, faded) = capture(&f, "swarm-seat-inactive.png");
    for &i in &red {
        let rgb = &faded[i..i + 3];
        assert!(
            rgb.iter().max().unwrap() - rgb.iter().min().unwrap() < 32,
            "inactive PTY stayed colorful: {rgb:?}"
        );
    }
    f.ctl(&["ui", "focus", &first]);
    f.ctl(&["ui", "clock", "advance", "900"]);
    let (_, restored) = capture(&f, "swarm-seat-refocused.png");
    for i in red {
        assert_eq!(
            &vivid[i..i + 4],
            &restored[i..i + 4],
            "focus did not restore original cell pixels"
        );
    }
    for swarm in [&third_swarm, &first_swarm, &second_swarm] {
        click_region(&f, &format!("swarm:{swarm}"));
        clean(&f);
    }
    let owners = f.owners();
    let before = f.ctl(&["seat", "ls"]);
    let mut old = f.app.take().unwrap();
    old.kill().unwrap();
    old.wait().unwrap();
    f.start();
    assert_eq!(f.owners(), owners);
    let after = f.ctl(&["seat", "ls"]);
    for seat in [&first, &second] {
        let get = |rows: &Value| {
            rows.as_array()
                .unwrap()
                .iter()
                .find(|r| r["record"]["id"] == *seat)
                .map(|r| {
                    (
                        r["record"]["generation"].clone(),
                        r["state"]["process"].clone(),
                    )
                })
                .unwrap()
        };
        assert_eq!(get(&before), get(&after));
    }
    clean(&f);
    f.ctl(&["app", "stop"]);
    assert!(f.app.as_mut().unwrap().wait().unwrap().success());
}

// keeps: spec mcp-control-plane
#[test]
#[ignore = "private unattended seat-MCP/GUI/PTY boundary; make test-oracle-control"]
fn oracle_mcp_splits_focuses_and_reattaches_without_restarting_harnesses() {
    use bbt_mux::mcp::bridge::{Bridge, Config};
    use serde_json::json;
    let mut fixture = Fixture::new();
    // This fake harness receives exactly the normal per-seat launch admission.
    // It exports that admission only into this disposable fixture's HOME.
    fs::write(fixture.root.join("bin/claude"),
        "#!/bin/sh\numask 077\nenv > \"$HOME/$BBOXY_SEAT_ID.admission\"\nprintf '%s' \"$PWD\" > \"$HOME/$BBOXY_SEAT_ID.cwd\"\nprintf '\\nCWD:%s\\n' \"$PWD\"\nexec /bin/cat\n").unwrap();
    let expect_cwd = |fixture: &Fixture, seat: &str, expected: &Path| {
        fixture.ctl(&["seat", "wait", seat, "--text", "CWD:", "--timeout", "30000"]);
        assert_eq!(
            fs::read_to_string(fixture.root.join(format!("home/{seat}.cwd"))).unwrap(),
            expected.to_str().unwrap()
        );
    };
    fixture.start();
    let initial = fixture.ctl(&["ui", "layout"]);
    let swarm = initial["regions"]
        .as_object()
        .unwrap()
        .keys()
        .find_map(|k| k.strip_prefix("swarm:"))
        .unwrap()
        .to_owned();
    let oracle = fixture.spawn("claude", &swarm, None);
    expect_cwd(&fixture, &oracle, &fixture.root.join("repository"));
    let admission =
        fs::read_to_string(fixture.root.join(format!("home/{oracle}.admission"))).unwrap();
    let admission: std::collections::BTreeMap<_, _> = admission
        .lines()
        .filter_map(|l| l.split_once('='))
        .collect();
    let cap = admission["BBOXY_CAPABILITY"];
    let capability =
        std::array::from_fn(|i| u8::from_str_radix(&cap[i * 2..i * 2 + 2], 16).unwrap());
    let mut bridge = Bridge::new(Config {
        socket: admission["BBOXY_SOCK"].into(),
        seat: oracle.parse().unwrap(),
        generation: 1,
        capability,
    });
    let rpc = |bridge: &mut Bridge, method: &str, params: Value| {
        bridge
            .handle(
                &serde_json::to_vec(
                    &json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}),
                )
                .unwrap(),
            )
            .unwrap()
    };
    let initialized = rpc(
        &mut bridge,
        "initialize",
        json!({"protocolVersion":"2025-11-25","clientInfo":{"name":"oracle-fixture","version":"1"},"capabilities":{}}),
    );
    assert!(initialized.get("error").is_none(), "{initialized}");
    let call = |bridge: &mut Bridge, tool: &str, args: Value| {
        let reply = rpc(bridge, "tools/call", json!({"name":tool,"arguments":args}));
        assert!(reply.get("error").is_none(), "{reply}");
        assert_eq!(reply["result"]["isError"], false, "{reply}");
        reply["result"]["structuredContent"].clone()
    };
    let tab = fixture.ctl(&["ui", "layout"])["tab"]
        .as_str()
        .unwrap()
        .to_owned();
    let split = |side: &str| json!({"swarm_id":swarm,"approved_profile":"claude","cwd":fixture.root.join("project with spaces"),"seat_id":oracle,"expected_process_generation":"1","tab_id":tab,"side":side,"role":"architect"});
    let right = call(
        &mut bridge,
        "ctl",
        json!({"command":"seat_split","arguments":split("right")}),
    );
    let right = right["seat_id"].as_str().unwrap().to_owned();
    expect_cwd(&fixture, &right, &fixture.root.join("project with spaces"));
    let frame = call(
        &mut bridge,
        "seat_focus",
        json!({"seat_id":right,"expected_process_generation":"1"}),
    );
    assert!(frame["frame"]["frame_seq"].as_u64().unwrap() > 0);
    let layout = fixture.ctl(&["ui", "layout"]);
    assert_eq!(layout["focused"], right);
    let panes = layout["panes"].as_array().unwrap();
    assert_eq!(panes.len(), 2, "{layout}");
    assert_eq!(panes[0]["rect"][1], panes[1]["rect"][1]);
    let before = fixture.ctl(&["seat", "ls"]);
    let owners = fixture.owners();
    let mut ui = fixture.app.take().unwrap();
    ui.kill().unwrap();
    ui.wait().unwrap();
    let below = call(&mut bridge, "seat_split", split("bottom"));
    let below = below["seat_id"].as_str().unwrap().to_owned();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    expect_cwd(&fixture, &below, &fixture.root.join("project with spaces"));
    call(
        &mut bridge,
        "ctl",
        json!({"command":"seat_focus","arguments":{"seat_id":below,"expected_process_generation":"1"}}),
    );
    let layout = fixture.ctl(&["ui", "layout"]);
    assert_eq!(layout["tab"], tab);
    assert_eq!(layout["panes"].as_array().unwrap().len(), 3, "{layout}");
    let after = fixture.ctl(&["seat", "ls"]);
    for old in before.as_array().unwrap() {
        let current = after
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["record"]["id"] == old["record"]["id"])
            .unwrap();
        assert_eq!(current["record"]["generation"], old["record"]["generation"]);
        assert_eq!(current["state"]["process"], old["state"]["process"]);
    }
    let record = after
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == below)
        .unwrap();
    assert_eq!(record["record"]["role"]["name"], "architect");
    // Simulate an installed older control service using only this private
    // fixture's owner record. The next ordinary UI attach upgrades logic,
    // preserving the supervisor and every harness PID/generation.
    let old_owners = fixture.owners();
    let mut ui = fixture.app.take().unwrap();
    ui.kill().unwrap();
    ui.wait().unwrap();
    let owner_path = fixture.root.join("run/logic.owner.json");
    let mut owner: Value = serde_json::from_slice(&fs::read(&owner_path).unwrap()).unwrap();
    owner["ready"]
        .as_object_mut()
        .unwrap()
        .remove("mcp_control_version");
    fs::write(&owner_path, serde_json::to_vec(&owner).unwrap()).unwrap();
    fixture.start();
    let upgraded = fixture.owners();
    assert_eq!(upgraded[0], old_owners[0]);
    assert_ne!(upgraded[1], old_owners[1]);
    let upgraded_seats = fixture.ctl(&["seat", "ls"]);
    for old in after.as_array().unwrap() {
        let current = upgraded_seats
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["record"]["id"] == old["record"]["id"])
            .unwrap();
        assert_eq!(current["record"]["generation"], old["record"]["generation"]);
        assert_eq!(current["state"]["process"], old["state"]["process"]);
    }
    call(
        &mut bridge,
        "ctl",
        json!({"command":"layout_read","arguments":{}}),
    );
    // Matching versions do not prove health: a retained owner may have lost
    // its socket. One start must recover it, keeping all existing PTYs.
    let old_owners = fixture.owners();
    let mut ui = fixture.app.take().unwrap();
    ui.kill().unwrap();
    ui.wait().unwrap();
    let mut owner: Value = serde_json::from_slice(&fs::read(&owner_path).unwrap()).unwrap();
    owner["ready"]["root"] = json!(fixture.root.join("missing-logic-endpoint"));
    fs::write(&owner_path, serde_json::to_vec(&owner).unwrap()).unwrap();
    fixture.start();
    let recovered_owners = fixture.owners();
    assert_eq!(old_owners[0], recovered_owners[0]);
    assert_ne!(old_owners[1], recovered_owners[1]);
    let recovered = fixture.ctl(&["seat", "ls"]);
    for old in upgraded_seats.as_array().unwrap() {
        let current = recovered
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["record"]["id"] == old["record"]["id"])
            .unwrap();
        assert_eq!(current["record"]["generation"], old["record"]["generation"]);
        assert_eq!(current["state"]["process"], old["state"]["process"]);
    }
    call(
        &mut bridge,
        "ctl",
        json!({"command":"layout_read","arguments":{}}),
    );
    println!(
        "Oracle MCP: right split + completed focus frame; detached bottom split + reattach; control-service upgrade and missing-endpoint recovery retained all {} process identities",
        after.as_array().unwrap().len()
    );
}

// keeps: spec c1-live-seats; a desktop-size snapshot must reach the painted pane
#[test]
#[ignore = "private large-window GUI/PTY boundary; make test-large-gui-frame"]
fn large_terminal_snapshot_survives_resize_and_ui_reattach() {
    let mut fixture = Fixture::new();
    fs::write(fixture.root.join("bin/claude"),r#"#!/bin/sh
read -r line
/usr/bin/awk 'BEGIN { for (r=0;r<120;r++) { for (c=0;c<240;c++) printf "\033[38;2;%d;30;40mX",(c%2)*200; printf "\033[0m\r\n" } printf "\r\nLARGE-VIEW-READY\r\n" }'
exec /bin/cat
"#).unwrap();
    fixture.start();
    let initial = fixture.ctl(&["ui", "layout"]);
    let swarm = initial["regions"]
        .as_object()
        .unwrap()
        .keys()
        .find_map(|k| k.strip_prefix("swarm:"))
        .unwrap()
        .to_owned();
    let seat = fixture.spawn("claude", &swarm, None);
    fixture.ctl(&["app", "resize", "320x120"]);
    fixture.ctl(&["seat", "type", &seat, "render"]);
    fixture.ctl(&["seat", "key", &seat, "enter"]);
    fixture.ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "LARGE-VIEW-READY",
        "--timeout",
        "30000",
    ]);
    let screen = fixture.ctl(&["seat", "screen", &seat, "--cells"]);
    let bytes = serde_json::to_vec(&screen).unwrap().len();
    assert!(
        bytes > 1024 * 1024,
        "fixture must exercise a GUI reply above the previous 1 MiB cap: {bytes}"
    );
    let painted = |fixture: &Fixture, name: &str| {
        let path = fixture.root.join(name);
        fixture.ctl(&["seat", "capture", "window", path.to_str().unwrap()]);
        let decoder = png::Decoder::new(std::io::BufReader::new(fs::File::open(path).unwrap()));
        let mut reader = decoder.read_info().unwrap();
        let mut bytes = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut bytes).unwrap();
        assert_eq!(info.color_type, png::ColorType::Rgba);
        let red = bytes[..info.buffer_size()]
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                let x = (*i as u32) % info.width;
                let y = (*i as u32) / info.width;
                x > 400 && y > 120 && y + 100 < info.height && p[0] > 140 && p[1] < 80 && p[2] < 100
            })
            .count();
        assert!(
            red > 1000,
            "large snapshot did not paint its colored terminal glyphs: {red} red pixels"
        );
    };
    painted(&fixture, "large-before.png");
    let before = fixture.ctl(&["seat", "ls"]);
    let old = before
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == seat)
        .unwrap();
    let owners = fixture.owners();
    let mut ui = fixture.app.take().unwrap();
    ui.kill().unwrap();
    ui.wait().unwrap();
    fixture.start();
    assert_eq!(fixture.owners(), owners);
    fixture.ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "LARGE-VIEW-READY",
        "--timeout",
        "30000",
    ]);
    painted(&fixture, "large-after.png");
    let after = fixture.ctl(&["seat", "ls"]);
    let current = after
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["record"]["id"] == seat)
        .unwrap();
    assert_eq!(old["record"]["generation"], current["record"]["generation"]);
    assert_eq!(old["state"]["process"], current["state"]["process"]);
    println!(
        "Large terminal: {bytes} byte styled reply painted on a 320x120 terminal and after UI reattach; same PTY PID/generation"
    );
}
