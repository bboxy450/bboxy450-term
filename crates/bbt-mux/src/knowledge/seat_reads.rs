//! Pure document read jobs; the logic owner revalidates seat/root authority on completion.
use super::{data_source as d, roots::Host};
use crate::{
    index::Index,
    mcp::{self, Call, SeatActor, journal::Journal, tools},
    registry::{Registry, SeatRecord},
    store::{Result, invalid},
};
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, oneshot};

pub(crate) struct Completion {
    pub actor: SeatActor,
    pub revision: u64,
    pub reply: oneshot::Sender<Result<Value>>,
    pub result: Result<Value>,
}
pub(crate) fn handles(call: &Call) -> bool {
    matches!(
        call.tool.as_str(),
        "knowledge_fetch" | "knowledge_search" | "orient"
    ) || call.tool == "graph_read"
        && matches!(call.args["graph"].as_str(), Some("task" | "markdown"))
}
pub(crate) struct Job {
    pub host: Host,
    pub index: Arc<Index>,
    pub registry: Registry,
    pub actor: SeatActor,
    pub record: SeatRecord,
    pub call: Call,
    pub journal: Journal,
    pub orient: Option<Value>,
    pub revision: u64,
    pub reply: oneshot::Sender<Result<Value>>,
    pub permit: OwnedSemaphorePermit,
}
struct ReadFailure {
    code: String,
    message: String,
}
impl From<String> for ReadFailure {
    fn from(message: String) -> Self {
        Self {
            code: "rejected".into(),
            message,
        }
    }
}
impl From<d::CommandError> for ReadFailure {
    fn from(error: d::CommandError) -> Self {
        Self {
            code: error.code,
            message: error.message,
        }
    }
}
impl From<serde_json::Error> for ReadFailure {
    fn from(error: serde_json::Error) -> Self {
        error.to_string().into()
    }
}
impl Job {
    pub(crate) fn run(self) -> Completion {
        let Self {
            host,
            index,
            registry,
            actor,
            record,
            call,
            journal,
            orient,
            revision,
            reply,
            permit,
        } = self;
        let _permit = permit;
        let result = host
            .with_source(&index, &registry, |source| {
                let result: std::result::Result<Value, ReadFailure> = (|| match call.tool.as_str() {
                    "knowledge_fetch" => {
                        let mcp::knowledge::Request::Fetch(locator) =
                            mcp::knowledge::prepare(&call)?
                        else {
                            unreachable!()
                        };
                        Ok(serde_json::to_value(mcp::knowledge::fetch(
                            source, locator,
                        )?)?)
                    }
                    "knowledge_search" => {
                        let mcp::knowledge::Request::Search(request) =
                            mcp::knowledge::prepare(&call)?
                        else {
                            unreachable!()
                        };
                        let page = source.search(request)?;
                        let empty = page.items.is_empty();
                        Ok(tools::scoped_list(
                            &actor,
                            serde_json::to_value(page).map_err(|e| e.to_string())?,
                            empty.then_some("nothing_yet"),
                        ))
                    }
                    "orient" => {
                        let tasks = tools::orient_documents(source, &record)?;
                        let empty = tasks["items"].as_array().is_some_and(Vec::is_empty);
                        let mut base = orient.expect("host orientation prepared before IO");
                        base["tasks"] =
                            tools::scoped_list(&actor, tasks, empty.then_some("nothing_assigned"));
                        Ok(base)
                    }
                    "graph_read" => Ok(tools::graph(
                        &index,
                        source,
                        &actor,
                        &record,
                        &tools::parse(&call.args)?,
                        false,
                    )?),
                    _ => unreachable!("only document reads are scheduled"),
                })();
                let mut value = match result {
                    Ok(value) => {
                        mcp::knowledge::bounded(call.op_id, mcp::success(call.op_id, value), false)
                    }
                    Err(reason) => mcp::error(call.op_id, &reason.code, &reason.message),
                };
                if value["isError"] != true {
                    value["structuredContent"]["knowledge_root_revision"] = json!(revision);
                }
                value = mcp::knowledge::bounded(call.op_id, value, false);
                journal
                    .checked_authorized(&actor, &call, value, |old| {
                        old["structuredContent"]["knowledge_root_revision"] == json!(revision)
                            && tools::retained_documents(source, &actor, &record, &call, old)
                    })
                    .map_err(|e| d::CommandError::new("journal_unavailable", e.to_string()))
            })
            .map_err(|e| invalid(e.to_string()));
        Completion {
            actor,
            revision,
            reply,
            result,
        }
    }
}
