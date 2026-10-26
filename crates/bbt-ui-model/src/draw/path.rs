//! Retained stroke records. Coordinates stay in the path's local logical space.
use super::Pt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    Move(Pt),
    Line(Pt),
    Cubic([Pt; 3]),
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
            };
            if !started || coords.iter().flatten().any(|n| !n.is_finite()) {
                return Err("invalid path coordinates");
            }
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
    let mut include = |p: Pt| {
        for j in 0..2 {
            lo[j] = lo[j].min(p[j]);
            hi[j] = hi[j].max(p[j]);
        }
    };
    for p in points {
        match *p {
            PathCommand::Move(p) | PathCommand::Line(p) => {
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
