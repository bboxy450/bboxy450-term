// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Example `repo_graph` plugin: the root and its direct subdirectories that
//! are Cargo, npm, Nix or Git projects, each contained by the root. A meta
//! repo or monorepo tool would describe its own graph the same way.
#![allow(clippy::disallowed_methods)] // A plugin process: no UI thread.

use std::path::Path;

use bbt_plugin::repo_graph as rg;
use bbt_plugin_sdk::contracts::{AsRepoGraph, Base, RepoGraph};
use bbt_plugin_sdk::{Ctx, Init, Manifest, Refusal, run};

struct Scan;

fn kind(dir: &Path) -> Option<rg::RepoKind> {
    [
        ("Cargo.toml", rg::RepoKind::Cargo),
        ("package.json", rg::RepoKind::Npm),
        ("flake.nix", rg::RepoKind::Nix),
        (".git", rg::RepoKind::Git),
    ]
    .into_iter()
    .find(|(marker, _)| dir.join(marker).exists())
    .map(|(_, kind)| kind)
}

fn name(dir: &Path) -> String {
    dir.file_name()
        .map_or_else(|| dir.to_string_lossy(), |n| n.to_string_lossy())
        .into_owned()
}

impl Base for Scan {
    fn manifest() -> Manifest {
        Manifest::new(
            "bboxy450-term-repo_graph-example",
            "Example project scan",
            "The root's direct subdirectories that are projects",
        )
    }
    fn initialize(_init: Init, _ctx: &Ctx) -> Result<Self, Refusal> {
        Ok(Self)
    }
}

impl RepoGraph for Scan {
    fn describe(
        &mut self,
        req: rg::DescribeRequest,
        ctx: &Ctx,
    ) -> Result<rg::DescribeResult, Refusal> {
        let root = Path::new(&req.root);
        if !root.is_absolute() || !root.is_dir() {
            return Err(Refusal::new(
                "invalid_root",
                "root must be an absolute directory",
            ));
        }
        let mut repos = vec![rg::Repo {
            path: req.root.clone(),
            kind: kind(root).unwrap_or(rg::RepoKind::Other),
            name: name(root),
        }];
        let mut edges = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(root)
            .map_err(|e| Refusal::new("invalid_root", e.to_string()))?
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| path.is_dir())
            .collect();
        entries.sort();
        for dir in entries {
            ctx.check()?;
            if let Some(kind) = kind(&dir) {
                edges.push(rg::Edge {
                    from: repos[0].name.clone(),
                    to: name(&dir),
                    rel: rg::Rel::Contains,
                });
                repos.push(rg::Repo {
                    path: dir.to_string_lossy().into_owned(),
                    kind,
                    name: name(&dir),
                });
            }
        }
        Ok(rg::DescribeResult { repos, edges })
    }
    // A scan has no change log: every poll says nothing changed since.
    fn changes_since(
        &mut self,
        req: rg::ChangesRequest,
        _ctx: &Ctx,
    ) -> Result<rg::Changes, Refusal> {
        Ok(rg::Changes {
            changed: Vec::new(),
            next: req.cursor,
        })
    }
}

fn main() {
    run::<AsRepoGraph<Scan>>();
}
