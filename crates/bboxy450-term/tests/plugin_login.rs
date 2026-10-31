// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Plugin setup (#57) through the real app and `ctl`, unattended: the login
//! runs in a visible "plugin login" seat, the test types the code into it
//! with `ctl seat type` / `ctl seat key` as an operator would, and the result,
//! the closed row and the ended process group are checked.
#![allow(
    clippy::disallowed_methods,
    reason = "private unattended app/PTY fixture"
)]
use serde_json::Value;
use std::io::Read as _;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

const BIN: &str = env!("CARGO_BIN_EXE_bboxy450-term");

// Adapted from crates/bboxy450-term/tests/swarm_workdir.rs `Fixture` (same
// project): a private HOME, account and run directory, headless with ctl.
struct App {
    _directory: tempfile::TempDir,
    root: PathBuf,
    app: Option<Child>,
}

impl App {
    fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        for name in ["home", "repository/.bboxy/plugins"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }
        let mut app = Self {
            _directory: directory,
            root,
            app: None,
        };
        let log = fs::File::create(app.root.join("app.log")).unwrap();
        app.app = Some(
            app.command()
                .args(["--headless", "--ctl", "--unattended", "--test-account"])
                .arg(app.root.join("account"))
                .arg("--run-dir")
                .arg(app.root.join("run"))
                .arg("--repository")
                .arg(app.root.join("repository"))
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap(),
        );
        app.until("the app", || {
            let output = app
                .command()
                .args(["ctl", "app", "status", "--run-dir"])
                .arg(app.root.join("run"))
                .output()
                .ok()?;
            let status: Value = serde_json::from_slice(&output.stdout).ok()?;
            (status["data"]["gui_ready"] == true && status["data"]["backend_ready"] == true)
                .then_some(())
        });
        app
    }

    fn command(&self) -> Command {
        let mut command = Command::new(BIN);
        command
            .env_clear()
            .env("HOME", self.root.join("home"))
            .env("ZDOTDIR", self.root.join("home"))
            .env("PATH", "/usr/bin:/bin:/usr/sbin:/sbin")
            .env("TERM", "xterm-256color")
            .env("LANG", "en_US.UTF-8")
            .current_dir(&self.root)
            .stdin(Stdio::null());
        command
    }

    fn ctl_command(&self, args: &[&str]) -> Command {
        let mut command = self.command();
        command
            .arg("ctl")
            .args(args)
            .arg("--run-dir")
            .arg(self.root.join("run"));
        command
    }

    fn ctl(&self, args: &[&str]) -> Value {
        let output = self.ctl_command(args).output().unwrap();
        let reply: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "{args:?}: {} {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                self.log()
            )
        });
        assert!(
            output.status.success() && reply["ok"] == true,
            "{args:?}: {reply}\n{}",
            self.log()
        );
        reply["data"].clone()
    }

    fn log(&self) -> String {
        let mut log = fs::read_to_string(self.root.join("app.log")).unwrap_or_default();
        for entry in fs::read_dir(self.root.join("run"))
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry.path().extension().is_some_and(|ext| ext == "log") {
                log.push_str(&format!("\n== {}\n", entry.path().display()));
                log.push_str(&fs::read_to_string(entry.path()).unwrap_or_default());
            }
        }
        log
    }

    fn until<T>(&self, what: &str, mut probe: impl FnMut() -> Option<T>) -> T {
        let deadline = Instant::now() + Duration::from_secs(45);
        loop {
            if let Some(found) = probe() {
                return found;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {what}\n{}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    /// Install a typed-login fixture plugin and trust it; its candidate ID
    /// and data directory.
    fn trusted(&self, file: &str, nonce: &str) -> (String, PathBuf) {
        let source = self.root.join("repository/.bboxy/plugins").join(file);
        bbt_plugin_fixture::Knobs {
            auth: Some("typed".into()),
            ..bbt_plugin_fixture::Knobs::root()
        }
        .nonce(nonce)
        .install(&source);
        let trusted = self.ctl(&["plugin", "trust", source.to_str().unwrap()]);
        let entry = trusted["entries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["candidate"]["source"] == source.to_str().unwrap())
            .unwrap_or_else(|| panic!("{trusted}"))
            .clone();
        (
            entry["candidate"]["id"].as_str().unwrap().to_owned(),
            PathBuf::from(entry["trust"]["data_dir"].as_str().unwrap()),
        )
    }

    /// The open plugin login row, once it appears.
    fn login_seat(&self, setup: &mut Child) -> String {
        self.until("the plugin login row", || {
            if let Ok(Some(status)) = setup.try_wait() {
                let mut text = String::new();
                for pipe in [
                    setup
                        .stdout
                        .take()
                        .map(|p| Box::new(p) as Box<dyn std::io::Read>),
                    setup
                        .stderr
                        .take()
                        .map(|p| Box::new(p) as Box<dyn std::io::Read>),
                ]
                .into_iter()
                .flatten()
                {
                    let mut pipe = pipe;
                    let _ = pipe.read_to_string(&mut text);
                }
                panic!("setup ended before its login row ({status}): {text}");
            }
            self.ctl(&["seat", "ls"])
                .as_array()?
                .iter()
                .find(|row| {
                    row["record"]["harness_kind"] == "plugin-login"
                        && row["record"]["lifecycle"] == "open"
                })
                .map(|row| row["record"]["id"].as_str().unwrap().to_owned())
        })
    }

    /// Run setup, typing `code` into the login row; the setup reply's entry.
    fn setup_typing(&self, candidate: &str, swarm: Option<&str>, code: &str) -> (Value, String) {
        let mut args = vec!["plugin", "setup", candidate];
        if let Some(swarm) = swarm {
            args.extend(["--swarm", swarm]);
        }
        let mut setup = self
            .ctl_command(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let seat = self.login_seat(&mut setup);
        // The login's prompt is on the row's screen (ctl fails if it never is).
        self.ctl(&[
            "seat",
            "wait",
            &seat,
            "--text",
            "Code:",
            "--timeout",
            "20000",
        ]);
        self.ctl(&["seat", "type", &seat, code]);
        self.ctl(&["seat", "key", &seat, "enter"]);
        let output = setup.wait_with_output().unwrap();
        let reply: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
            panic!(
                "setup: {} {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                self.log()
            )
        });
        assert_eq!(reply["ok"], true, "{reply}");
        (reply["data"]["entries"][0].clone(), seat)
    }

    fn row(&self, seat: &str) -> Value {
        self.ctl(&["seat", "ls"])
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["record"]["id"] == seat)
            .cloned()
            .unwrap_or(Value::Null)
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self
            .command()
            .args(["ctl", "app", "stop", "--run-dir"])
            .arg(self.root.join("run"))
            .output();
        if let Some(mut app) = self.app.take() {
            let deadline = Instant::now() + Duration::from_secs(10);
            while app.try_wait().ok().flatten().is_none() && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = app.kill();
            let _ = app.wait();
        }
    }
}

fn gone(pid: i32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        // Existence check only: signal 0 is never delivered.
        if unsafe { libc::kill(pid, 0) } == -1 {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

fn grandchild(data_dir: &Path) -> i32 {
    fs::read_to_string(data_dir.join("grandchild.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

// keeps: #57 visible plugin login — the operator's typed code completes the
// login in its own row; the result is recorded, the row closes and the
// login's whole process group (a SIGHUP-ignoring grandchild) is ended. A
// wrong code is recorded as a refused login.
#[test]
fn a_typed_login_in_the_plugin_login_row_logs_in_or_is_refused() {
    let app = App::start();
    let (candidate, data_dir) = app.trusted("bboxy450-term-data_source-typed-fixture", "typed");
    // As Settings does: the login row joins the window's swarm.
    let swarm = app.ctl(&["ui", "layout"])["swarm"]
        .as_str()
        .unwrap()
        .to_owned();
    let (entry, seat) = app.setup_typing(&candidate, Some(&swarm), "FIX-1234");
    assert_eq!(entry["setup"]["state"], "logged_in", "{entry}");
    assert_eq!(entry["setup"]["login_exit"], 0, "{entry}");
    assert_eq!(entry["setup"]["detail"], "fixture account", "{entry}");
    assert_eq!(entry["state"], "disabled", "setup never enables: {entry}");
    assert!(data_dir.join("logged-in").exists());
    let child = grandchild(&data_dir);
    assert!(gone(child), "the login's grandchild {child} outlived it");
    let row = app.row(&seat);
    assert_ne!(row["record"]["lifecycle"], "open", "the row closes: {row}");
    assert_eq!(row["record"]["swarm"], swarm.as_str(), "{row}");

    // Unscoped (ctl without --swarm), and a wrong code.
    let (refused, _) = app.trusted("bboxy450-term-data_source-refused-fixture", "refused");
    let (entry, _) = app.setup_typing(&refused, None, "WRONG");
    assert_eq!(entry["setup"]["state"], "login_refused", "{entry}");
    assert_eq!(entry["setup"]["login_exit"], 2, "{entry}");
}
