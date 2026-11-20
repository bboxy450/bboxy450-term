// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Retained stroke records. Coordinates stay in the path's local logical space.
use super::{Paint, Pt};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    Move(Pt),
    Line(Pt),
    Cubic([Pt; 3]),
    /// Close to the current subpath start, preserving a stroke join rather than caps.
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FillRule {
    #[default]
    NonZero,
    EvenOdd,
}

/// SVG/CSS affine order: [a, b, c, d, e, f] maps (x,y) to
/// (a*x+c*y+e, b*x+d*y+f). Translation is never rounded here.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Affine(pub [f32; 6]);
impl Default for Affine {
    fn default() -> Self {
        Self([1., 0., 0., 1., 0., 0.])
    }
}
impl Affine {
    pub fn validate(self) -> Result<(), &'static str> {
        if self.0.iter().all(|n| n.is_finite()) {
            Ok(())
        } else {
            Err("nonfinite affine transform")
        }
    }
    pub fn map(self, p: Pt) -> Pt {
        let [a, b, c, d, e, f] = self.0;
        [a * p[0] + c * p[1] + e, b * p[0] + d * p[1] + f]
    }
    /// Compose `self * local`: the local transform acts first.
    pub fn concat(self, local: Self) -> Self {
        let [a, b, c, d, e, f] = self.0;
        let [g, h, i, j, k, l] = local.0;
        Self([
            a * g + c * h,
            b * g + d * h,
            a * i + c * j,
            b * i + d * j,
            a * k + c * l + e,
            b * k + d * l + f,
        ])
    }
    pub fn command(self, command: PathCommand) -> PathCommand {
        match command {
            PathCommand::Move(p) => PathCommand::Move(self.map(p)),
            PathCommand::Line(p) => PathCommand::Line(self.map(p)),
            PathCommand::Cubic(p) => PathCommand::Cubic(p.map(|p| self.map(p))),
            PathCommand::Close => PathCommand::Close,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    #[default]
    Butt,
    Round,
    Square,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    #[default]
    Miter,
    Round,
    Bevel,
}
#[derive(Debug, Clone, PartialEq)]
pub struct PathStroke {
    pub width: f32,
    pub dash: Arc<[f32]>,
    pub cap: LineCap,
    pub join: LineJoin,
}
impl Default for PathStroke {
    fn default() -> Self {
        Self {
            width: 1.,
            dash: Arc::from([]),
            cap: LineCap::Butt,
            join: LineJoin::Miter,
        }
    }
}
impl PathStroke {
    pub fn validate(&self, points: &[PathCommand]) -> Result<(), &'static str> {
        if !self.width.is_finite()
            || self.width < 0.
            || self.dash.len() > 256
            || self.dash.iter().any(|n| !n.is_finite() || *n < 0.)
            || points.len() > 8192
        {
            return Err("invalid path stroke");
        }
        let mut started = false;
        for command in points {
            let coords: &[_] = match command {
                PathCommand::Move(p) => {
                    started = true;
                    std::slice::from_ref(p)
                }
                PathCommand::Line(p) => std::slice::from_ref(p),
                PathCommand::Cubic(p) => p,
                PathCommand::Close => &[],
            };
            if !started || coords.iter().flatten().any(|n| !n.is_finite()) {
                return Err("invalid path coordinates");
            }
        }
        Ok(())
    }
}

/// Complete retained path paint for affine vector consumers. Coverage shares
/// the existing shadow/mask primitive; legacy Prim::Path painting is preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorPath {
    pub points: Arc<[PathCommand]>,
    pub transform: Affine,
    pub fill: Option<Paint>,
    pub fill_rule: FillRule,
    pub fill_opacity: f32,
    pub stroke: PathStroke,
    pub stroke_paint: Option<Paint>,
    pub stroke_opacity: f32,
    pub miter_limit: f32,
    pub dash_offset: f32,
}
impl VectorPath {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.stroke.validate(&self.points)?;
        self.transform.validate()?;
        for paint in self.fill.iter().chain(self.stroke_paint.iter()) {
            paint.validate()?;
        }
        if [self.fill_opacity, self.stroke_opacity]
            .iter()
            .any(|n| !n.is_finite() || !(0. ..=1.).contains(n))
            || !self.miter_limit.is_finite()
            || self.miter_limit < 1.
            || !self.dash_offset.is_finite()
        {
            return Err("invalid vector paint");
        }
        Ok(())
    }
}

/// Centreline bounds and segment endpoints, independently of stroke rasterisation.
pub fn path_geometry(points: &[PathCommand]) -> ([f32; 4], Vec<Pt>) {
    let mut lo = [f32::INFINITY; 2];
    let mut hi = [f32::NEG_INFINITY; 2];
    let mut endpoints = vec![];
    let mut at = [0.; 2];
    let mut start = at;
    let mut include = |p: Pt| {
        for j in 0..2 {
            lo[j] = lo[j].min(p[j]);
            hi[j] = hi[j].max(p[j]);
        }
    };
    for p in points {
        match *p {
            PathCommand::Move(p) => {
                include(p);
                endpoints.push(p);
                at = p;
                start = p;
            }
            PathCommand::Close => {
                endpoints.push(start);
                at = start;
            }
            PathCommand::Line(p) => {
                include(p);
                endpoints.push(p);
                at = p;
            }
            PathCommand::Cubic(c) => {
                for j in 0..2 {
                    let [p0, p1, p2, p3] = [at[j], c[0][j], c[1][j], c[2][j]].map(f64::from);
                    let a = -p0 + 3. * p1 - 3. * p2 + p3;
                    let b = 2. * (p0 - 2. * p1 + p2);
                    let d = p1 - p0;
                    let disc = b * b - 4. * a * d;
                    let roots = if a.abs() < 1e-12 {
                        [if b.abs() > 1e-12 { -d / b } else { -1. }, -1.]
                    } else if disc >= 0. {
                        [(-b + disc.sqrt()) / (2. * a), (-b - disc.sqrt()) / (2. * a)]
                    } else {
                        [-1.; 2]
                    };
                    for t in roots {
                        if t > 0. && t < 1. {
                            let u = 1. - t;
                            include(std::array::from_fn(|k| {
                                (u * u * u * f64::from(at[k])
                                    + 3. * u * u * t * f64::from(c[0][k])
                                    + 3. * u * t * t * f64::from(c[1][k])
                                    + t * t * t * f64::from(c[2][k]))
                                    as f32
                            }));
                        }
                    }
                }
                include(c[2]);
                endpoints.push(c[2]);
                at = c[2];
            }
        }
    }
    if endpoints.is_empty() {
        ([0.; 4], endpoints)
    } else {
        ([lo[0], lo[1], hi[0] - lo[0], hi[1] - lo[1]], endpoints)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // keeps: spec svg-renderer — closure retains each subpath and current point
    #[test]
    fn close_returns_to_each_subpath_start_before_following_curves() {
        let commands = [
            PathCommand::Move([2., 3.]),
            PathCommand::Line([12., 3.]),
            PathCommand::Close,
            PathCommand::Cubic([[2., 7.], [6., 7.], [6., 3.]]),
            PathCommand::Move([20., 4.]),
            PathCommand::Line([24., 4.]),
            PathCommand::Close,
        ];
        assert!(PathStroke::default().validate(&commands).is_ok());
        assert!(
            PathStroke::default()
                .validate(&[PathCommand::Close])
                .is_err()
        );
        let (bounds, endpoints) = path_geometry(&commands);
        assert_eq!(bounds, [2., 3., 22., 3.]);
        assert_eq!(
            endpoints,
            [
                [2., 3.],
                [12., 3.],
                [2., 3.],
                [6., 3.],
                [20., 4.],
                [24., 4.],
                [20., 4.]
            ]
        );
    }
    // keeps: spec renderer §4.4a — exact vector bounds and endpoints
    #[test]
    fn cubic_bounds_include_interior_extrema_not_control_hull() {
        let (bounds, ends) = path_geometry(&[
            PathCommand::Move([0., 0.]),
            PathCommand::Cubic([[0., 10.], [10., 10.], [10., 0.]]),
        ]);
        assert_eq!(bounds, [0., 0., 10., 7.5]);
        assert_eq!(ends, [[0., 0.], [10., 0.]]);
    }
}
