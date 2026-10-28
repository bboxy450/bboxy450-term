// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
#![cfg(all(target_os = "macos", debug_assertions))]
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended desktop/UI/PTY process boundary"
)]

use serde_json::{Value, json};
use std::{
    cell::Cell,
    fs,
    io::{BufRead, BufReader, Write},
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");
const DEADLINE: Duration = Duration::from_secs(30);

fn wait_until(label: &str, timeout: Duration, mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + timeout;
    while !ready() {
        assert!(Instant::now() < deadline, "timed out waiting for {label}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn wait_child(child: &mut Child, timeout: Duration) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(_) => return None,
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn alive(identity: &Value) -> bool {
    let Some(pid) = identity["pid"].as_u64().and_then(|n| i32::try_from(n).ok()) else {
        return false;
    };
    // SAFETY: proc_pidinfo fills one bounded plain-C process record. Birth checks
    // prevent a reused PID from becoming cleanup authority.
    let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
    let size = std::mem::size_of_val(&info) as i32;
    let read = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            (&mut info as *mut libc::proc_bsdinfo).cast(),
            size,
        )
    };
    read == size
        && info.pbi_status != 5
        && identity["birth"][0].as_u64() == Some(info.pbi_start_tvsec)
        && identity["birth"][1].as_u64() == Some(info.pbi_start_tvusec)
}

fn stop_identity(identity: &Value) {
    if !alive(identity) {
        return;
    }
    let pid = identity["pid"]
        .as_u64()
        .and_then(|n| i32::try_from(n).ok())
        .unwrap();
    // SAFETY: the PID and birth match an owner recorded by this private fixture.
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    while alive(identity) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(25));
    }
    if alive(identity) {
        // SAFETY: revalidate the same private process birth immediately above.
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
    }
}

struct Fixture {
    directory: Option<tempfile::TempDir>,
    root: PathBuf,
    request: Cell<u64>,
    ui: Option<Child>,
    desktop: Option<Child>,
    input: Option<ChildStdin>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        for name in ["home", "bin", "repository"] {
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
            "#!/bin/sh\nprintf '\\nDESKTOP-PTY-READY\\n'\nexec /bin/cat\n",
        )
        .unwrap();
        fs::set_permissions(root.join("bin/claude"), fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            directory: Some(directory),
            root,
            request: Cell::new(0),
            ui: None,
            desktop: None,
            input: None,
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
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("SHELL", "/bin/zsh")
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }
    fn try_ctl(&self, args: &[&str]) -> Option<Value> {
        let number = self.request.get();
        self.request.set(number + 1);
        let stdout = self.root.join(format!("ctl-{number}.stdout"));
        let stderr = self.root.join(format!("ctl-{number}.stderr"));
        let mut child = self
            .command()
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(self.root.join("run"))
            .stdout(fs::File::create(&stdout).unwrap())
            .stderr(fs::File::create(&stderr).unwrap())
            .spawn()
            .unwrap();
        let status = wait_child(&mut child, Duration::from_secs(15)).unwrap_or_else(|| {
            let _ = child.kill();
            let _ = child.wait();
            panic!("private ctl timed out: {args:?}")
        });
        if !status.success() {
            return None;
        }
        let reply: Value = serde_json::from_slice(&fs::read(stdout).unwrap()).unwrap();
        (reply["ok"] == true).then(|| reply["data"].clone())
    }
    fn ctl(&self, args: &[&str]) -> Value {
        self.try_ctl(args).unwrap_or_else(|| {
            panic!(
                "private ctl failed: {args:?}; logs at {}",
                self.root.display()
            )
        })
    }
    fn wait_ui(&self) {
        wait_until("render-ready private UI", DEADLINE, || {
            self.try_ctl(&["app", "status"])
                .is_some_and(|s| s["gui_ready"] == true && s["backend_ready"] == true)
        });
    }
    fn start_ui(&mut self) {
        let log = fs::File::create(self.root.join("initial-ui.log")).unwrap();
        self.ui = Some(
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
        self.wait_ui();
        assert!(
            !self.root.join("run/desktop.owner.json").exists(),
            "ordinary headless fixtures must not create a native status item"
        );
    }
    fn owner(&self, name: &str) -> Value {
        let record: Value = serde_json::from_slice(
            &fs::read(self.root.join(format!("run/{name}.owner.json"))).unwrap(),
        )
        .unwrap();
        record["process"].clone()
    }
    fn launched_ui(&self) -> Value {
        serde_json::from_slice(&fs::read(self.root.join("run/desktop-launched-ui.json")).unwrap())
            .unwrap()
    }
    fn start_desktop(&mut self, supervisor: &Value) {
        let log = fs::File::create(self.root.join("desktop.log")).unwrap();
        let mut child = self
            .command()
            .arg("desktop")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(log)
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (sender, ready) = mpsc::channel();
        let reader = std::thread::spawn(move || {
            let mut line = String::new();
            let result = BufReader::new(stdout).read_line(&mut line).map(|_| line);
            let _ = sender.send(result);
        });
        self.desktop = Some(child);
        serde_json::to_writer(
            &mut input,
            &json!({
                "executable": BIN, "socket": self.root.join("run/mux.sock"),
                "run_dir": self.root.join("run"), "repository": self.root.join("repository"),
                "shell_config": null, "ctl": true, "test_account": self.root.join("account")
            }),
        )
        .unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
        self.input = Some(input);
        let line = ready
            .recv_timeout(DEADLINE)
            .expect("desktop native readiness timeout")
            .unwrap();
        reader.join().unwrap();
        let reply: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["version"], 1);
        assert_eq!(reply["supervisor_pid"], supervisor["pid"]);
        assert_eq!(
            self.owner("desktop")["pid"].as_u64(),
            Some(u64::from(self.desktop.as_ref().unwrap().id()))
        );
    }
    fn action(&mut self, action: &str) {
        let input = self.input.as_mut().unwrap();
        serde_json::to_writer(&mut *input, action).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
    }
    fn assert_desktop_alive(&mut self) {
        assert!(
            self.desktop.as_mut().unwrap().try_wait().unwrap().is_none(),
            "desktop must survive UI detach"
        );
    }
    fn seat(&self, id: &str) -> Value {
        self.ctl(&["seat", "ls"])
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["record"]["id"] == id)
            .unwrap()
            .clone()
    }
    fn assert_work_preserved(&self, owners: &[Value; 2], seat: &str, before: &Value) {
        assert_eq!([self.owner("supervisor"), self.owner("logic")], *owners);
        assert!(owners.iter().all(alive));
        let after = self.seat(seat);
        for path in ["/state/process", "/state/generation", "/record/generation"] {
            assert_eq!(
                after.pointer(path),
                before.pointer(path),
                "harness identity changed at {path}"
            );
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        // Every fallback is bounded and scoped to this fixture's recorded births.
        if let Some(input) = &mut self.input {
            let _ = input.write_all(b"\"hard_quit\"\n");
            let _ = input.flush();
        }
        if let Some(child) = &mut self.desktop {
            let _ = wait_child(child, Duration::from_secs(10));
        }
        if let Some(mut child) = self.ui.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        if let Ok(bytes) = fs::read(self.root.join("run/desktop-launched-ui.json"))
            && let Ok(identity) = serde_json::from_slice(&bytes)
        {
            stop_identity(&identity);
        }
        for name in ["logic", "supervisor"] {
            if let Ok(bytes) = fs::read(self.root.join(format!("run/{name}.owner.json")))
                && let Ok(record) = serde_json::from_slice::<Value>(&bytes)
            {
                stop_identity(&record["process"]);
            }
        }
        if let Some(mut child) = self.desktop.take() {
            if child.try_wait().ok().flatten().is_none() {
                let _ = child.kill();
            }
            let _ = child.wait();
        }
        if std::thread::panicking()
            && let Some(directory) = self.directory.take()
        {
            eprintln!(
                "private desktop failure retained at {}",
                directory.keep().display()
            );
        }
    }
}

// keeps: operator desktop ownership — detach/reopen never replaces PTY owners.
#[test]
#[ignore = "private native desktop/AppKit + headless UI/PTY lifecycle; make test-desktop"]
fn daemon_survives_ui_detach_and_reopens_the_same_running_harness() {
    let mut fixture = Fixture::new();
    fixture.start_ui();
    let owners = [fixture.owner("supervisor"), fixture.owner("logic")];
    let swarm = fixture.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let seat = fixture.ctl(&[
        "seat", "spawn", "claude", "--place", "tab", "--swarm", &swarm,
    ])["seat"]
        .as_str()
        .unwrap()
        .to_owned();
    fixture.ctl(&[
        "seat",
        "wait",
        &seat,
        "--text",
        "DESKTOP-PTY-READY",
        "--timeout",
        "10000",
    ]);
    let before = fixture.seat(&seat);
    assert_eq!(before["state"]["process"]["kind"], "running");
    fixture.start_desktop(&owners[0]);
    let mut ui = fixture.ui.take().unwrap();
    ui.kill().unwrap();
    assert!(wait_child(&mut ui, Duration::from_secs(5)).is_some());
    fixture.assert_desktop_alive();
    assert!(owners.iter().all(alive));

    fixture.action("show_window");
    fixture.wait_ui();
    let first_ui = fixture.launched_ui();
    assert!(alive(&first_ui));
    fixture.assert_work_preserved(&owners, &seat, &before);
    fixture.action("settings");
    wait_until("native desktop Settings action", DEADLINE, || {
        fixture.ctl(&["ui", "layout"])["page"] == "settings"
    });
    fixture.action("new_tab");
    wait_until("native desktop New Tab action", DEADLINE, || {
        fixture.ctl(&["ui", "layout"])["launcher_open"] == true
    });

    fixture.action("close_window");
    wait_until("desktop close detaches UI", DEADLINE, || !alive(&first_ui));
    fixture.assert_desktop_alive();
    assert!(owners.iter().all(alive));
    fixture.action("show_window");
    fixture.wait_ui();
    let second_ui = fixture.launched_ui();
    assert!(alive(&second_ui));
    assert_ne!(first_ui, second_ui);
    fixture.assert_work_preserved(&owners, &seat, &before);

    fixture.action("hard_quit");
    let status =
        wait_child(fixture.desktop.as_mut().unwrap(), DEADLINE).expect("desktop hard quit timeout");
    assert!(status.success());
    wait_until("hard quit stops backend owners and UI", DEADLINE, || {
        owners.iter().all(|owner| !alive(owner)) && !alive(&second_ui)
    });
    println!(
        "Desktop status-item owner survived UI detach; Show/Settings/New Tab/Close/Show preserved supervisor, logic and harness PID/generation; hard quit stopped this private run."
    );
}
