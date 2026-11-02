// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A package's shipped TLA+ models (`models/*.tla` with `.cfg`): read back
//! into the native `bboxy450-tlc` checker ([`bboxy450_tlc::parse`]) and, when
//! Java TLC is available, checked by it too. `Broken<Name>` controls must fail;
//! every other model must pass. Java verdicts follow `scripts/formal.py`.
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::probe::TempDir;
use super::{Options, Report};

/// The native corpus bound (`verification/README.md`).
const STATES: usize = 100_000;
/// One Java TLC run, as in `scripts/formal.py`.
const JAVA: Duration = Duration::from_secs(60);

pub(super) fn run(report: &mut Report, dir: &Path, options: &Options) {
    report.ran(&["models.present", "models.native", "models.java"]);
    let mut models: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| {
                    e.file_name()
                        .to_str()?
                        .strip_suffix(".tla")
                        .map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    if models.is_empty() {
        report.error(
            "models.present",
            format!("no models/*.tla in {}", dir.display()),
            "ship the plugin's lifecycle model and its Broken control: build them with bboxy450_tlc::build, export with build::artifacts into models/, and list models/ in the package",
        );
        report.skip("models.native", "no models");
        report.skip("models.java", "no models");
        return;
    }
    let java = options.java.as_ref().zip(options.tlc_jar.as_ref());
    if java.is_none() {
        report.skip(
            "models.java",
            "Java TLC is not configured (set BBOXY450_JAVA and BBOXY450_TLC_JAR, or --java/--tlc-jar)",
        );
    }
    for name in &models {
        let broken = name.starts_with("Broken");
        let expected = if broken { "a violation" } else { "pass" };
        let tla = std::fs::read_to_string(dir.join(format!("{name}.tla"))).unwrap_or_default();
        let Ok(cfg) = std::fs::read_to_string(dir.join(format!("{name}.cfg"))) else {
            report.error(
                "models.present",
                format!("{name}.tla has no {name}.cfg"),
                "ship the .cfg that bboxy450_tlc::build::artifacts exports beside each .tla",
            );
            continue;
        };
        if !broken && !models.contains(&format!("Broken{name}")) {
            report.warn(
                "models.native",
                format!("{name} has no Broken{name} control"),
                "ship a broken control that removes one guard and must fail (build::family)",
            );
        }
        match bboxy450_tlc::parse::parse(&tla, &cfg) {
            Err(error) => report.error(
                "models.native",
                format!("{name}: outside the native checker's subset: {error}"),
                "write the model with bboxy450_tlc::build and export it with build::artifacts; never hand-edit the .tla",
            ),
            Ok(model) => match model.check(STATES) {
                Err(error) => report.error(
                    "models.native",
                    format!("{name}: the native checker could not finish: {error}"),
                    "bound the model's domains so it explores fewer states",
                ),
                Ok(checked) => {
                    let failed = !checked.violations.is_empty();
                    if failed != broken {
                        let got = checked
                            .violations
                            .first()
                            .map_or_else(|| "it passed".to_owned(), |v| format!("{} is violated", v.property));
                        report.error(
                            "models.native",
                            format!("{name}: expected {expected}, but {got} natively ({} states)", checked.states.len()),
                            if broken {
                                "a Broken control must fail: remove the guard its property depends on"
                            } else {
                                "fix the model (or the design it describes) until its properties hold"
                            },
                        );
                    }
                }
            },
        }
        if let Some((java, jar)) = java {
            match tlc(java, jar, &tla, &cfg, name) {
                Err(error) => report.error(
                    "models.java",
                    format!("{name}: Java TLC did not reach a verdict: {error}"),
                    "keep the model inside what TLC 1.7.4 parses and checks within 60 s",
                ),
                Ok(verdict) if (verdict != "pass") != broken => report.error(
                    "models.java",
                    format!("{name}: expected {expected}, but Java TLC says {verdict}"),
                    "the model's properties must hold in TLC too; native and Java verdicts must agree",
                ),
                Ok(_) => {}
            }
        }
    }
}

/// One Java TLC run in a private directory: `pass`, `safety`, `deadlock` or
/// `liveness`, or why there is no verdict.
fn tlc(java: &Path, jar: &Path, tla: &str, cfg: &str, name: &str) -> Result<&'static str, String> {
    let own = TempDir::new("bboxy450-plugin-doctor-tlc").map_err(|e| e.to_string())?;
    std::fs::write(own.0.join(format!("{name}.tla")), tla).map_err(|e| e.to_string())?;
    std::fs::write(own.0.join(format!("{name}.cfg")), cfg).map_err(|e| e.to_string())?;
    let tmp = own.0.join("java-tmp");
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut child = Command::new(java)
        .arg(format!("-Djava.io.tmpdir={}", tmp.display()))
        .args([
            "-XX:ActiveProcessorCount=2",
            "-Xmx256m",
            "-XX:+UseSerialGC",
            "-cp",
        ])
        .arg(jar)
        .args(["tlc2.TLC", "-workers", "1", "-cleanup", "-config"])
        .arg(format!("{name}.cfg"))
        .arg("-metadir")
        .arg(own.0.join("metadata"))
        .arg(name)
        .current_dir(&own.0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot run {}: {e}", java.display()))?;
    let mut stdout = child.stdout.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut stdout, &mut text);
        text
    });
    let started = Instant::now();
    let status = loop {
        if let Ok(Some(status)) = child.try_wait() {
            break status;
        }
        if started.elapsed() > JAVA {
            let _ = child.kill();
            let _ = child.wait();
            return Err("timed out; incomplete is not a pass".into());
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let output = reader.join().unwrap_or_default();
    let code = status.code().unwrap_or(-1);
    if code == 0 && output.contains("Model checking completed. No error has been found.") {
        return Ok("pass");
    }
    for (verdict, exit, diagnostic) in [
        ("safety", 12, "is violated"),
        ("deadlock", 11, "Deadlock reached"),
        ("liveness", 13, "Temporal properties were violated"),
    ] {
        if code == exit && output.contains(diagnostic) {
            return Ok(verdict);
        }
    }
    let tail: String = output
        .chars()
        .rev()
        .take(400)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    Err(format!("exit {code}: {tail}"))
}
