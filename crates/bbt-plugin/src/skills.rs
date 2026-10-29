// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Optional instruction metadata carried by a trusted plugin descriptor.
//!
//! Content preserves the exact UTF-8 bytes of its canonical SKILL.md. This is
//! a bounded metadata envelope, not an Agent Skills parser or an upstream MCP
//! skills protocol. Instructions never grant authority or change a contract.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::Refusal;

/// Maximum instruction entries carried by one descriptor.
pub const MAX_SKILLS: usize = 16;
/// Maximum bytes in a lowercase ASCII instruction name.
pub const MAX_NAME_BYTES: usize = 64;
/// Maximum UTF-8 bytes in a nonblank discovery description.
pub const MAX_DESCRIPTION_BYTES: usize = 1024;
/// Maximum UTF-8 bytes in one complete instruction document.
pub const MAX_CONTENT_BYTES: usize = 64 * 1024;
/// Maximum serialized JSON bytes in the complete instruction array.
pub const MAX_SKILLS_BYTES: usize = 256 * 1024;

/// Discoverable plugin instructions; unknown fields remain additive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Skill {
    /// Lowercase ASCII alphanumeric words separated by single hyphens.
    pub name: String,
    /// Nonblank discovery text, preserved without trimming.
    pub description: String,
    /// Exact canonical SKILL.md bytes, including frontmatter and whitespace.
    pub content: String,
}

/// Validate names, text and the complete serialized envelope without rewriting it.
pub fn validate(skills: &[Skill]) -> Result<(), Refusal> {
    if skills.len() > MAX_SKILLS {
        return Err(Refusal::new(
            "skills_too_large",
            "at most 16 skills are allowed",
        ));
    }
    let mut names = BTreeSet::new();
    for skill in skills {
        if skill.name.is_empty()
            || skill.name.len() > MAX_NAME_BYTES
            || skill.name.starts_with('-')
            || skill.name.ends_with('-')
            || skill.name.contains("--")
            || !skill
                .name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return Err(Refusal::new(
                "invalid_skill",
                "skill names must be 1..64 lowercase ASCII alphanumeric bytes with single internal hyphens",
            ));
        }
        if !names.insert(skill.name.as_str()) {
            return Err(Refusal::new("duplicate_skill", &skill.name));
        }
        if skill.description.trim().is_empty()
            || skill.content.trim().is_empty()
            || skill.content.contains('\0')
        {
            return Err(Refusal::new(
                "invalid_skill",
                "skill descriptions and content must be nonblank; content must not contain NUL",
            ));
        }
        if skill.description.len() > MAX_DESCRIPTION_BYTES
            || skill.content.len() > MAX_CONTENT_BYTES
        {
            return Err(Refusal::new(
                "skill_too_large",
                "skill descriptions are limited to 1024 UTF-8 bytes and content to 64 KiB",
            ));
        }
    }
    // The wire budget includes JSON punctuation and escaping, not just text bytes.
    let bytes = serde_json::to_vec(skills)
        .map_err(|error| Refusal::new("invalid_skill", error.to_string()))?;
    if bytes.len() > MAX_SKILLS_BYTES {
        return Err(Refusal::new(
            "skills_too_large",
            "serialized skills exceed 256 KiB",
        ));
    }
    Ok(())
}
