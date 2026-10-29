// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Additive descriptors, strict version majors and deterministic public names.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

use crate::{
    Refusal,
    contracts::Contract,
    policy::{Hook, Tier},
    skills::{self, Skill},
};

/// The current descriptor ABI; minor versions may add unknown fields.
pub const SHAPE_VERSION: &str = "1.0";
/// The bounded name understood by both harness tool catalogues.
pub const MAX_TOOL_NAME_BYTES: usize = 64;
/// JSON Schema for the descriptor, independent of a host validator library.
pub const SCHEMA: &str = include_str!("../schemas/shape-1.0.schema.json");

/// MAJOR.MINOR is deliberate: a patch semver string is not this wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}
impl Version {
    /// Parse canonical decimal components, refusing unbounded or partial values.
    pub fn parse(value: &str) -> Result<Self, Refusal> {
        let invalid = || {
            Refusal::new(
                "invalid_version",
                format!("expected MAJOR.MINOR, got {value}"),
            )
        };
        let (major, minor) = value.split_once('.').ok_or_else(invalid)?;
        let number = |part: &str| {
            if part.is_empty()
                || !part.bytes().all(|b| b.is_ascii_digit())
                || (part.len() > 1 && part.starts_with('0'))
            {
                return Err(invalid());
            }
            part.parse().map_err(|_| invalid())
        };
        Ok(Self {
            major: number(major)?,
            minor: number(minor)?,
        })
    }

    /// New minor fields are additive; incompatible majors are refused by name.
    pub fn compatible(offered: &str, supported: &str) -> Result<(), Refusal> {
        let offered_version = Self::parse(offered)?;
        let supported_version = Self::parse(supported)?;
        if offered_version.major != supported_version.major {
            return Err(Refusal::new(
                "major_mismatch",
                format!("offered {offered}, supported {supported}"),
            ));
        }
        Ok(())
    }
}

/// One app-scoped executable name; content-addressed copies retain this data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginName {
    pub system: Contract,
    pub provider: String,
}
impl PluginName {
    /// Strip the exact app prefix, then match the longest known system token.
    pub fn parse(app: &str, file_name: &str) -> Result<Self, Refusal> {
        let rest = file_name
            .strip_prefix(app)
            .and_then(|s| s.strip_prefix('-'))
            .ok_or_else(|| {
                Refusal::new(
                    "invalid_plugin_name",
                    "executable has a different app prefix",
                )
            })?;
        let matched = Contract::ALL
            .into_iter()
            .filter_map(|system| {
                rest.strip_prefix(system.name())
                    .and_then(|s| s.strip_prefix('-'))
                    .map(|provider| (system, provider))
            })
            .max_by_key(|(system, _)| system.name().len());
        let (system, provider) =
            matched.ok_or_else(|| Refusal::new("invalid_plugin_name", "unknown system token"))?;
        if provider.is_empty()
            || !provider
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        {
            return Err(Refusal::new(
                "invalid_plugin_name",
                "provider must match [a-z0-9-]+",
            ));
        }
        Ok(Self {
            system,
            provider: provider.to_owned(),
        })
    }

    /// Preserve provider hyphens; never silently shorten a colliding name.
    pub fn tool(&self, contract: Contract, method: &str) -> Result<String, Refusal> {
        contract.role(method)?;
        let name = format!("plugin_{}_{}_{method}", contract.name(), self.provider);
        if name.len() > MAX_TOOL_NAME_BYTES {
            return Err(Refusal::new(
                "tool_name_too_long",
                format!("{name} exceeds {MAX_TOOL_NAME_BYTES} bytes"),
            ));
        }
        if !name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
        {
            return Err(Refusal::new(
                "invalid_tool_name",
                "tool names must match [a-z0-9_-]+",
            ));
        }
        Ok(name)
    }
}

/// Tool input/output schemas are advertised by the provider, never authority.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Command {
    pub name: String,
    pub args: Value,
    pub result: Value,
    pub timeout_ms: u64,
}
/// A descriptor can fulfil several contracts on the same MCP connection.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct System {
    pub contract: String,
    pub contract_version: String,
    pub commands: Vec<Command>,
}
/// Setup remains the later registry/login cut; a checker never calls login.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Auth {
    pub status_command: String,
    pub login_command: String,
}

/// Unknown fields are intentionally ignored by the plugin boundary DTOs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape {
    /// Human-facing name; executable and tool identity come from PluginName.
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
    /// Presentation metadata only. The host never executes or trusts it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identity: Option<Value>,
    /// Opt-in to the host-signed data_source write extension.
    #[serde(default)]
    pub signed_writer: bool,
    /// Checker-only bootstrap of a host-owned disposable data_source fixture.
    #[serde(default)]
    pub fixture_bootstrap: bool,
    pub version: String,
    pub shape_version: String,
    pub systems: Vec<System>,
    /// Optional instructions, cached with the trusted descriptor's binary digest.
    #[serde(default)]
    pub skills: Vec<Skill>,
    #[serde(default)]
    pub hooks: Vec<Hook>,
    #[serde(default)]
    pub auth: Option<Auth>,
    pub health_command: String,
    #[serde(default)]
    pub concurrent: bool,
    #[serde(default)]
    pub trace: bool,
}

impl Shape {
    /// Validate fixed contract identity and policy before any MCP method runs.
    pub fn validate(&self, name: &PluginName) -> Result<Tier, Refusal> {
        Version::compatible(&self.shape_version, SHAPE_VERSION)?;
        if self.name.len() > 512
            || self.description.len() > 4096
            || self.icon.as_ref().is_some_and(|icon| icon.len() > 4096)
            || self
                .identity
                .as_ref()
                .is_some_and(|identity| !identity.is_object() || identity.to_string().len() > 4096)
        {
            return Err(Refusal::new(
                "invalid_shape",
                "presentation metadata exceeds its bounds",
            ));
        }
        skills::validate(&self.skills)?;
        if self.name.is_empty()
            || self.version.is_empty()
            || self.systems.is_empty()
            || self.health_command.is_empty()
        {
            return Err(Refusal::new(
                "invalid_shape",
                "name, version, systems and health_command are required",
            ));
        }
        let mut contracts = BTreeSet::new();
        let mut tier = Tier::Read;
        for system in &self.systems {
            let contract = Contract::named(&system.contract)?;
            Version::compatible(&system.contract_version, contract.version())?;
            if !contracts.insert(contract) {
                return Err(Refusal::new("duplicate_contract", &system.contract));
            }
            tier = tier.max(contract.tier());
            let mut methods = BTreeSet::new();
            for command in &system.commands {
                name.tool(contract, &command.name)?;
                if !methods.insert(command.name.as_str()) {
                    return Err(Refusal::new("duplicate_method", &command.name));
                }
                if command.timeout_ms == 0 {
                    return Err(Refusal::new("invalid_timeout", &command.name));
                }
            }
            for method in contract.methods() {
                if !methods.contains(method) {
                    return Err(Refusal::new(
                        "missing_method",
                        format!("{}_{method}", contract.name()),
                    ));
                }
            }
        }
        if !contracts.contains(&name.system) {
            return Err(Refusal::new(
                "filename_contract_mismatch",
                "the named system is absent from the descriptor",
            ));
        }
        for hook in &self.hooks {
            hook.validate()?;
            if hook.carries_payload()? {
                tier = tier.max(Tier::Export);
            }
        }
        Ok(tier)
    }
}
