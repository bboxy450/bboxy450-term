// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Resolved launcher fields and edit state; hosts supply defaults and perform launches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKey {
    Role,
    Via,
    Connect,
    Port,
    Key,
    Directory,
    Command,
    Arguments,
    Environment,
    TransportParam(u8),
}
impl FieldKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::Role => "Role",
            Self::Via => "Via",
            Self::Connect => "Connect",
            Self::Port => "Port",
            Self::Key => "Key",
            Self::Directory => "Directory",
            Self::Command => "Command",
            Self::Arguments => "Arguments",
            Self::Environment => "Environment",
            Self::TransportParam(_) => "Transport parameter",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Settings,
    LastChoice,
    BuiltIn,
    ThisLaunch,
}
impl Source {
    pub fn label(self) -> &'static str {
        match self {
            Self::Settings => "Settings",
            Self::LastChoice => "last choice",
            Self::BuiltIn => "built-in",
            Self::ThisLaunch => "this launch",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValueChoice {
    pub value: String,
    pub label: String,
    pub purpose: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportParameter {
    pub owner: String,
    pub name: String,
    pub label: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnField {
    pub key: FieldKey,
    pub value: String,
    pub source: Source,
    /// None means inline text; Some means a value list.
    pub choices: Option<Vec<ValueChoice>>,
    pub fixed: bool,
    pub parameter: Option<TransportParameter>,
}

impl SpawnField {
    pub fn label(&self) -> &str {
        self.parameter
            .as_ref()
            .map_or_else(|| self.key.label(), |p| p.label.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnParams {
    pub launch: String,
    pub fields: Vec<SpawnField>,
    pub editing: Option<usize>,
    pub choosing: Option<usize>,
}
