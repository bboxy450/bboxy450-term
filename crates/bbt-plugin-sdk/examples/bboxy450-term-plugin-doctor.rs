// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! The plugin doctor CLI (`make plugin-doctor PLUGIN=<binary-or-package>`):
//! runs a plugin through the host contract offline and lists its gaps.
//! Exit 0 when no gap is an error, 1 otherwise, 2 on usage errors.
// A command-line tool with no UI thread.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;
use std::time::Duration;

use bbt_plugin_sdk::doctor::{self, CHECKS, Options};

const USAGE: &str = "usage: bboxy450-term-plugin-doctor <binary-or-package-dir> [--json]
  [--root DIR | --scratch DIR] [--package DIR] [--models DIR]
  [--child NAME]... [--ceiling-ms N] [--java PATH --tlc-jar PATH]
Runs the plugin offline (no app) and prints its gaps; exit 1 on any error.
--root binds a real knowledge root read-only; --scratch is a disposable root
it may write to; neither means a temporary root the doctor owns.
--child NAME observes a program the plugin starts through PATH.
--java/--tlc-jar (or BBOXY450_JAVA/BBOXY450_TLC_JAR) also run Java TLC.
--list prints every check.";

fn main() {
    let mut args = std::env::args().skip(1);
    let mut options = Options::default();
    let mut target = None;
    let mut json = false;
    while let Some(arg) = args.next() {
        let mut value = |name: &str| {
            args.next().unwrap_or_else(|| {
                eprintln!("{name} needs a value\n{USAGE}");
                std::process::exit(2)
            })
        };
        match arg.as_str() {
            "--json" => json = true,
            "--root" => options.root = Some(PathBuf::from(value("--root"))),
            "--scratch" => options.scratch = Some(PathBuf::from(value("--scratch"))),
            "--package" => options.package = Some(PathBuf::from(value("--package"))),
            "--models" => options.models = Some(PathBuf::from(value("--models"))),
            "--child" => options.children.push(value("--child")),
            "--java" => options.java = Some(PathBuf::from(value("--java"))),
            "--tlc-jar" => options.tlc_jar = Some(PathBuf::from(value("--tlc-jar"))),
            "--ceiling-ms" => {
                let ms = value("--ceiling-ms").parse().unwrap_or_else(|_| {
                    eprintln!("--ceiling-ms needs milliseconds\n{USAGE}");
                    std::process::exit(2)
                });
                options.ceiling = Duration::from_millis(ms);
            }
            "--list" => {
                for (id, what) in CHECKS {
                    println!("{id:<18} {what}");
                }
                return;
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            _ if target.is_none() && !arg.starts_with('-') => target = Some(PathBuf::from(arg)),
            _ => {
                eprintln!("unexpected argument {arg}\n{USAGE}");
                std::process::exit(2)
            }
        }
    }
    let Some(target) = target else {
        eprintln!("{USAGE}");
        std::process::exit(2)
    };
    if options.root.is_some() && options.scratch.is_some() {
        eprintln!("--root and --scratch are exclusive\n{USAGE}");
        std::process::exit(2)
    }
    let report = doctor::run(&target, &options);
    if json {
        println!("{}", report.json());
    } else {
        print!("{}", report.human());
    }
    std::process::exit(if report.is_clean() { 0 } else { 1 });
}
