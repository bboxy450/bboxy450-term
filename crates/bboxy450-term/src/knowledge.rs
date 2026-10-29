// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! One bounded acceptance-record loader for the screen and `ctl e2e`.
//! Each step is a portable DataSource read; the IO owner drives replies and
//! uses its existing document notices as wakes. There is no filesystem reader,
//! polling timer, new consumer, or UI-thread IO here.
use bbt_mux::knowledge::data_source as d;
use bbt_ui::modules::e2e_checklist::{Checklist, Promise, Record, Task, parse_promises};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::collections::{BTreeSet, VecDeque};

const MAX_LIST_PAGES: usize = 32;
const MAX_RECORDS: usize = 128;
const MAX_TASKS: usize = 128;
const MAX_SNAPSHOT_BYTES: usize = 1024 * 1024;

/// JSON envelope only: arguments are serialized existing data_source DTOs.
#[derive(Debug, Clone)]
pub struct ReadCall {
    pub method: &'static str,
    pub args: Value,
}
#[derive(Debug, Clone)]
enum Work {
    Changes,
    List(Option<String>),
    Record(d::RecordRef),
    ResolveTask(String),
    Task(String, d::RecordRef),
}

pub fn promises() -> Result<Vec<Promise>, String> {
    parse_promises(include_str!(concat!(
        env!("BBOXY_INPUT_ROOT"),
        "/.bboxy/specs/e2e-acceptance.md"
    )))
}
fn value<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("portable DataSource request DTO")
}
fn decode<T: DeserializeOwned>(value: Value) -> d::Result<T> {
    serde_json::from_value(value)
        .map_err(|e| d::CommandError::new("invalid_provider_result", e.to_string()))
}
fn error(message: impl Into<String>) -> d::CommandError {
    d::CommandError::new("invalid_provider_result", message)
}

/// At most one outstanding call. Changes during a load coalesce to one more
/// refresh, so a page cannot overwrite a later document notice with stale data.
pub struct E2eLoader {
    snapshot: Checklist,
    revision: u64,
    cursor: String,
    dirty: bool,
    loading: bool,
    pending: Option<Work>,
    queue: VecDeque<Work>,
    records: Vec<Record>,
    tasks: Vec<Task>,
    tasks_started: bool,
    list_pages: usize,
    listed_records: usize,
    cursors: BTreeSet<String>,
    references: BTreeSet<(String, String)>,
}
impl E2eLoader {
    pub fn new(promises: Vec<Promise>) -> Self {
        Self {
            snapshot: Checklist {
                promises,
                ..Default::default()
            },
            revision: 0,
            cursor: "0".into(),
            dirty: true,
            loading: false,
            pending: None,
            queue: VecDeque::new(),
            records: vec![],
            tasks: vec![],
            tasks_started: false,
            list_pages: 0,
            listed_records: 0,
            cursors: BTreeSet::new(),
            references: BTreeSet::new(),
        }
    }
    pub fn snapshot(&self) -> &Checklist {
        &self.snapshot
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn changed(&mut self) {
        self.dirty = true;
    }
    /// The activity cursor belongs to one source epoch; reconnect starts it
    /// again, while a provider failure keeps the displayed summary red.
    pub fn reconnect(&mut self) {
        self.cursor = "0".into();
        self.fail(error("acceptance records are reconnecting"));
        self.dirty = true;
    }
    fn begin(&mut self) {
        self.dirty = false;
        self.loading = true;
        self.records.clear();
        self.tasks.clear();
        self.tasks_started = false;
        self.list_pages = 0;
        self.listed_records = 0;
        self.cursors.clear();
        self.references.clear();
        self.queue.push_back(Work::Changes);
    }
    fn fail(&mut self, error: d::CommandError) {
        let faults = vec![format!(
            "Acceptance records unavailable: {}",
            error.to_string().chars().take(512).collect::<String>()
        )];
        if self.snapshot.faults != faults {
            self.snapshot.faults = faults;
            self.revision = self.revision.wrapping_add(1);
        }
        self.pending = None;
        self.queue.clear();
        self.loading = false;
    }
    fn finish(&mut self) {
        if self.dirty {
            // A notice arrived after this scan began. Do not publish a green
            // snapshot that predates it; coalesce into one fresh scan instead.
            self.loading = false;
            return;
        }
        let next = Checklist {
            promises: self.snapshot.promises.clone(),
            records: std::mem::take(&mut self.records),
            tasks: std::mem::take(&mut self.tasks),
            faults: vec![],
        };
        if self.snapshot != next {
            self.snapshot = next;
            self.revision = self.revision.wrapping_add(1);
        }
        self.loading = false;
    }
    pub fn next_request(&mut self) -> Option<ReadCall> {
        if self.pending.is_some() {
            return None;
        }
        loop {
            if !self.loading {
                if !self.dirty {
                    return None;
                }
                self.begin();
            }
            if let Some(work) = self.queue.pop_front() {
                let (method, args) = match &work {
                    Work::Changes => (
                        "changes_since",
                        value(d::ChangesRequest {
                            cursor: self.cursor.clone(),
                            limit: Some(d::MAX_PAGE),
                        }),
                    ),
                    Work::List(cursor) => (
                        "list",
                        value(d::ListRequest {
                            filter: d::Filter::default(),
                            cursor: cursor.clone(),
                            limit: Some(d::MAX_PAGE),
                        }),
                    ),
                    Work::Record(reference) | Work::Task(_, reference) => (
                        "fetch",
                        value(d::FetchRequest {
                            reference: reference.clone(),
                        }),
                    ),
                    Work::ResolveTask(locator) => (
                        "resolve",
                        value(d::ResolveRequest {
                            locator: locator.clone(),
                        }),
                    ),
                };
                self.pending = Some(work);
                return Some(ReadCall { method, args });
            }
            if !self.tasks_started {
                self.tasks_started = true;
                let view = Checklist {
                    promises: self.snapshot.promises.clone(),
                    records: self.records.clone(),
                    ..Default::default()
                };
                // The projection owns relation/alias/status rules. Use its
                // unresolved tracking targets instead of another link matcher.
                let targets = view
                    .project()
                    .rows
                    .iter()
                    .flat_map(|r| &r.tracking)
                    .map(|t| t.reference.clone())
                    .collect::<BTreeSet<_>>();
                if targets.len() > MAX_TASKS {
                    self.fail(error("acceptance tracked-task bound exceeded"));
                    return None;
                }
                self.queue
                    .extend(targets.into_iter().map(Work::ResolveTask));
                continue;
            }
            self.finish();
        }
    }
    pub fn respond(&mut self, response: d::Result<Value>) {
        let Some(work) = self.pending.take() else {
            self.fail(error("unsolicited knowledge reply"));
            return;
        };
        let response = match response {
            Err(e)
                if matches!(&work, Work::ResolveTask(_) | Work::Task(_, _))
                    && matches!(
                        e.code.as_str(),
                        "not_found" | "ambiguous_reference" | "invalid_locator"
                    ) =>
            {
                let target = match work {
                    Work::ResolveTask(t) | Work::Task(t, _) => t,
                    _ => unreachable!(),
                };
                self.tasks.push(Task {
                    reference: target,
                    status: "unknown".into(),
                });
                return;
            }
            other => other,
        };
        let result = response.and_then(|value| {
            if serde_json::to_vec(&value)
                .map_err(|e| error(e.to_string()))?
                .len()
                > d::MAX_PAGE_BYTES
            {
                return Err(error("knowledge reply exceeds provider page bound"));
            }
            self.apply(work, value)
        });
        if let Err(error) = result {
            self.fail(error);
        }
    }
    fn apply(&mut self, work: Work, value: Value) -> d::Result<()> {
        match work {
            Work::Changes => {
                let changes: d::Changes = decode(value)?;
                if changes.changed.len() > d::MAX_PAGE || changes.next.is_empty() {
                    return Err(error("invalid knowledge changes page"));
                }
                self.cursor = changes.next;
                // The event is a wake, never document truth. A bounded full
                // read also stays fresh while an old activity cursor catches up.
                self.queue.push_back(Work::List(None));
            }
            Work::List(_) => {
                let page: d::Page<d::RecordRef> = decode(value)?;
                self.list_pages += 1;
                if page.items.len() > d::MAX_PAGE || self.list_pages > MAX_LIST_PAGES {
                    return Err(error("acceptance catalogue page bound exceeded"));
                }
                for reference in page.items {
                    if !self
                        .references
                        .insert((reference.id.clone(), reference.path.clone()))
                    {
                        return Err(error("knowledge list repeated a record"));
                    }
                    if reference.path.starts_with("e2e/") {
                        if !reference.path.ends_with(".md")
                            || reference
                                .path
                                .split('/')
                                .any(|s| s.is_empty() || s == "." || s == "..")
                        {
                            return Err(error("invalid acceptance record path"));
                        }
                        self.listed_records += 1;
                        if self.listed_records > MAX_RECORDS {
                            return Err(error("acceptance record bound exceeded"));
                        }
                        self.queue.push_back(Work::Record(reference));
                    }
                }
                if let Some(next) = page.next {
                    if next.is_empty() || !self.cursors.insert(next.clone()) {
                        return Err(error("knowledge list repeated an invalid cursor"));
                    }
                    self.queue.push_back(Work::List(Some(next)));
                }
            }
            Work::Record(reference) => {
                let record: d::Record = decode(value)?;
                validate_record(&record, &reference)?;
                if record.frontmatter["type"] != "e2e" {
                    return Err(error(format!("{} is not an e2e record", reference.path)));
                }
                let row: Record = decode(record.frontmatter)?;
                if row.number != reference.number || row.number.is_none() {
                    return Err(error(format!("{} has an invalid number", reference.path)));
                }
                self.records.push(row);
            }
            Work::ResolveTask(target) => {
                self.queue.push_front(Work::Task(target, decode(value)?));
            }
            Work::Task(target, reference) => {
                let record: d::Record = decode(value)?;
                validate_record(&record, &reference)?;
                self.tasks.push(Task {
                    reference: target,
                    status: if record.frontmatter["type"] == "task" {
                        record.frontmatter["status"].as_str().unwrap_or("unknown")
                    } else {
                        "unknown"
                    }
                    .into(),
                });
            }
        }
        if serde_json::to_vec(&(&self.records, &self.tasks))
            .map_err(|e| error(e.to_string()))?
            .len()
            > MAX_SNAPSHOT_BYTES
        {
            return Err(error("acceptance snapshot byte bound exceeded"));
        }
        Ok(())
    }
}

/// IO-owned observer. One request may be on the wire; closing stops further
/// reads and reconnecting refuses replies from the previous source epoch.
pub struct Watch {
    loader: E2eLoader,
    observing: bool,
    connected: bool,
    pending: Option<bbt_mux::Ulid>,
    outstanding: BTreeSet<bbt_mux::Ulid>,
    read_limit: bool,
    restart: bool,
    published: u64,
    scope: Result<Option<bbt_proto::gui::SwarmScope>, String>,
    source: Option<crate::documents::Source>,
    active_call: Option<ReadCall>,
    retry_call: Option<ReadCall>,
    retry_at: Option<std::time::Instant>,
    retry_started: Option<std::time::Instant>,
    retries: u8,
}
impl Watch {
    pub fn new(promises: Vec<Promise>) -> Self {
        Self {
            loader: E2eLoader::new(promises),
            observing: false,
            connected: false,
            pending: None,
            outstanding: BTreeSet::new(),
            read_limit: false,
            restart: false,
            published: 0,
            scope: Ok(None),
            source: None,
            active_call: None,
            retry_call: None,
            retry_at: None,
            retry_started: None,
            retries: 0,
        }
    }
    pub fn bind_scope(&mut self, scope: Result<Option<bbt_proto::gui::SwarmScope>, String>) {
        if self.scope == scope {
            return;
        }
        self.scope = scope;
        self.reset_source();
        if let Err(reason) = &self.scope {
            self.loader
                .fail(d::CommandError::new("workspace_unavailable", reason));
        }
    }
    pub fn scope(&self) -> Option<bbt_proto::gui::SwarmScope> {
        self.scope.as_ref().ok().copied().flatten()
    }
    pub fn deadline(&self) -> Option<std::time::Instant> {
        self.retry_at
    }
    fn reset_source(&mut self) {
        let promises = self.loader.snapshot().promises.clone();
        self.loader = E2eLoader::new(promises);
        self.pending = None;
        self.source = None;
        self.active_call = None;
        self.retry_call = None;
        self.retry_at = None;
        self.retry_started = None;
        self.retries = 0;
        self.restart = false;
        self.read_limit = false;
        self.published = u64::MAX;
    }
    /// Called only for a coalesced UI demand notification. A close/open pair
    /// may coalesce to true, so true also forces a fresh source snapshot.
    pub fn observe(&mut self, enabled: bool) {
        self.observing = enabled;
        if enabled {
            self.read_limit = false;
            if self.pending.is_some() {
                self.restart = true;
            } else {
                self.loader.reconnect();
            }
        }
    }
    pub fn connection(&mut self, connected: bool) {
        if self.connected != connected {
            self.connected = connected;
            self.pending = None;
            self.outstanding.clear();
            self.read_limit = false;
            self.restart = false;
            self.active_call = None;
            self.retry_call = None;
            self.retry_at = None;
            self.retry_started = None;
            self.retries = 0;
            self.source = None;
            if self.observing {
                self.loader.reconnect();
            }
        }
    }
    pub fn source_changed(&mut self) {
        self.reset_source();
    }
    pub fn changed(&mut self) {
        if self.observing {
            self.loader.changed();
        }
    }
    pub fn next_action(&mut self) -> Option<bbt_proto::GuiAction> {
        if !self.observing
            || !self.connected
            || self.pending.is_some()
            || self.read_limit
            || self.scope.is_err()
            || self
                .retry_at
                .is_some_and(|at| std::time::Instant::now() < at)
        {
            return None;
        }
        if self.outstanding.len() >= 8 {
            self.read_limit = true;
            self.loader.fail(d::CommandError::new(
                "workspace_read_limit",
                "eight previous graph reads are still outstanding; reopen after they finish",
            ));
            return None;
        }
        self.retry_at = None;
        let call = self
            .retry_call
            .take()
            .or_else(|| self.loader.next_request())?;
        self.active_call = Some(call.clone());
        let id = bbt_mux::Ulid::new();
        self.pending = Some(id);
        self.outstanding.insert(id);
        Some(bbt_proto::GuiAction::Knowledge {
            id,
            scope: self.scope(),
            method: call.method.into(),
            args: call.args,
        })
    }
    pub fn response(&mut self, op: bbt_mux::Ulid, mut result: d::Result<Value>) {
        self.outstanding.remove(&op);
        if self.pending != Some(op) {
            return;
        }
        self.pending = None;
        if self.restart || !self.observing {
            self.active_call = None;
            self.restart = false;
            self.loader.reconnect();
            return;
        }
        if let Err(error) = &result
            && error.code == "workspace_pending"
        {
            let now = std::time::Instant::now();
            let began = self.retry_started.get_or_insert(now);
            if self.retries < 50
                && now.saturating_duration_since(*began) < std::time::Duration::from_secs(10)
            {
                self.retries += 1;
                self.retry_call = self.active_call.take();
                self.retry_at = Some(
                    now + std::time::Duration::from_millis(
                        error.retry_after_ms.unwrap_or(100).clamp(25, 1000),
                    ),
                );
                return;
            }
        }
        self.active_call = None;
        if let (Some(scope), Ok(value)) = (self.scope(), &result) {
            match crate::documents::Source::decode(value, scope) {
                Ok(source) if self.source.as_ref().is_none_or(|old| *old == source) => {
                    self.source = Some(source)
                }
                _ => {
                    result = Err(d::CommandError::new(
                        "document_source_changed",
                        "graph replies changed workspace/provider during loading",
                    ))
                }
            }
        }
        self.loader.respond(result);
    }
    pub fn publication(&mut self) -> Option<std::sync::Arc<Checklist>> {
        if !self.observing || self.published == self.loader.revision() {
            return None;
        }
        self.published = self.loader.revision();
        Some(std::sync::Arc::new(self.loader.snapshot().clone()))
    }
}
fn validate_record(record: &d::Record, reference: &d::RecordRef) -> d::Result<()> {
    if &record.reference != reference || record.etag != reference.digest {
        return Err(error("knowledge record changed during the read"));
    }
    if let Some(reason) = &record.parse_error {
        return Err(error(format!("{}: {reason}", reference.path)));
    }
    if reference.number.is_none_or(|n| n == 0)
        || record.frontmatter["number"].as_u64() != reference.number
    {
        return Err(error(format!("{} has an invalid number", reference.path)));
    }
    Ok(())
}

/// The group flag limits rows, never the global acceptance verdict.
pub fn e2e_lines(checklist: &Checklist, group: Option<&str>) -> Result<Vec<Value>, String> {
    if let Some(group) = group
        && !checklist.promises.iter().any(|p| p.group == group)
    {
        return Err(format!("unknown e2e group: {group}"));
    }
    let projection = checklist.project();
    let mut rows = projection.rows.iter().filter(|r| group.is_none_or(|g| r.promise.group == g)).map(|r| {
        json!({"kind":"e2e","function":r.promise.id,"group":r.promise.group,"status":r.status.as_str(),"fault":r.fault,
            "number":r.record.and_then(|r|r.number),"last_run":r.record.and_then(|r|r.last_run.as_deref()),
            "run_by":r.record.and_then(|r|r.run_by.as_deref()),"coverage":r.record.and_then(|r|r.coverage.as_deref()),
            "partial":r.partial,"tracked_tasks":r.tracking,"tracked_by":r.tracked_by})
    }).collect::<Vec<_>>();
    let mut summary = value(&projection.summary);
    summary["kind"] = json!("summary");
    summary["state"] = summary["tone"].take();
    summary
        .as_object_mut()
        .expect("summary object")
        .remove("tone");
    rows.push(summary);
    Ok(rows)
}
