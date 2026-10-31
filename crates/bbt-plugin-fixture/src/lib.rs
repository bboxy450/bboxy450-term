// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Private host test fixture plugins, built on the plugin SDK.
//!
//! A variant is a set of [`Knobs`] compiled into the binary
//! (`BBOXY_PLUGIN_FIXTURE`, read with `option_env!`), so every variant has
//! its own pinned bytes and digest. Tests call [`Knobs::build`] (or
//! [`Knobs::install`]); each distinct variant is built once into a private
//! target directory next to the test executables and cached by its knobs
//! and the fixture, SDK and contract sources. Binaries are never patched.

// Test-support IO and a nested Cargo build; never UI code.
#![allow(clippy::disallowed_methods)]

use std::fs;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The compile-time variable that carries a variant's knobs.
pub const KNOBS_VAR: &str = "BBOXY_PLUGIN_FIXTURE";
/// The fixture executable's (and package's) name.
pub const BINARY: &str = "bboxy450-term-plugin-fixture";

/// Which fixture program the binary runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// The P-0 reference contract server and conformance adversary.
    #[default]
    Reference,
    /// A credential-free data_source over the Markdown files of its root.
    Root,
    /// A shape-only descriptor for inspection tests.
    Inspect,
    /// Out-of-order replies, a host request and a stray ID (client tests).
    Demux,
    /// Asks the host `host/<tool name>` while serving each call.
    Callback,
}

/// Every behaviour knob. Absent fields keep the reference behaviour.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Knobs {
    pub kind: Kind,
    /// Distinguishes otherwise identical variants (independent digests).
    pub nonce: Option<String>,
    // Reference.
    /// Contract token; `data_source` when absent.
    pub system: Option<String>,
    /// One named fault (see the reference fixture).
    pub fault: Option<String>,
    /// Declare `concurrent` and answer a delayed `changes_since` out of order.
    pub concurrent: bool,
    /// Hold contract calls for one second while this file exists.
    pub delay_file: Option<String>,
    /// Declare a worker tool `echo` and an oracle tool `audit`.
    pub tools: bool,
    /// Declared hooks (shape `hooks`).
    pub hooks: Vec<Value>,
    /// Append each delivered hook (and host answer) as one JSON line.
    pub hook_log: Option<String>,
    /// `allow`, `deny`, `deny-rm` or `slow`.
    pub decision: Option<String>,
    /// Requested host API methods; adds the worker tool `host`.
    pub host_api: Vec<String>,
    /// Per-command budget other than 2000 ms.
    pub timeout_ms: Option<u64>,
    /// Append `shape`, `initialize` and `call` observations as JSON lines.
    pub observe: Option<String>,
    /// Wait for one byte from this Unix socket during `initialize`
    /// (reference) or before printing the descriptor (inspect).
    pub initialize_gate: Option<String>,
    // Root.
    /// Append protocol trace lines.
    pub trace: Option<String>,
    /// Hold `fetch` on this Unix socket for `gate_root` (while `gate_armed` exists).
    pub gate: Option<String>,
    pub gate_root: Option<String>,
    pub gate_armed: Option<String>,
    /// Signed writer with commit, fixture bootstrap and a distinct external root.
    pub harmony: bool,
    // Inspect.
    /// The descriptor printed by `--shape`.
    pub descriptor: Option<Value>,
    /// Phase markers, as JSON lines.
    pub marker: Option<String>,
    /// The candidate's original path, checked against the trust record.
    pub source: Option<String>,
    /// The account database whose trust record must exist before `--shape`.
    pub account_db: Option<String>,
    /// Ignore SIGTERM and never print the descriptor.
    pub hang: bool,
}

impl Knobs {
    pub fn reference() -> Self {
        Self::default()
    }

    pub fn root() -> Self {
        Self {
            kind: Kind::Root,
            ..Self::default()
        }
    }

    pub fn of(kind: Kind) -> Self {
        Self {
            kind,
            ..Self::default()
        }
    }

    pub fn system(mut self, system: &str) -> Self {
        self.system = Some(system.into());
        self
    }

    pub fn fault(mut self, fault: &str) -> Self {
        self.fault = Some(fault.into());
        self
    }

    pub fn nonce(mut self, nonce: impl ToString) -> Self {
        self.nonce = Some(nonce.to_string());
        self
    }

    /// The knobs as compiled into the binary.
    pub fn encoded(&self) -> String {
        serde_json::to_string(self).expect("knobs serialize")
    }

    /// The knobs compiled into this binary (fixture side).
    pub fn compiled(encoded: Option<&str>) -> Self {
        encoded.map_or_else(Self::default, |text| {
            serde_json::from_str(text).expect("valid compiled fixture knobs")
        })
    }

    /// The cached executable for this variant, building it once.
    pub fn build(&self) -> PathBuf {
        build(self)
    }

    /// Copy this variant to `path` as an owner-only executable.
    pub fn install(&self, path: &Path) -> PathBuf {
        let built = self.build();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture plugin directory");
        }
        // A fresh file (never an overwritten one): exec of a replaced inode is safe.
        let staged = path.with_extension("fixture-copy");
        fs::copy(&built, &staged).expect("copy fixture plugin");
        fs::set_permissions(&staged, fs::Permissions::from_mode(0o700))
            .expect("fixture plugin mode");
        fs::rename(&staged, path).expect("place fixture plugin");
        path.to_path_buf()
    }
}

/// Workspace root of this source tree.
pub fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

// The private fixture directory beside the running test executable
// (`<target-or-build-dir>/<profile>/deps/<test>`), so concurrent worktrees
// and the shared machine-wide build directory never meet.
fn base() -> PathBuf {
    let exe = std::env::current_exe().expect("test executable path");
    exe.ancestors()
        .find(|dir| dir.file_name().is_some_and(|name| name == "deps"))
        .and_then(Path::parent)
        .and_then(Path::parent)
        .map_or_else(|| workspace().join("target"), Path::to_path_buf)
        .join("plugin-fixtures")
}

// FNV-1a 64: a stable cache key, not a security digest.
fn fnv(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
}

fn stamp(hash: &mut u64, path: &Path) {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return;
    };
    if meta.is_dir() {
        let mut entries: Vec<PathBuf> = fs::read_dir(path)
            .map(|dir| dir.filter_map(|e| e.ok().map(|e| e.path())).collect())
            .unwrap_or_default();
        entries.sort();
        for entry in entries {
            stamp(hash, &entry);
        }
    } else if let Ok(bytes) = fs::read(path) {
        fnv(hash, path.to_string_lossy().as_bytes());
        fnv(hash, &bytes);
    }
}

fn key(knobs: &Knobs) -> String {
    let root = workspace();
    let mut hash = 0xcbf2_9ce4_8422_2325;
    fnv(&mut hash, knobs.encoded().as_bytes());
    for part in [
        "Cargo.lock",
        "crates/bbt-plugin-fixture/src",
        "crates/bbt-plugin-fixture/Cargo.toml",
        "crates/bbt-plugin-sdk/src",
        "crates/bbt-plugin-sdk/Cargo.toml",
        "crates/bbt-plugin/src",
        "crates/bbt-plugin/schemas",
        "crates/bbt-plugin/tests/fixtures/data_source/conformance.json",
        "crates/bbt-plugin/Cargo.toml",
    ] {
        stamp(&mut hash, &root.join(part));
    }
    for var in ["RUSTUP_TOOLCHAIN", "RUSTFLAGS"] {
        fnv(&mut hash, std::env::var(var).unwrap_or_default().as_bytes());
    }
    format!("{hash:016x}")
}

static BUILD: Mutex<()> = Mutex::new(());

/// Seconds spent building variants in this process, for slow-test diagnosis.
pub static BUILD_SECONDS: Mutex<f64> = Mutex::new(0.0);

fn build(knobs: &Knobs) -> PathBuf {
    let base = base();
    let variant = base.join("variants").join(key(knobs));
    let binary = variant.join(BINARY);
    if binary.is_file() {
        return binary;
    }
    let _guard = BUILD.lock().unwrap_or_else(|poison| poison.into_inner());
    fs::create_dir_all(&base).expect("fixture build directory");
    // One build at a time across test processes sharing this directory.
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(base.join("build.lock"))
        .expect("fixture build lock");
    use std::os::fd::AsRawFd;
    // SAFETY: flock only locks the descriptor owned by `lock`; closing it unlocks.
    assert_eq!(
        unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX) },
        0,
        "fixture lock"
    );
    if binary.is_file() {
        return binary;
    }
    let started = Instant::now();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        // Small, stripped and optimized: the host hashes the pinned copy before
        // every spawn and call, and a 12 MB debug binary cost ~250 ms per call.
        .args([
            "build",
            "--locked",
            "--offline",
            "--quiet",
            "--release",
            "-p",
            BINARY,
            "--bin",
            BINARY,
        ])
        .env("CARGO_PROFILE_RELEASE_OPT_LEVEL", "s")
        .env("CARGO_PROFILE_RELEASE_STRIP", "symbols")
        .env("CARGO_PROFILE_RELEASE_DEBUG", "false")
        .arg("--manifest-path")
        .arg(workspace().join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", base.join("target"))
        .env("CARGO_BUILD_BUILD_DIR", base.join("build"))
        .env(KNOBS_VAR, knobs.encoded())
        .env("RUSTC_WRAPPER", "")
        .env("RUSTC_WORKSPACE_WRAPPER", "")
        .env_remove("CARGO_INCREMENTAL")
        .output()
        .expect("run cargo for the fixture plugin");
    assert!(
        output.status.success(),
        "fixture plugin build failed for {}:\n{}",
        knobs.encoded(),
        String::from_utf8_lossy(&output.stderr)
    );
    fs::create_dir_all(&variant).expect("fixture variant directory");
    let staged = variant.join(format!("{BINARY}.partial"));
    fs::copy(base.join("target/release").join(BINARY), &staged).expect("copy fixture build");
    fs::rename(&staged, &binary).expect("publish fixture variant");
    let seconds = started.elapsed().as_secs_f64();
    *BUILD_SECONDS.lock().unwrap_or_else(|p| p.into_inner()) += seconds;
    let _ = writeln!(
        std::io::stderr(),
        "plugin fixture: built {} in {seconds:.1}s",
        knobs.encoded()
    );
    binary
}

/// Wait for `path` to exist, for tests that synchronize through the fixture's files.
pub fn wait_for(path: &Path, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    path.exists()
}
