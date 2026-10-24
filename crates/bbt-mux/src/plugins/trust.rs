//! The checker owns one disposable account, never a production account handle.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use bbt_plugin::{
    Refusal,
    policy::Tier,
    shape::{PluginName, Shape},
    trust::TrustRecord,
};
use sha2::{Digest, Sha256};

use super::failure;
use crate::store::{Store, sqlite::SqliteStore};

pub(super) struct FixtureAccount {
    _root: tempfile::TempDir,
    store: SqliteStore,
    pub record: TrustRecord,
    pub shape_runs: usize,
}

fn open_regular(path: &Path) -> Result<File, Refusal> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|e| failure("plugin_file_unavailable", e))?;
    if !file
        .metadata()
        .map_err(|e| failure("plugin_file_unavailable", e))?
        .is_file()
    {
        return Err(failure(
            "directory_plugin_refused",
            "v1 pins one regular executable file",
        ));
    }
    Ok(file)
}

fn hash_file(path: &Path) -> Result<String, Refusal> {
    let mut file = open_regular(path)?;
    let mut digest = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|e| failure("plugin_file_unavailable", e))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()))
}

fn production_plugin_path(source: &Path) -> bool {
    let parts: Vec<_> = source
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    if parts.windows(2).any(|p| p == ["plugins", "bin"]) {
        return true;
    }
    let mut roots = Vec::new();
    if let Some(run) = std::env::var_os("BBOXY_RUN_DIR") {
        roots.push(PathBuf::from(run).join("plugins"));
    }
    if let Some(home) = std::env::var_os("HOME") {
        let home = PathBuf::from(home);
        roots.push(home.join("Library/Application Support/bboxy450-term/plugins"));
        roots.push(home.join(".local/share/bboxy450-term/plugins"));
    }
    if let Some(data) = std::env::var_os("XDG_DATA_HOME") {
        roots.push(PathBuf::from(data).join("bboxy450-term/plugins"));
    }
    roots.iter().any(|root| {
        source.starts_with(root)
            || root
                .canonicalize()
                .is_ok_and(|root| source.starts_with(root))
    })
}

impl FixtureAccount {
    pub fn signed_task_fixture(&self) -> Result<String, Refusal> {
        // Existing DSSE writer, an explicitly public fixture key, and a root we
        // own. This is a test vector, never a production task transition/key.
        let root = self._root.path().join("checker-knowledge/.bboxy");
        let index = crate::index::Index::open(&root, self.store.clone())
            .map_err(|e| failure("fixture_record_unavailable", e))?;
        let key = ed25519_dalek::SigningKey::from_bytes(&[73; 32]);
        let seat = crate::Ulid::from(7_u128);
        index
            .write(
                "tasks/p0-fixture.md",
                serde_json::json!({
                    "id":seat.to_string(),"type":"task","title":"P-0 conformance fixture",
                    "status":"reported","assignee":format!("seat:{seat}"),
                    "verify":"true","report":{"summary":"fixture only"},
                    "verify_result":{"command":"true","exit_code":0,"evidence":[]}
                }),
                "P-0 fixture only, not an accepted task.\n",
                seat,
                &key,
            )
            .map_err(|e| failure("fixture_record_unavailable", e))?;
        fs::read_to_string(root.join("tasks/p0-fixture.md"))
            .map_err(|e| failure("fixture_record_unavailable", e))
    }

    pub fn prepare(binary: &Path) -> Result<Self, Refusal> {
        bbt_rt::assert_not_ui_thread();
        let metadata =
            fs::symlink_metadata(binary).map_err(|e| failure("plugin_file_unavailable", e))?;
        if metadata.is_dir() {
            return Err(failure(
                "directory_plugin_refused",
                "v1 pins one executable file, not a directory",
            ));
        }
        if metadata.file_type().is_symlink() {
            return Err(failure(
                "plugin_symlink_refused",
                "supply the single executable itself",
            ));
        }
        let source = fs::canonicalize(binary).map_err(|e| failure("plugin_file_unavailable", e))?;
        // Checking a fixture is not an alternative production-trust entry point.
        // No production store is opened, even to see whether this file is trusted.
        if production_plugin_path(&source) {
            return Err(failure(
                "untrusted_production_file",
                "plugin check only seeds an isolated fixture account; production trust belongs to ctl plugin trust",
            ));
        }
        let filename = source
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| failure("invalid_plugin_name", "executable name must be UTF-8"))?;
        let name = PluginName::parse("bboxy450-term", filename)?;
        let mut input = open_regular(&source)?;
        if input
            .metadata()
            .map_err(|e| failure("plugin_file_unavailable", e))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(failure(
                "plugin_not_executable",
                "the fixture must be executable",
            ));
        }
        let root = tempfile::Builder::new()
            .prefix("bboxy-plugin-check-")
            .tempdir()
            .map_err(|e| failure("fixture_store_unavailable", e))?;
        let bins = root.path().join("plugins/bin");
        fs::create_dir_all(&bins).map_err(|e| failure("fixture_store_unavailable", e))?;
        let temporary = bins.join("copy-in-progress");
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|e| failure("fixture_store_unavailable", e))?;
        let mut digest = Sha256::new();
        let mut buffer = [0; 64 * 1024];
        loop {
            let count = input
                .read(&mut buffer)
                .map_err(|e| failure("plugin_file_unavailable", e))?;
            if count == 0 {
                break;
            }
            output
                .write_all(&buffer[..count])
                .map_err(|e| failure("fixture_store_unavailable", e))?;
            digest.update(&buffer[..count]);
        }
        output
            .sync_all()
            .map_err(|e| failure("fixture_store_unavailable", e))?;
        let sha256 = format!("{:x}", digest.finalize());
        let copy = bins.join(&sha256);
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o500))
            .map_err(|e| failure("fixture_store_unavailable", e))?;
        fs::rename(&temporary, &copy).map_err(|e| failure("fixture_store_unavailable", e))?;
        let data_dir = root.path().join("plugins/data").join(&sha256);
        fs::create_dir_all(&data_dir).map_err(|e| failure("fixture_store_unavailable", e))?;
        let record = TrustRecord {
            sha256,
            source: source.to_string_lossy().into_owned(),
            copy: copy.to_string_lossy().into_owned(),
            data_dir: data_dir.to_string_lossy().into_owned(),
            name,
            scope: "isolated-conformance-fixture".into(),
            approved_tier: Tier::Transport,
            grants: Default::default(),
            shape: None,
        };
        let store = SqliteStore::open(&root.path().join("account"))
            .map_err(|e| failure("fixture_store_unavailable", e))?;
        let result = Self {
            _root: root,
            store,
            record,
            shape_runs: 0,
        };
        // This is before --shape; a repository-side JSON file is never consulted.
        result.persist()?;
        result.executable()?;
        Ok(result)
    }

    fn key(&self) -> String {
        format!("plugins/trust/{}", self.record.sha256)
    }

    fn persist(&self) -> Result<(), Refusal> {
        let bytes =
            serde_json::to_vec(&self.record).map_err(|e| failure("invalid_trust_record", e))?;
        self.store
            .put(&self.key(), &bytes)
            .map_err(|e| failure("fixture_store_unavailable", e))
    }

    /// Read back account authority and hash the pinned copy before each spawn.
    pub fn executable(&self) -> Result<PathBuf, Refusal> {
        let bytes = self
            .store
            .get(&self.key())
            .map_err(|e| failure("fixture_store_unavailable", e))?
            .ok_or_else(|| {
                failure(
                    "untrusted_plugin",
                    "no operator account trust record for this digest",
                )
            })?;
        let stored: TrustRecord =
            serde_json::from_slice(&bytes).map_err(|e| failure("invalid_trust_record", e))?;
        if stored.sha256 != self.record.sha256
            || stored.copy != self.record.copy
            || stored.data_dir != self.record.data_dir
        {
            return Err(failure(
                "invalid_trust_record",
                "pinned location differs from the fixture account",
            ));
        }
        let copy = PathBuf::from(&stored.copy);
        let expected = self._root.path().join("plugins/bin").join(&stored.sha256);
        if copy != expected || hash_file(&copy)? != stored.sha256 {
            return Err(failure(
                "plugin_digest_changed",
                "the pinned file changed; new operator trust is required",
            ));
        }
        let mode = open_regular(&copy)?
            .metadata()
            .map_err(|e| failure("plugin_file_unavailable", e))?
            .permissions()
            .mode();
        if mode & 0o222 != 0 {
            return Err(failure(
                "plugin_copy_writable",
                "the pinned copy must be read-only",
            ));
        }
        Ok(copy)
    }

    pub fn cache_shape(&mut self, shape: Shape) -> Result<(), Refusal> {
        self.record.validate_shape(&shape)?;
        self.record.shape = Some(shape);
        self.persist()
    }

    pub fn restore_shape(&mut self) -> Result<bool, Refusal> {
        self.executable()?;
        let bytes = self
            .store
            .get(&self.key())
            .map_err(|e| failure("fixture_store_unavailable", e))?
            .ok_or_else(|| failure("untrusted_plugin", "missing account trust record"))?;
        let record: TrustRecord =
            serde_json::from_slice(&bytes).map_err(|e| failure("invalid_trust_record", e))?;
        if let Some(shape) = record.shape {
            self.record.validate_shape(&shape)?;
            self.record.shape = Some(shape);
            return Ok(true);
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_readonly_copy_and_account_record_are_required_before_execution() {
        let files = tempfile::tempdir().unwrap();
        let source = files.path().join("bboxy450-term-data_source-reference");
        fs::write(&source, b"#!/bin/sh\nexit 0\n").unwrap();
        fs::set_permissions(&source, fs::Permissions::from_mode(0o700)).unwrap();
        let fixture = FixtureAccount::prepare(&source).unwrap();
        let copy = fixture.executable().unwrap();
        assert_ne!(copy, source);
        assert_eq!(
            copy.file_name().unwrap().to_str().unwrap(),
            fixture.record.sha256
        );
        assert_eq!(
            fs::metadata(&copy).unwrap().permissions().mode() & 0o777,
            0o500
        );
        fs::write(&source, b"#!/bin/sh\nexit 3\n").unwrap();
        assert!(
            fixture.executable().is_ok(),
            "changes to discovery path do not change the pinned bytes"
        );
        fs::set_permissions(&copy, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            fixture.executable().unwrap_err().code,
            "plugin_copy_writable"
        );
        fs::write(&copy, b"changed").unwrap();
        fs::set_permissions(&copy, fs::Permissions::from_mode(0o500)).unwrap();
        assert_eq!(
            fixture.executable().unwrap_err().code,
            "plugin_digest_changed"
        );
        fixture
            .store
            .sql("DELETE FROM kv WHERE key = ?1", &[fixture.key().into()])
            .unwrap();
        assert_eq!(fixture.executable().unwrap_err().code, "untrusted_plugin");
    }
}
