//! Knowledge reads and operator-signed record writes.
use super::*;

/// Root names come from the host; selecting never accepts a caller-supplied path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum KnowledgeRequest {
    Status,
    Reset {
        root: String,
    },
    Select {
        root: String,
        provider: String,
        revision: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        sha256: Option<String>,
    },
}
impl KnowledgeRequest {
    pub fn mutation(&self) -> bool {
        !matches!(self, Self::Status)
    }
}

/// Compact eligible choices, rather than a duplicate of every plugin schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeProvider {
    pub id: String,
    pub name: String,
    pub sha256: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeRoot {
    pub id: String,
    pub path: String,
    pub provider: KnowledgeProvider,
    pub revision: u64,
    pub records: Option<u64>,
    pub process: Option<u32>,
    pub refusal: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KnowledgeStatus {
    pub roots: Vec<KnowledgeRoot>,
    pub offered: Vec<KnowledgeProvider>,
}

/// This noun's GuiAction variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! record_actions {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            KnowledgeControl { id: Ulid, request: KnowledgeRequest },
            /// An operator-signed knowledge write of an `e2e` record (`ctl record write`, e2e
            /// acceptance §2): `args` is the knowledge_write DTO (`path` or `ref`, `frontmatter`,
            /// `body`, `etag`). Logic signs it with the operator device under its issuer and journals
            /// it; the reply is `Knowledge` with the serialized data_source result.
            RecordWrite {
                id: Ulid,
                args: serde_json::Value,
            },
            /// Read-only portable data_source call. `args` is the existing method DTO;
            /// admission is the same operator GUI admission as every other view read.
            Knowledge {
                id: Ulid,
                method: String,
                args: serde_json::Value,
            },
        ] $($head)*);
    };
}
pub(super) use record_actions;

/// This noun's GuiReply variants, spliced into the flat enum by `flat_enum` in `gui/mod.rs`.
macro_rules! record_replies {
    ($cb:ident, $rest:tt [$($acc:tt)*] $($head:tt)*) => {
        $cb!(@next $rest [$($acc)*
            KnowledgeControl { op: Ulid, status: KnowledgeStatus },
            /// Serialized data_source Result<Value, CommandError>, bounded by the
            /// provider page limit. It carries no new document or error DTO.
            Knowledge {
                op: Ulid,
                response: serde_json::Value,
            },
        ] $($head)*);
    };
}
pub(super) use record_replies;
