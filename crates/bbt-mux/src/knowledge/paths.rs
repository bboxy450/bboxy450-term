// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Pure write admission; resolving existing records does not rename their paths.
use super::data_source as d;
use std::path::{Component, Path};

pub(crate) fn validate(path: &str, kind: &str) -> d::Result<()> {
    let relative = Path::new(path);
    if path.len() > 4096
        || relative.is_absolute()
        || relative.extension().is_none_or(|ext| ext != "md")
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(d::CommandError::new(
            "invalid_path",
            "Use a relative Markdown path inside the knowledge root.",
        ));
    }
    let prefix = path.split_once('/').map(|(prefix, _)| prefix);
    // Directory names organize records, never authorize their frontmatter type.
    // Custom Oracle-defined types may use their own singular/plural namespace.
    let typed = prefix.is_some_and(|prefix| {
        matches!(
            prefix,
            "artifacts"
                | "bugs"
                | "comments"
                | "commands"
                | "config"
                | "context"
                | "decisions"
                | "e2e"
                | "epics"
                | "interfaces"
                | "invariants"
                | "notes"
                | "plans"
                | "reports"
                | "research"
                | "specs"
                | "tasks"
        ) || (!kind.is_empty()
            && (prefix == kind
                || prefix.strip_suffix('s') == Some(kind)
                || kind
                    .strip_suffix('y')
                    .is_some_and(|stem| prefix.strip_suffix("ies") == Some(stem))))
    });
    if !typed {
        return Err(d::CommandError::new(
            "document_type_prefix_required",
            "Use a document-type prefix, such as tasks/<slug>.md, specs/<slug>.md or research/<slug>.md.",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn document_type_prefix_is_required_for_every_write() {
        for path in ["a.md", "tasks.md", "misc/a.md"] {
            assert_eq!(
                validate(path, "note").unwrap_err().code,
                "document_type_prefix_required",
                "{path}"
            );
        }
        for (path, kind) in [
            ("tasks/a.md", "task"),
            ("specs/auth/a.md", "spec"),
            ("research/a.md", "research"),
            ("e2e/a.md", "e2e"),
            ("policies/a.md", "policy"),
            ("policy/a.md", "policy"),
        ] {
            assert!(validate(path, kind).is_ok(), "{path}");
        }
        for path in ["/tasks/a.md", "tasks/../a.md", "./a.md", "tasks/a.txt"] {
            assert_eq!(
                validate(path, "task").unwrap_err().code,
                "invalid_path",
                "{path}"
            );
        }
    }
}
