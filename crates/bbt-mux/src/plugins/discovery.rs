//! File observations only: no PATH lookup, repository trust or executable calls.

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

use bbt_plugin::{
    Refusal,
    registry::{Candidate, Scope},
    shape::PluginName,
};
use sha2::{Digest, Sha256};

use super::{failure, trust::hash_file};

const MAX_ENTRIES: usize = 512;

/// Explicit roots keep test-account discovery out of the operator's HOME.
#[derive(Debug, Clone)]
pub struct DiscoveryRoots {
    application: PathBuf,
    repository: PathBuf,
}

impl DiscoveryRoots {
    pub fn new(app_support: &Path, repository: &Path) -> Self {
        Self {
            application: app_support.join("plugins"),
            repository: repository.join(".bboxy/plugins"),
        }
    }

    pub(super) fn scan(&self, only_digest: Option<&str>) -> Result<Vec<Candidate>, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let mut entries = Vec::new();
        for (scope, root) in [
            (Scope::Application, &self.application),
            (Scope::Repository, &self.repository),
        ] {
            let metadata = match fs::symlink_metadata(root) {
                Ok(metadata) => metadata,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(failure("plugin_discovery_failed", e)),
            };
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(failure("plugin_root_refused", root.display()));
            }
            let root = root
                .canonicalize()
                .map_err(|e| failure("plugin_discovery_failed", e))?;
            for item in fs::read_dir(root).map_err(|e| failure("plugin_discovery_failed", e))? {
                if entries.len() == MAX_ENTRIES {
                    return Err(failure(
                        "plugin_discovery_limit",
                        "more than 512 entries; no partial inventory authorizes execution",
                    ));
                }
                let path = item
                    .map_err(|e| failure("plugin_discovery_failed", e))?
                    .path();
                entries.push(observe(scope, &path, only_digest));
            }
        }
        entries.sort_by(|a, b| a.source.cmp(&b.source).then(a.id.cmp(&b.id)));
        let mut names: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, entry) in entries.iter().enumerate() {
            if let Some(name) = &entry.name {
                names
                    .entry(format!("{}_{}", name.system.name(), name.provider))
                    .or_default()
                    .push(i);
            }
        }
        for indices in names.values().filter(|indices| indices.len() > 1) {
            for i in indices {
                entries[*i].refusal = Some(failure(
                    "plugin_name_collision",
                    "all candidates with this system/provider are refused; no root takes precedence",
                ));
            }
        }
        Ok(entries)
    }
}

fn observe(scope: Scope, path: &Path, only_digest: Option<&str>) -> Candidate {
    let source = path.to_string_lossy().into_owned();
    let id = format!("{:x}", Sha256::digest(format!("{}:{source}", scope.name())));
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| failure("invalid_plugin_name", "filename must be UTF-8"))
        .and_then(|name| PluginName::parse("bboxy450-term", name));
    let mut entry = Candidate {
        id,
        source,
        scope,
        name: name.as_ref().ok().cloned(),
        sha256: None,
        refusal: name.err(),
    };
    let inspect = || {
        let metadata =
            fs::symlink_metadata(path).map_err(|e| failure("plugin_file_unavailable", e))?;
        if metadata.file_type().is_symlink() {
            return Err(failure("plugin_symlink_refused", path.display()));
        }
        if !metadata.is_file() {
            return Err(failure("directory_plugin_refused", path.display()));
        }
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(failure("plugin_not_executable", path.display()));
        }
        if only_digest.is_none_or(|selected| selected == entry.id) {
            hash_file(path).map(Some)
        } else {
            Ok(None)
        }
    };
    match inspect() {
        Ok(digest) => entry.sha256 = digest,
        Err(reason) => entry.refusal = Some(reason),
    }
    entry
}
