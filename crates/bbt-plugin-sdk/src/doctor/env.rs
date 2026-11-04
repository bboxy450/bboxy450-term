// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Observe what a plugin passes to its children: a recording shim for each
//! named child, first on the plugin's `PATH`, writes the child's environment
//! and then runs the real program (or exits 0 when there is none).
use std::collections::BTreeSet;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

/// A forbidden name smuggled into `_meta`: it must never reach a child.
pub(super) const SECRET: &str = "bboxy450-doctor-secret-canary";
/// An allowlisted attribution value: only its own call's children may see it.
pub(super) const SESSION: &str = "bboxy450-doctor-session-canary";

/// The `_meta` of an attributed call, with one forbidden name.
pub(super) fn attributed() -> Value {
    json!({"bboxy450/attribution": {"env": {
        "ATC_SESSION_ID": SESSION,
        "SESSION_ACTIVITY_PROVIDER": "doctor",
        "ATC_TOKEN": SECRET,
    }}})
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', "'\\''"))
}

/// Write one shim per name into `shims`, recording into `observed`. Returns
/// the `PATH` the plugin should run with.
pub(super) fn shims(
    names: &[String],
    path: &str,
    shims: &Path,
    observed: &Path,
) -> std::io::Result<String> {
    std::fs::create_dir_all(shims)?;
    std::fs::create_dir_all(observed)?;
    for name in names {
        let real = std::env::split_paths(path)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.is_file());
        let run = real.map_or_else(
            || "exit 0".to_owned(),
            |real| format!("exec {} \"$@\"", quote(&real)),
        );
        let script = format!(
            "#!/bin/sh\n# bboxy450 plugin doctor: record the environment, then run the real {name}.\nenv > {}/{name}.$$.env\n{run}\n",
            quote(observed)
        );
        let file = shims.join(name);
        std::fs::write(&file, script)?;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(format!("{}:{path}", shims.display()))
}

/// The recorded child environments so far.
pub(super) fn recorded(observed: &Path) -> BTreeSet<PathBuf> {
    std::fs::read_dir(observed)
        .map(|dir| dir.filter_map(|e| e.ok().map(|e| e.path())).collect())
        .unwrap_or_default()
}

/// The child's name from a recording's file name.
pub(super) fn child(file: &Path) -> String {
    file.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.split('.').next())
        .unwrap_or("child")
        .to_owned()
}
