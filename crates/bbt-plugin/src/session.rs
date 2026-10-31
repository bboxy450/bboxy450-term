// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed requests and results of the describe-only `session` 0.1 contract
//! (`schemas/session-0.1.schema.json`): external harness sessions a plugin
//! knows about.
use serde::{Deserialize, Serialize};

/// `session` 0.1 method names.
pub const METHODS: [&str; 1] = ["describe"];

/// `describe` takes no arguments (unknown fields are ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribeRequest {}

/// One external session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    pub harness: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
}

/// Every session the plugin can describe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribeResult {
    pub sessions: Vec<Session>,
}
