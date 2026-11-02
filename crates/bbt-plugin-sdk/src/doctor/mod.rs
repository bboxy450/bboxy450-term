// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The plugin doctor: run a built plugin through the host contract offline,
//! with no running app, and list concrete gaps. Each [`Gap`] names its check,
//! the observed evidence and how to fix it.
//!
//! [`run`] takes a plugin executable or a Cargo package directory (built
//! with `cargo build`; its `models/` are checked too). The CLI is the SDK
//! example `bboxy450-term-plugin-doctor` (`make plugin-doctor PLUGIN=…`);
//! tests call [`crate::testkit::doctor`]. [`CHECKS`] lists every check.
//!
//! Writes happen only in a root the doctor owns (the default, a temporary
//! directory) or one passed as [`Options::scratch`], never in
//! [`Options::root`]. The doctor is a smoke test, not trust: the host's own
//! conformance checker (`bboxy450-term plugin check`) still runs at trust.

mod env;
mod formal;
mod probe;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};

/// Every check, with what it proves. Gap ids are these names.
pub const CHECKS: &[(&str, &str)] = &[
    (
        "shape.name",
        "file name is bboxy450-term-<contract>-<provider>; a package's binaries equal its bboxy450- package name",
    ),
    (
        "shape.descriptor",
        "--shape answers within 2 s and passes the host's descriptor validator",
    ),
    (
        "shape.digest",
        "the binary's SHA-256 (the trust digest) and its --shape bytes are identical across runs",
    ),
    (
        "shape.catalogue",
        "tools/list terminates and lists every contract and declared tool with compiling schemas",
    ),
    (
        "shape.skills",
        "each declared skill is listed and reads back byte for byte",
    ),
    (
        "contract.dto",
        "methods, declared tools, hooks and health answer DTOs valid against the contract schema, the declared schema and the typed SDK contract",
    ),
    (
        "call.refusal",
        "every failure is a typed tool refusal: no panic, JSON-RPC error, crash or missing reply",
    ),
    (
        "call.cold_start",
        "initialize and the first call fit the host's 2 s budget",
    ),
    (
        "call.deadline",
        "every call fits min(declared timeout_ms, host ceiling)",
    ),
    (
        "result.size",
        "results stay under MAX_RESULT (256 KiB); large unpaged results are flagged",
    ),
    (
        "page.limit",
        "limit is honoured, the default page is at most 128 and oversize limits are clamped or refused",
    ),
    ("page.cursor", "cursors terminate and never repeat"),
    (
        "page.order",
        "keyset order is stable, with no duplicates or skips across page sizes",
    ),
    (
        "identity.stable",
        "identical calls return identical ids, digests and etags",
    ),
    (
        "identity.id",
        "an id resolves to itself and external_id is never swapped in",
    ),
    (
        "write.roundtrip",
        "a write followed by a fetch returns exactly what was written (writable roots only)",
    ),
    (
        "write.status",
        "an unmappable status is refused or stored verbatim (writable roots only)",
    ),
    (
        "pending.retry_hint",
        "pending and busy refusals carry retry_after_ms",
    ),
    (
        "concurrency.busy",
        "pipelined calls each get one reply and busy is retryable",
    ),
    (
        "env.scrubbed",
        "the plugin works under the host's scrubbed environment",
    ),
    (
        "env.children",
        "no forbidden or earlier-call attribution reaches an observed child process",
    ),
    (
        "models.present",
        "the package ships models/*.tla, each with its .cfg",
    ),
    (
        "models.native",
        "each model checks with the native bboxy450-tlc; Broken* controls fail",
    ),
    (
        "models.java",
        "the same verdicts with Java TLC, when it is available",
    ),
];

/// How much a gap matters. Only errors fail the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

/// One concrete gap: the check, what was observed and how to fix it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Gap {
    pub check: &'static str,
    pub severity: Severity,
    pub evidence: String,
    pub fix: String,
}

/// The doctor's findings for one plugin.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Report {
    /// The executable that was run.
    pub binary: PathBuf,
    pub gaps: Vec<Gap>,
    /// Checks that ran and found no error or warning.
    pub passed: Vec<&'static str>,
    /// Checks that could not run here, with the reason.
    pub skipped: Vec<(&'static str, String)>,
}

impl Report {
    fn gap(
        &mut self,
        check: &'static str,
        severity: Severity,
        evidence: impl Into<String>,
        fix: &str,
    ) {
        debug_assert!(CHECKS.iter().any(|(id, _)| *id == check), "{check}");
        let gap = Gap {
            check,
            severity,
            evidence: evidence.into(),
            fix: fix.into(),
        };
        if !self.gaps.contains(&gap) {
            self.gaps.push(gap);
        }
    }
    fn error(&mut self, check: &'static str, evidence: impl Into<String>, fix: &str) {
        self.gap(check, Severity::Error, evidence, fix);
    }
    fn warn(&mut self, check: &'static str, evidence: impl Into<String>, fix: &str) {
        self.gap(check, Severity::Warning, evidence, fix);
    }
    fn skip(&mut self, check: &'static str, reason: impl Into<String>) {
        if !self.skipped.iter().any(|(id, _)| *id == check) {
            self.skipped.push((check, reason.into()));
        }
    }
    /// Mark `checks` as run; those without a gap and not skipped pass.
    fn ran(&mut self, checks: &[&'static str]) {
        for check in checks {
            if !self.passed.contains(check) {
                self.passed.push(check);
            }
        }
    }
    fn settle(&mut self) {
        let gaps = self.gaps.clone();
        let skipped = self.skipped.clone();
        self.passed.retain(|id| {
            !gaps.iter().any(|g| g.check == *id) && !skipped.iter().any(|(s, _)| s == id)
        });
        self.skipped
            .retain(|(id, _)| !gaps.iter().any(|g| g.check == *id));
        let order = |id: &str| CHECKS.iter().position(|(c, _)| *c == id);
        self.gaps
            .sort_by_key(|g| (order(g.check), g.severity, g.evidence.clone()));
        self.passed.sort_by_key(|id| order(id));
        self.skipped.sort_by_key(|(id, _)| order(id));
    }

    /// Gaps of [`Severity::Error`].
    pub fn errors(&self) -> impl Iterator<Item = &Gap> {
        self.gaps.iter().filter(|g| g.severity == Severity::Error)
    }

    /// True when no gap is an error.
    pub fn is_clean(&self) -> bool {
        self.errors().next().is_none()
    }

    /// Gaps raised by `check`.
    pub fn of(&self, check: &str) -> Vec<&Gap> {
        self.gaps.iter().filter(|g| g.check == check).collect()
    }

    /// The machine-readable form (`--json`).
    pub fn json(&self) -> Value {
        json!({
            "binary": self.binary,
            "ok": self.is_clean(),
            "gaps": self.gaps,
            "passed": self.passed,
            "skipped": self.skipped.iter().map(|(check, reason)| json!({"check": check, "reason": reason})).collect::<Vec<_>>(),
        })
    }

    /// The author-facing list.
    pub fn human(&self) -> String {
        let mut out = format!("plugin doctor: {}\n", self.binary.display());
        for gap in &self.gaps {
            let level = match gap.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
            };
            out.push_str(&format!(
                "  {level:<7} {:<18} {}\n          fix: {}\n",
                gap.check, gap.evidence, gap.fix
            ));
        }
        for (check, reason) in &self.skipped {
            out.push_str(&format!("  skipped {check:<18} {reason}\n"));
        }
        let errors = self.errors().count();
        out.push_str(&format!(
            "{} passed, {errors} error(s), {} warning(s), {} skipped\n",
            self.passed.len(),
            self.gaps.len() - errors,
            self.skipped.len()
        ));
        out
    }
}

/// What to run against. Defaults: a temporary root the doctor owns (writes
/// allowed), the host's 2 s call ceiling, no observed children, no models
/// unless the target is a package.
#[derive(Debug, Clone)]
pub struct Options {
    /// A real knowledge root to bind; only reads run against it.
    pub root: Option<PathBuf>,
    /// A disposable root the doctor may write to.
    pub scratch: Option<PathBuf>,
    /// The package directory (name and brand rules, `models/`).
    pub package: Option<PathBuf>,
    /// The models directory; defaults to `<package>/models`.
    pub models: Option<PathBuf>,
    /// Child executables (names on `PATH`) whose environment to observe.
    pub children: Vec<String>,
    /// The host's per-call ceiling (`ProviderConfig::timeout`, 2 s).
    pub ceiling: Duration,
    /// Java and `tla2tools.jar` for the Java TLC run; unset skips it.
    pub java: Option<PathBuf>,
    pub tlc_jar: Option<PathBuf>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            root: None,
            scratch: None,
            package: None,
            models: None,
            children: Vec::new(),
            ceiling: Duration::from_millis(crate::DEFAULT_TIMEOUT_MS),
            java: std::env::var_os("BBOXY450_JAVA")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            tlc_jar: std::env::var_os("BBOXY450_TLC_JAR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
        }
    }
}

impl Options {
    /// Check the plugin as part of the package at `dir` (usually
    /// `env!("CARGO_MANIFEST_DIR")`): its name rules and its `models/`.
    pub fn package(dir: impl Into<PathBuf>) -> Self {
        Self {
            package: Some(dir.into()),
            ..Self::default()
        }
    }
}

/// Run every check against `target`: an executable, or a package directory
/// holding `Cargo.toml` (built first, then checked with its `models/`).
pub fn run(target: &Path, options: &Options) -> Report {
    let mut options = options.clone();
    let mut report = Report::default();
    let binary = if target.join("Cargo.toml").is_file() {
        options.package.get_or_insert_with(|| target.to_owned());
        match build_package(target) {
            Ok(binary) => binary,
            Err(error) => {
                report.binary = target.to_owned();
                report.error(
                    "shape.name",
                    format!("the package did not build to one plugin binary: {error}"),
                    "give the package exactly one [[bin]] named after the package",
                );
                report.settle();
                return report;
            }
        }
    } else {
        target.to_owned()
    };
    // Children run in their own data directory: never a relative path.
    let binary = binary.canonicalize().unwrap_or(binary);
    report.binary = binary.clone();
    if let Some(package) = &options.package {
        name_rules(&mut report, &binary, package);
    }
    probe::run(&mut report, &binary, &options);
    let models = options
        .models
        .clone()
        .or_else(|| options.package.as_ref().map(|p| p.join("models")));
    match models {
        Some(dir) => formal::run(&mut report, &dir, &options),
        None => {
            for check in ["models.present", "models.native", "models.java"] {
                report.skip(
                    check,
                    "a bare binary carries no models; pass the package directory (or --models)",
                );
            }
        }
    }
    report.settle();
    report
}

/// The package's metadata, from `cargo metadata`.
fn metadata(package: &Path) -> Result<Value, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(package.join("Cargo.toml"))
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let all: Value = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;
    let manifest = package
        .join("Cargo.toml")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    all["packages"]
        .as_array()
        .and_then(|packages| {
            packages
                .iter()
                .find(|p| p["manifest_path"].as_str().map(Path::new) == Some(manifest.as_path()))
        })
        .cloned()
        .ok_or_else(|| "cargo metadata does not list this package".into())
}

fn bins(package: &Value) -> Vec<String> {
    package["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| {
            t["kind"]
                .as_array()
                .is_some_and(|k| k.iter().any(|k| k == "bin"))
        })
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect()
}

/// Build the package's one binary and return its path.
fn build_package(dir: &Path) -> Result<PathBuf, String> {
    let package = metadata(dir)?;
    let name = package["name"].as_str().unwrap_or_default().to_owned();
    let bins = bins(&package);
    let [bin] = bins.as_slice() else {
        return Err(format!("{} binaries: {bins:?}", bins.len()));
    };
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args([
            "build",
            "--message-format",
            "json",
            "-p",
            &name,
            "--bin",
            bin,
            "--manifest-path",
        ])
        .arg(dir.join("Cargo.toml"))
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!("cargo build exited {}", output.status));
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|m| m["reason"] == "compiler-artifact" && m["target"]["name"] == bin.as_str())
        .find_map(|m| m["executable"].as_str().map(PathBuf::from))
        .ok_or_else(|| "cargo reported no executable".into())
}

/// Decision 362: an installed binary equals its `bboxy450-` package name.
fn name_rules(report: &mut Report, binary: &Path, package: &Path) {
    report.ran(&["shape.name"]);
    let package = match metadata(package) {
        Ok(package) => package,
        Err(error) => {
            report.skip("shape.name", format!("cargo metadata failed: {error}"));
            return;
        }
    };
    let name = package["name"].as_str().unwrap_or_default();
    if !name.starts_with("bboxy450-") {
        report.error(
            "shape.name",
            format!("package {name:?} is not bboxy450- branded"),
            "name the package bboxy450-term-<contract>-<provider> (decision 362, scripts/brand.sh)",
        );
    }
    for bin in bins(&package) {
        if bin != name {
            report.error(
                "shape.name",
                format!("installed binary {bin:?} differs from package {name:?}"),
                "make the package name and its one [[bin]] equal; helper tools are examples",
            );
        }
    }
    let file = binary.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if bins(&package).iter().any(|b| b == file) && file != name {
        report.error(
            "shape.name",
            format!("binary {file:?} is not the package name {name:?}"),
            "rename the binary after its package",
        );
    }
}
