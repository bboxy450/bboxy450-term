//! Portable provider contracts. This kit owns no process, filesystem, UI or
//! application wire type. The host owns execution, authority and schema checking.
pub mod contracts;
pub mod data_source;
pub mod policy;
pub mod registry;
pub mod shape;
pub mod trust;

/// A stable, named refusal suitable for a checker or registry entry.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Refusal {
    pub code: String,
    pub message: String,
}

impl Refusal {
    /// Keep the machine-readable reason separate from its explanation.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Refusal {}
