// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{Diagnostic, Result};
use bbt_ui_model::draw::{PathCommand, PathStroke};
use std::sync::Arc;

/// The existing SVG token parser expands relative, smooth and elliptical forms;
/// this module only lowers them to the shared retained drawing commands.
pub fn parse_path(data: &str) -> Result<Arc<[PathCommand]>> {
    use svgtypes::SimplePathSegment as S;
    let mut commands = Vec::new();
    let mut at = [0_f64; 2];
    let mut start = at;
    let point = |x: f64, y: f64| -> Result<[f32; 2]> {
        let p = [x as f32, y as f32];
        if p.iter().any(|n| !n.is_finite()) {
            return Err(Diagnostic::refused(
                "nonfinite",
                "path coordinate overflows",
            ));
        }
        Ok(p)
    };
    for segment in svgtypes::SimplifyingPathParser::from(data) {
        if commands.len() == 8192 {
            return Err(Diagnostic::refused(
                "path_commands",
                "path exceeds 8192 lowered commands",
            ));
        }
        let segment = segment.map_err(|e| Diagnostic::refused("path_syntax", e.to_string()))?;
        commands.push(match segment {
            S::MoveTo { x, y } => {
                at = [x, y];
                start = at;
                PathCommand::Move(point(x, y)?)
            }
            S::LineTo { x, y } => {
                at = [x, y];
                PathCommand::Line(point(x, y)?)
            }
            S::CurveTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                at = [x, y];
                PathCommand::Cubic([point(x1, y1)?, point(x2, y2)?, point(x, y)?])
            }
            S::Quadratic { x1, y1, x, y } => {
                let c = [
                    [
                        at[0] + (x1 - at[0]) * 2. / 3.,
                        at[1] + (y1 - at[1]) * 2. / 3.,
                    ],
                    [x + (x1 - x) * 2. / 3., y + (y1 - y) * 2. / 3.],
                    [x, y],
                ];
                at = [x, y];
                PathCommand::Cubic([
                    point(c[0][0], c[0][1])?,
                    point(c[1][0], c[1][1])?,
                    point(x, y)?,
                ])
            }
            S::ClosePath => {
                at = start;
                PathCommand::Close
            }
        });
    }
    PathStroke::default()
        .validate(&commands)
        .map_err(|e| Diagnostic::refused("path_coordinates", e))?;
    Ok(commands.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec svg-renderer — relative commands, close, curves and finite limits.
    #[test]
    fn curves_and_close_keep_subpath_endpoints() {
        let p = parse_path("M2 3h10v4z m20 0q3 6 6 0t6 0a4 4 0 0 1 4 4").unwrap();
        assert!(matches!(p[3], PathCommand::Close));
        assert!(matches!(p[4], PathCommand::Move([22., 3.])));
        let (_, ends) = bbt_ui_model::draw::path_geometry(&p);
        assert_eq!(*ends.last().unwrap(), [38., 7.]);
        assert!(parse_path("M0 0L1e100 0").is_err());
        assert!(parse_path("M0 0Q1 2").is_err());
    }
    // keeps: spec svg-renderer — command expansion is charged before publication.
    #[test]
    fn path_command_boundary_is_enforced_after_lowering() {
        let accepted = format!("M0 0{}", "L1 1".repeat(8191));
        assert_eq!(parse_path(&accepted).unwrap().len(), 8192);
        assert_eq!(
            parse_path(&(accepted + "L2 2")).unwrap_err().code,
            "path_commands"
        );
    }
}
