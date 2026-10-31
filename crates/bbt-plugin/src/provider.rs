// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Typed requests and results of the describe-only `provider` 0.1 contract
//! (`schemas/provider-0.1.schema.json`): hosts (SSH or cloud) a plugin can
//! offer for seats.
use serde::{Deserialize, Serialize};

/// `provider` 0.1 method names.
pub const METHODS: [&str; 1] = ["describe"];

/// `describe` takes no arguments (unknown fields are ignored).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribeRequest {}

/// How a host is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostKind {
    Ssh,
    Cloud,
}

/// One host.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub kind: HostKind,
    pub address: String,
}

/// Every host the plugin can describe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DescribeResult {
    pub hosts: Vec<Host>,
}
