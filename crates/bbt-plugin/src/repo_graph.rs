// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed requests and results of the `repo_graph` 1.0 contract
//! (`schemas/repo_graph-1.0.schema.json`): the repositories under a root and
//! how they relate. `describe` takes the host-owned `root`, so it is never a
//! seat tool.
use serde::{Deserialize, Serialize};

pub use crate::data_source::ChangesRequest;

/// `repo_graph` 1.0 method names.
pub const METHODS: [&str; 2] = ["describe", "changes_since"];

/// `describe` the repositories under the host-chosen `root`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribeRequest {
    pub root: String,
}

/// The build or source system a repository belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoKind {
    Cargo,
    Npm,
    Nix,
    Git,
    Other,
}

/// One repository or package.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repo {
    pub path: String,
    pub kind: RepoKind,
    pub name: String,
}

/// How two repositories relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Rel {
    Depends,
    Contains,
    Workspace,
}

/// A directed edge between two repository `name`s or paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub rel: Rel,
}

/// The whole graph under the root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribeResult {
    pub repos: Vec<Repo>,
    pub edges: Vec<Edge>,
}

/// `changes_since` result: repository paths changed after the cursor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Changes {
    pub changed: Vec<String>,
    pub next: String,
}
