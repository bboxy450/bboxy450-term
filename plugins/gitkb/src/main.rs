// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-term-data_source-gitkb`: the GitKB data_source 1.0 provider
//! (plus explicit `commit`) over the installed, compile-time pinned `git-kb`
//! CLI, built on `bboxy450-term-plugin-sdk`. Translated from bboxy450-term
//! `plugins/gitkb/adapter.py`. Uses (does not copy) GitKB's source-aware CLI
//! contracts, studied in https://github.com/gitkb/gitkb-core :
//! crates/gitkb-cli/src/commands/{show,workspace}.rs and
//! crates/gitkb-core/src/kb/{types,ops_impl}.rs. Native storage remains GitKB's.

// A plugin is its own process with no UI thread; blocking process IO is its job.
#![allow(clippy::disallowed_methods)]

mod backend;
mod methods;
mod native;
mod store;

use std::path::{Path, PathBuf};

use bbt_plugin_sdk::{Contract, Ctx, Init, Manifest, Plugin, Refusal, Skill};
use serde_json::{Value, json};

use backend::Backend;
use store::GitKb;

pub const EXECUTABLE: &str = "bboxy450-term-data_source-gitkb";
const PIN_PATH: &str = env!("BBOXY_GITKB_PIN_PATH");
const PIN_SHA256: &str = env!("BBOXY_GITKB_PIN_SHA256");
const PIN_VERSION: &str = env!("BBOXY_GITKB_PIN_VERSION");
const SKILL: &str = include_str!("../SKILL.md");

/// The compile-time backend pin, as the shape and health report it.
fn pin() -> Value {
    json!({"path": PIN_PATH, "sha256": PIN_SHA256, "version": PIN_VERSION.replace("\\n", "\n")})
}

/// The embedded skill. Its frontmatter must be exactly
/// `---\nname: gitkb\ndescription: <text>\n---\n`.
fn skill() -> Result<Skill, &'static str> {
    let rest = SKILL
        .strip_prefix("---\nname: gitkb\ndescription: ")
        .ok_or("SKILL.md must start with gitkb name frontmatter")?;
    let (description, rest) = rest.split_once('\n').ok_or("unterminated description")?;
    if description.is_empty() || !rest.starts_with("---\n") || SKILL.len() > 64 * 1024 {
        return Err("SKILL.md frontmatter must be name, description, then ---");
    }
    Ok(Skill {
        name: "gitkb".into(),
        description: description.into(),
        content: SKILL.into(),
    })
}

fn invalid_root(message: &str) -> Refusal {
    Refusal::new("invalid_knowledge_root", message.to_owned())
}

impl GitKb {
    fn bind(init: &Init, ctx: &Ctx) -> Result<Self, Refusal> {
        let supplied = init
            .root
            .as_deref()
            .map(Path::new)
            .filter(|p| p.is_absolute())
            .ok_or_else(|| {
                invalid_root("initialize.root must be an absolute project, .kb or .bboxy directory")
            })?;
        if supplied
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink())
        {
            return Err(invalid_root("symlinked roots are refused"));
        }
        let mut root = supplied
            .canonicalize()
            .map_err(|_| invalid_root("initialize.root must name an existing directory"))?;
        if root
            .file_name()
            .is_some_and(|n| n == ".kb" || n == ".bboxy")
        {
            root = root.parent().map(Path::to_path_buf).unwrap_or(root);
        }
        let cwd = std::env::current_dir()
            .and_then(|d| d.canonicalize())
            .map_err(|_| Refusal::new("invalid_arguments", "plugin data directory unavailable"))?;
        let mut backend = Backend::new(PIN_PATH, PIN_SHA256, cwd)?;
        backend.set_env("GITKB_ROOT", &root.display().to_string());
        // Hash the binary once per process; later calls compare file identity.
        backend.verify(ctx)?;
        let mut kb = GitKb::new(backend, ctx.clone(), root.clone(), init.fixture);
        if init.fixture {
            let temporary = std::env::temp_dir()
                .canonicalize()
                .unwrap_or_else(|_| std::env::temp_dir());
            if root.file_name().is_none_or(|n| n != "checker-knowledge")
                || !root.starts_with(&temporary)
                || !root.join(".bboxy/p0-fixture.md").is_file()
            {
                return Err(Refusal::new(
                    "fixture_scope_refused",
                    "bootstrap requires the host's disposable checker-knowledge root",
                ));
            }
            // Approved test-only setup budget: native init measured 6.077 s.
            // Production initialize and every data call retain the 2 s cap.
            if !root.join(".kb").exists() {
                // `git-kb init` explicitly uses cwd (not GITKB_ROOT). This
                // exception is confined to the disposable checker.
                kb.backend.cli_in(
                    ctx,
                    &root,
                    &["init", "--name", "Disposable bboxy plugin conformance"],
                    None,
                    true,
                )?;
            }
        }
        let kb_dir = root.join(".kb");
        if !kb_dir
            .symlink_metadata()
            .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
        {
            return Err(invalid_root(
                "the exact project must own a real .kb directory; no ancestor discovery",
            ));
        }
        let identity = kb.cli(&["workspace", "resolve", "--json"])?;
        let resolved = identity["kbRoot"]
            .as_str()
            .and_then(|p| Path::new(p).canonicalize().ok());
        if resolved.as_deref() != Some(root.as_path()) {
            return Err(Refusal::new(
                "root_mismatch",
                "GitKB resolved a different knowledge root",
            ));
        }
        kb.workspace = identity["workspace"].as_str().unwrap_or("").to_owned();
        kb.workspace_path = PathBuf::from(identity["workspacePath"].as_str().unwrap_or(""));
        if !kb.workspace_path.starts_with(kb_dir.join("workspaces")) || kb.workspace.is_empty() {
            return Err(Refusal::new(
                "workspace_outside_root",
                "GitKB workspace escaped the initialized root",
            ));
        }
        let workspace = kb.workspace.clone();
        kb.backend.set_env("GITKB_WORKSPACE", &workspace);
        if init.fixture {
            kb.bootstrap_fixture()?;
        }
        Ok(kb)
    }

    /// Import the host's canonical conformance seed through `workspace write`.
    fn bootstrap_fixture(&mut self) -> Result<(), Refusal> {
        if self.catalog(&[])?.rows.contains_key("p0-fixture") {
            return Ok(());
        }
        use std::os::unix::fs::MetadataExt;
        let path = self.root.join(".bboxy/p0-fixture.md");
        let unsafe_seed = || {
            Refusal::new(
                "unsafe_fixture",
                "fixture must be one confined regular Markdown seed",
            )
        };
        let info = path.symlink_metadata().map_err(|_| unsafe_seed())?;
        if !info.is_file() || info.nlink() != 1 || info.len() > bbt_plugin_sdk::MAX_RESULT as u64 {
            return Err(unsafe_seed());
        }
        let raw = std::fs::read_to_string(&path).map_err(|_| unsafe_seed())?;
        let (start, end) = seed_id(&raw).ok_or_else(|| {
            Refusal::new(
                "invalid_fixture",
                "expected canonical host conformance seed",
            )
        })?;
        let raw = format!(
            "{}{}{}",
            &raw[..start],
            native::native(&raw[start..end])?,
            &raw[end..]
        );
        let raw = raw.replacen("---\n", "---\nslug: p0-fixture\n", 1);
        self.cli_stdin(
            &[
                "workspace",
                "write",
                "p0-fixture",
                "--stdin",
                "--create-if-missing",
                "--origin-id",
                "bboxy-conformance-seed",
                "--json",
            ],
            &raw,
        )?;
        Ok(())
    }
}

/// Byte range of the ULID in the first `id: <ULID>` line (optionally quoted).
fn seed_id(raw: &str) -> Option<(usize, usize)> {
    let mut start = 0;
    for line in raw.split_inclusive('\n') {
        let text = line.trim_end_matches('\n');
        if let Some(rest) = text.strip_prefix("id:") {
            let value = rest.trim_start_matches(' ');
            let skipped = text.len() - value.len();
            let unquoted = value.trim_start_matches(['"', '\'']);
            let quote = value.len() - unquoted.len();
            let id = unquoted.get(..26)?;
            let tail = unquoted[26..].trim_start_matches(['"', '\'']);
            let tail_ok =
                unquoted.len() - 26 - tail.len() <= 1 && tail.trim_end_matches(' ').is_empty();
            if quote <= 1
                && tail_ok
                && id
                    .bytes()
                    .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
            {
                let at = start + skipped + quote;
                return Some((at, at + 26));
            }
        }
        start += line.len();
    }
    None
}

impl Plugin for GitKb {
    fn manifest() -> Manifest {
        let mut manifest = Manifest::new(
            EXECUTABLE,
            "GitKB",
            "Source-aware local GitKB documents and task storage",
        )
        .contract(Contract::DataSource)
        .method(Contract::DataSource, "commit")
        .timeout(Contract::DataSource, "commit", 15_000)
        .identity("provider", "gitkb")
        .identity("backend", pin())
        .signed_writer()
        .fixture_bootstrap();
        if let Ok(skill) = skill() {
            manifest = manifest.skill(skill);
        }
        manifest
    }

    fn initialize(init: Init, ctx: &Ctx) -> Result<Self, Refusal> {
        Self::bind(&init, ctx)
    }

    fn call(
        &mut self,
        _contract: Contract,
        method: &str,
        args: Value,
        ctx: &Ctx,
    ) -> Result<Value, Refusal> {
        self.ctx = ctx.clone();
        self.call_records.clear();
        match method {
            "resolve" => self.resolve(&args),
            "fetch" => {
                let reference = args
                    .get("ref")
                    .ok_or_else(|| Refusal::new("invalid_arguments", "missing argument ref"))?;
                let record = self.check_ref(reference)?;
                self.resources(record)
            }
            "list" => self.listing(&args, false),
            "search" => self.listing(&args, true),
            "links" => self.links(&args),
            "graph" => self.graph(&args),
            "write" => self.write(&args),
            "changes_since" => self.changes(&args),
            "commit" => self.commit(&args),
            _ => Err(Refusal::new("unknown_method", method.to_owned())),
        }
    }

    fn health(&mut self, _ctx: &Ctx) -> Result<Value, Refusal> {
        Ok(
            json!({"healthy": true, "root": self.root_text, "workspace": self.workspace,
            "backend": pin()}),
        )
    }
}

fn main() {
    bbt_plugin_sdk::run::<GitKb>();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_skill_has_canonical_frontmatter() {
        let skill = skill().unwrap();
        assert_eq!(skill.name, "gitkb");
        assert!(SKILL.contains(&format!("description: {}\n---\n", skill.description)));
        assert_eq!(skill.content, SKILL);
    }

    #[test]
    fn manifest_passes_the_host_validator() {
        let manifest = GitKb::manifest();
        manifest.validate().unwrap();
        let shape = manifest.shape();
        assert_eq!(shape["name"], "GitKB");
        assert_eq!(shape["identity"]["provider"], "gitkb");
        assert_eq!(shape["identity"]["executable"], EXECUTABLE);
        assert_eq!(shape["identity"]["backend"]["sha256"], PIN_SHA256);
        assert_eq!(shape["signed_writer"], true);
        assert_eq!(shape["fixture_bootstrap"], true);
        assert_eq!(shape["skills"].as_array().unwrap().len(), 1);
        let commands = shape["systems"][0]["commands"].as_array().unwrap();
        assert_eq!(commands.len(), 9);
        let commit = commands.iter().find(|c| c["name"] == "commit").unwrap();
        assert_eq!(commit["timeout_ms"], 15_000);
        assert_eq!(
            manifest.skill_uri(&manifest.skills[0]),
            "gitkb://skills/gitkb/SKILL.md"
        );
    }

    #[test]
    fn seed_id_finds_the_ulid_line() {
        let raw = "---\nid: 00000000000000000000000007\ntype: note\n---\n";
        let (s, e) = seed_id(raw).unwrap();
        assert_eq!(&raw[s..e], "00000000000000000000000007");
        let quoted = "---\nid: \"01ARZ3NDEKTSV4RRFFQ69G5FAV\" \n---\n";
        let (s, e) = seed_id(quoted).unwrap();
        assert_eq!(&quoted[s..e], "01ARZ3NDEKTSV4RRFFQ69G5FAV");
        assert!(seed_id("---\nid: 01ARZ3NDEKTSV4RRFFQ69G5FAVX\n---\n").is_none());
        assert!(seed_id("---\nid: 0000000000000000000000000I\n---\n").is_none());
    }
}
