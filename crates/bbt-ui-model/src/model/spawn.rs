//! Resolved launcher fields and edit state; hosts supply defaults and perform launches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKey {
    Role,
    Directory,
    Command,
    Arguments,
    Environment,
}
impl FieldKey {
    pub fn label(self) -> &'static str {
        match self {
            Self::Role => "Role",
            Self::Directory => "Directory",
            Self::Command => "Command",
            Self::Arguments => "Arguments",
            Self::Environment => "Environment",
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
pub struct SpawnField {
    pub key: FieldKey,
    pub value: String,
    pub source: Source,
    /// None means inline text; Some means a value list.
    pub choices: Option<Vec<ValueChoice>>,
    pub fixed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnParams {
    pub launch: String,
    pub fields: Vec<SpawnField>,
    pub editing: Option<usize>,
    pub choosing: Option<usize>,
}
