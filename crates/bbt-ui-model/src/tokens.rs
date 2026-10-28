// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Compile-time tokens; no file parsing or third-party crate in the runtime model.

mod generated {
    include!(concat!(env!("OUT_DIR"), "/tokens.rs"));
}
pub use generated::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}

impl Theme {
    pub fn palette(self) -> &'static Palette {
        match self {
            Self::Dark => &TOKENS.color.dark,
            Self::Light => &TOKENS.color.light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Density {
    Touch,
    Comfortable,
    #[default]
    Cozy,
    Compact,
}

impl Density {
    pub fn tokens(self) -> &'static DensityTokens {
        match self {
            Self::Touch => &TOKENS.density.touch,
            Self::Comfortable => &TOKENS.density.comfortable,
            Self::Cozy => &TOKENS.density.cozy,
            Self::Compact => &TOKENS.density.compact,
        }
    }
    pub fn shows_title(self) -> bool {
        self != Self::Compact
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq)]
enum Scalar {
    Bool(bool),
    Number(f64),
    Text(String),
    ArrayLength(usize),
    ObjectKeys(Vec<String>),
}

#[cfg(test)]
trait Visit {
    fn visit(&self, path: &str, out: &mut std::collections::BTreeMap<String, Scalar>);
}
#[cfg(test)]
impl Visit for f64 {
    fn visit(&self, p: &str, o: &mut std::collections::BTreeMap<String, Scalar>) {
        o.insert(p.into(), Scalar::Number(*self));
    }
}
#[cfg(test)]
impl Visit for bool {
    fn visit(&self, p: &str, o: &mut std::collections::BTreeMap<String, Scalar>) {
        o.insert(p.into(), Scalar::Bool(*self));
    }
}
#[cfg(test)]
impl Visit for &str {
    fn visit(&self, p: &str, o: &mut std::collections::BTreeMap<String, Scalar>) {
        o.insert(p.into(), Scalar::Text((*self).into()));
    }
}
#[cfg(test)]
impl<T: Visit> Visit for &[T] {
    fn visit(&self, p: &str, o: &mut std::collections::BTreeMap<String, Scalar>) {
        o.insert(p.into(), Scalar::ArrayLength(self.len()));
        for (i, v) in self.iter().enumerate() {
            v.visit(&format!("{p}[{i}]"), o);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn flatten(v: &serde_json::Value, path: &str, out: &mut BTreeMap<String, Scalar>) {
        match v {
            serde_json::Value::Object(map) => {
                out.insert(
                    path.into(),
                    Scalar::ObjectKeys(map.keys().cloned().collect()),
                );
                for (key, value) in map {
                    flatten(value, &format!("{path}.{key}"), out);
                }
            }
            serde_json::Value::Array(array) => {
                out.insert(path.into(), Scalar::ArrayLength(array.len()));
                for (i, value) in array.iter().enumerate() {
                    flatten(value, &format!("{path}[{i}]"), out);
                }
            }
            serde_json::Value::Bool(b) => {
                out.insert(path.into(), Scalar::Bool(*b));
            }
            serde_json::Value::Number(n) => {
                out.insert(path.into(), Scalar::Number(n.as_f64().unwrap()));
            }
            serde_json::Value::String(s) => {
                out.insert(path.into(), Scalar::Text(s.clone()));
            }
            serde_json::Value::Null => panic!("unexpected null web token"),
        }
    }

    // keeps: spec gui-components §1.5 — generated lab/native token equality and index selection
    #[test]
    fn every_exported_web_token_matches_generated_fields() {
        let js = include_str!("../../../lab/web/tokens.js");
        let mut expected = BTreeMap::new();
        for name in ["TOKENS", "STYLE"] {
            let regex =
                regex::Regex::new(&format!(r"(?s)window\.{name}\s*=\s*(\{{.*?\}});")).unwrap();
            let capture = regex.captures(js).expect("web token export");
            let value: serde_json::Value = serde_json::from_str(&capture[1]).unwrap();
            flatten(&value, name, &mut expected);
        }
        let mut actual = BTreeMap::new();
        TOKENS.visit("TOKENS", &mut actual);
        STYLE.visit("STYLE", &mut actual);
        assert_eq!(
            actual, expected,
            "regenerate the web tokens if design sources changed"
        );
        eprintln!(
            "{} web-exported token paths (object keys, arrays and scalar values) match generated Rust fields",
            actual.len()
        );
    }

    // keeps: spec gui-components §1.5 — generated lab/native token equality and index selection
    #[test]
    fn density_and_theme_are_views_of_the_constants() {
        assert_eq!(Density::Cozy.tokens().row_h, 24.0);
        assert_eq!(Density::Compact.tokens().row_h, 20.0);
        assert_eq!(Density::Touch.tokens().row_h, 44.0);
        assert!(!Density::Compact.shows_title());
        assert_eq!(Theme::Light.palette().bg, TOKENS.color.light.bg);
        assert_ne!(
            Theme::Dark.palette().sidebar,
            Theme::Light.palette().sidebar
        );
    }
}
