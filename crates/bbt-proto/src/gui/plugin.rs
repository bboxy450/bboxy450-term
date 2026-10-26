//! Operator controls for the account-owned production plugin registry.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PluginRequest {
    List,
    /// Only a discovered path is eligible; this never imports an arbitrary executable.
    Trust {
        path: String,
    },
    Enable {
        candidate: String,
    },
    Disable {
        candidate: String,
    },
}

impl PluginRequest {
    pub fn mutation(&self) -> bool {
        !matches!(self, Self::List)
    }
}

macro_rules! plugin_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Plugin { id: Ulid, request: PluginRequest },
        ] $($head)*);
    };
}
pub(super) use plugin_actions;

macro_rules! plugin_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            Plugin { op: Ulid, entries: Vec<bbt_plugin::registry::Entry> },
        ] $($head)*);
    };
}
pub(super) use plugin_replies;
