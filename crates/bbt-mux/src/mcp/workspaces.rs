// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Managed source checkouts. Knowledge roots and process ownership stay separate.
mod git;
#[cfg(test)]
mod tests;

use super::{Call, SeatActor, tools};
use crate::{
    knowledge::{DataSource, data_source as d, workspaces::RootBinding},
    registry::{Class, Lifecycle, Registry},
    store::{Result, Store, invalid, sqlite::SqliteStore},
};
use rusqlite::{TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use ulid::Ulid;

pub(crate) fn handles(name: &str) -> bool {
    matches!(
        name,
        "workspace_orient"
            | "workspace_prepare"
            | "workspace_get"
            | "workspace_list"
            | "workspace_release"
    )
}
pub(crate) fn mutation(name: &str) -> bool {
    matches!(name, "workspace_prepare" | "workspace_release")
}
pub(crate) fn catalogue(actor: &SeatActor) -> Vec<Value> {
    let id = json!({"type":"string","pattern":"^[0-7][0-9A-HJKMNP-TV-Z]{25}$"});
    let mut rows = vec![(
        "workspace_get",
        "Read a managed implementation workspace; omit workspace_id for your own binding. Preparing/unknown resources are never a launch permission.",
        json!({"workspace_id":id}),
        vec![],
    )];
    if actor.role.class == Class::Oracle {
        rows.extend([
            ("workspace_prepare","Prepare an isolated local Git worktree for a task. repository is current or an absolute Git root beneath the selected knowledge repository; base defaults to origin/main. Returns preparing promptly; inspect workspace_get. Keeps the swarm's canonical KB. Remote providers are explicitly unavailable. Never shell out to the retired dispatch script.",json!({"task_path":{"type":"string","minLength":1,"maxLength":4096},"repository":{"type":"string","minLength":1,"maxLength":4096},"base":{"type":"string","minLength":1,"maxLength":256},"provider":{"enum":["git"]},"profile":{"enum":["git"]},"attempt":{"type":"integer","minimum":1},"integration_ref":{"type":"string","minLength":1,"maxLength":256}}),vec!["task_path","repository","provider"]),
            ("workspace_list","List managed workspaces in your swarm, including partial/unknown resources. Bounded opaque cursor; no filesystem scan.",json!({"cursor":id,"limit":{"type":"integer","minimum":1,"maximum":128},"state":{"enum":["preparing","ready","in_use","failed","unknown","releasing","released"]},"task_id":{"type":"string"}}),vec![]),
            ("workspace_release","Release only this service's exact owned resources. Requires a closed/dead writer, preserved commits and no tracked, untracked, ignored private files or open processes. Never force-removes or stashes. Returns releasing; inspect workspace_get.",json!({"workspace_id":id,"expected_revision":{"type":"integer","minimum":1}}),vec!["workspace_id","expected_revision"]),
        ]);
    }
    rows.push(("workspace_orient","Load the managed worker workspace workflow, configured providers and exact prepare/launch/delegate/recovery/release guidance.",json!({}),vec![]));
    rows.into_iter().map(|(name,description,properties,required)|json!({"name":name,"description":description,"inputSchema":{"type":"object","properties":properties,"required":required,"additionalProperties":false},"outputSchema":{"type":"object","additionalProperties":true},"annotations":{"readOnlyHint":!mutation(name),"destructiveHint":mutation(name),"idempotentHint":true,"openWorldHint":false}})).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Reference {
    pub id: Ulid,
    pub expected_revision: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum State {
    Preparing,
    Ready,
    InUse,
    Failed,
    Unknown,
    Releasing,
    Released,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Holder {
    pub seat_id: Ulid,
    pub generation: u64,
    pub launch_operation: Ulid,
    pub pid: Option<u32>,
    #[serde(default)]
    pub launch_requested: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct Source {
    pub repository: PathBuf,
    pub common: PathBuf,
    pub common_identity: (u64, u64),
    pub base: String,
    pub integration_ref: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Record {
    pub workspace_id: Ulid,
    pub revision: u64,
    pub state: State,
    pub swarm_id: Ulid,
    pub task_ref: d::RecordRef,
    pub task_etag: String,
    pub task_contract_digest: String,
    pub source: Source,
    pub provider: String,
    pub profile: String,
    pub attempt: u64,
    pub knowledge_binding: RootBinding,
    pub provider_revision: u64,
    pub cwd: PathBuf,
    pub branch: String,
    pub resource_token: String,
    pub checkout_identity: Option<(u64, u64)>,
    #[serde(default)]
    pub preserved_module_repositories: Option<PathBuf>,
    pub holder: Option<Holder>,
    pub operation_id: Ulid,
    pub actor: SeatActor,
    pub request: Call,
    pub error: Option<String>,
}
impl Record {
    pub fn view(&self) -> Value {
        let mut v = serde_json::to_value(self).expect("workspace serialization");
        // Custody/input bookkeeping is not a second public authoring API.
        for key in ["actor", "request", "resource_token", "checkout_identity"] {
            v.as_object_mut().unwrap().remove(key);
        }
        v["target"] = json!({"kind":"local","cwd":self.cwd});
        v
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Prepare {
    pub task_path: String,
    pub repository: String,
    #[serde(default = "base")]
    pub base: String,
    pub provider: String,
    pub profile: Option<String>,
    #[serde(default = "attempt")]
    pub attempt: u64,
    pub integration_ref: Option<String>,
}
fn base() -> String {
    "origin/main".into()
}
fn attempt() -> u64 {
    1
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Get {
    pub workspace_id: Option<Ulid>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct List {
    pub cursor: Option<Ulid>,
    pub limit: Option<usize>,
    pub state: Option<State>,
    pub task_id: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Release {
    pub workspace_id: Ulid,
    pub expected_revision: u64,
}

#[derive(Clone)]
pub(crate) struct Service {
    store: SqliteStore,
    parent: PathBuf,
}
pub(crate) enum Work {
    Prepare(Record),
    Inspect(Record),
    Release(Record),
}
impl Work {
    pub fn record(&self) -> &Record {
        match self {
            Self::Prepare(r) | Self::Inspect(r) | Self::Release(r) => r,
        }
    }
}
pub(crate) struct Begun {
    pub value: Value,
    pub work: Option<Work>,
}
impl Service {
    pub fn open(run_dir: &Path) -> Result<Self> {
        let dir = crate::store::object_dir(run_dir, "implementation-workspaces")?;
        let parent = crate::store::object_dir(&dir, "checkouts")?.canonicalize()?;
        let service = Self {
            store: SqliteStore::open(&dir)?,
            parent,
        };
        // One startup pass, never a build/file-watch reaction. External effects
        // are only inspected after restart; a missing response never repeats add/remove.
        for (_, bytes) in service.store.list("workspace/")? {
            let mut r: Record = serde_json::from_slice(&bytes)?;
            if matches!(r.state, State::Preparing | State::Releasing) {
                let release = r.state == State::Releasing;
                r.state = State::Unknown;
                r.error = Some(
                    if release {
                        "release: backend restarted; inspect the original resource"
                    } else {
                        "backend restarted during provider work; inspect the original resource"
                    }
                    .into(),
                );
                r.revision += 1;
                service
                    .store
                    .put(&key(r.workspace_id), &serde_json::to_vec(&r)?)?;
            }
        }
        Ok(service)
    }
    pub fn get(&self, id: Ulid) -> Result<Record> {
        let bytes = self
            .store
            .get(&key(id))?
            .ok_or_else(|| invalid("workspace_not_found: unknown workspace_id"))?;
        Ok(serde_json::from_slice(&bytes)?)
    }
    fn update<T>(&self, id: Ulid, f: impl FnOnce(&mut Record) -> Result<T>) -> Result<T> {
        let mut db = self.store.connection()?;
        let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let bytes: Vec<u8> =
            tx.query_row("SELECT value FROM kv WHERE key=?1", [key(id)], |r| r.get(0))?;
        let mut record: Record = serde_json::from_slice(&bytes)?;
        let value = f(&mut record)?;
        tx.execute(
            "UPDATE kv SET value=?2 WHERE key=?1",
            params![key(id), serde_json::to_vec(&record)?],
        )?;
        if let Some(h) = &record.holder {
            tx.execute(
                "INSERT OR REPLACE INTO kv(key,value) VALUES(?1,?2)",
                params![
                    format!("holder/{}/{}", h.seat_id, h.generation),
                    record.workspace_id.to_string().into_bytes()
                ],
            )?;
        }
        if let Some(h) = &record.holder {
            tx.execute(
                "INSERT OR REPLACE INTO kv(key,value) VALUES(?1,?2)",
                params![
                    format!("holder-seat/{}", h.seat_id),
                    record.workspace_id.to_string().into_bytes()
                ],
            )?;
        }
        tx.commit()?;
        Ok(value)
    }
    pub fn own(&self, actor: &SeatActor) -> Result<Option<Record>> {
        let Some(bytes) = self.store.get(&format!("holder-seat/{}", actor.seat_id))? else {
            return Ok(None);
        };
        let r = self.get(String::from_utf8(bytes)?.parse::<Ulid>()?)?;
        if r.swarm_id != actor.swarm_id
            || r.holder.as_ref().is_none_or(|h| {
                h.seat_id != actor.seat_id || h.generation != actor.process_generation
            })
        {
            return Err(invalid(
                "workspace_stale_holder: this seat generation cannot silently become an unmanaged writer",
            ));
        }
        Ok(Some(r))
    }
    pub fn authorize(record: &Record, actor: &SeatActor) -> Result<()> {
        if record.swarm_id != actor.swarm_id
            || (actor.role.class != Class::Oracle
                && !record.holder.as_ref().is_some_and(|h| {
                    h.seat_id == actor.seat_id && h.generation == actor.process_generation
                }))
        {
            return Err(invalid(
                "workspace_forbidden: outside current seat/swarm authority",
            ));
        }
        Ok(())
    }
    #[allow(
        clippy::too_many_arguments,
        reason = "captured authority, source, KB context and registry are independent trust boundaries"
    )]
    pub fn begin(
        &self,
        actor: &SeatActor,
        call: &Call,
        repository: &Path,
        binding: &RootBinding,
        provider_revision: u64,
        source: &dyn DataSource,
        registry: &Registry,
    ) -> Result<Begun> {
        if (mutation(&call.tool) || call.tool == "workspace_list")
            && actor.role.class != Class::Oracle
        {
            return Err(invalid("workspace_forbidden: requires oracle authority"));
        }
        match call.tool.as_str() {
            "workspace_orient" => {
                let _: tools::Empty =
                    tools::parse(&call.args).map_err(|e| invalid(e.to_string()))?;
                let tools = catalogue(actor);
                let can_provision = tools.iter().any(|tool| tool["name"] == "workspace_prepare");
                Ok(Begun {
                    value: json!({"can_provision":can_provision,"providers":[{"id":"git","profiles":["git"],"execution":"local","repository":"current admitted source repository, or an explicit canonical Git root beneath the selected knowledge repository (including child projects); unrelated external roots are not configured in this first provider","base_default":"origin/main","base_resolution":"Refs resolve in the admitted source repository. Pass an explicit commit SHA to fork a worker branch checkpoint; uncommitted WIP is not copied.","fetch":"explicit repository preparation outside this tool; missing source fails without branch fallback","setup":"pinned recursive submodule materialization; repository build/test commands remain in its Makefile"}],"tools":tools,"implementation_workspace":self.own(actor)?.map(|r|r.view()),"workflow":["Create/read the native task with its exact allowed_paths and verify command.","workspace_prepare {task_path,repository:current,provider:git,base,attempt?} reserves one resource; retain returned operation ID and workspace_id.","workspace_get {workspace_id} until ready. Preparing/releasing/unknown never authorizes launch. Existing operation_status inspects lost-response outcomes; never repeat an unknown effect.","seat_spawn or seat_split with workspace:{id,expected_revision}, approved_profile,swarm_id and role. Supply workspace OR cwd, never both. Managed source checkout does not replace the canonical knowledge root.","task_delegate {task_path,seat_id} for open/unassigned work, or task_reassign for an existing assignment; launch success is not task delivery. The binding must name this task and current seat generation.","Review actual source and verification, land through repository policy, then close the writer. No automatic merge or push.","workspace_release {workspace_id,expected_revision} after work is preserved. Dirty/untracked/ignored files, unpreserved commits, live/uncertain writers and changed resource identities refuse. Inspect releasing/unknown through workspace_get; no force delete."],"guidance":"Use these MCP tools for managed workers. The old make dispatch and dispatch-codex scripts are retired. Remote providers and arbitrary setup scripts are not installed; no silent local fallback."}),
                    work: None,
                })
            }
            "workspace_prepare" => {
                let args: Prepare = tools::parse(&call.args).map_err(|e| invalid(e.to_string()))?;
                if args.provider != "git" || args.profile.as_deref().is_some_and(|v| v != "git") {
                    return Err(invalid(
                        "workspace_provider_unavailable: only local git profile is installed",
                    ));
                }
                if args.attempt == 0 {
                    return Err(invalid("invalid_attempt: expected a positive integer"));
                }
                let knowledge_repository = repository.canonicalize()?;
                let repository = if args.repository == "current" {
                    self.own(actor)?
                        .map(|r| r.source.repository)
                        .unwrap_or_else(|| knowledge_repository.clone())
                } else {
                    let selected = Path::new(&args.repository);
                    if !selected.is_absolute() {
                        return Err(invalid(
                            "workspace_repository_forbidden: expected current or an explicit absolute repository under the selected knowledge repository",
                        ));
                    }
                    selected.canonicalize()?
                };
                if !repository.starts_with(&knowledge_repository) {
                    return Err(invalid(
                        "workspace_repository_forbidden: source repository must stay under this swarm's canonical knowledge repository; symlink escapes are refused",
                    ));
                }
                let task = source
                    .resolve(d::ResolveRequest {
                        locator: args.task_path,
                    })
                    .and_then(|reference| source.fetch(d::FetchRequest { reference }))
                    .map_err(|e| invalid(e.to_string()))?;
                if !matches!(
                    task.frontmatter["type"].as_str(),
                    Some("task" | "bug" | "epic")
                ) || !matches!(
                    task.frontmatter["status"].as_str(),
                    Some("open" | "assigned" | "active" | "running" | "blocked")
                ) {
                    return Err(invalid(
                        "workspace_task_invalid: expected an open native task",
                    ));
                }
                let source = git::source(&repository, &args.base, args.integration_ref.as_deref())?;
                let mut db = self.store.connection()?;
                let tx = db.transaction_with_behavior(TransactionBehavior::Immediate)?;
                let mut query = tx.prepare("SELECT value FROM kv WHERE key GLOB 'workspace/*'")?;
                for bytes in query.query_map([], |row| row.get::<_, Vec<u8>>(0))? {
                    let r: Record = serde_json::from_slice(&bytes?)?;
                    if r.swarm_id == actor.swarm_id
                        && r.knowledge_binding.root_id == binding.root_id
                        && r.task_ref.id == task.reference.id
                        && r.state != State::Released
                    {
                        if r.attempt == args.attempt {
                            return Err(invalid(format!(
                                "workspace_already_exists: use workspace_get {}",
                                r.workspace_id
                            )));
                        }
                        if r.holder.is_some()
                            || matches!(
                                r.state,
                                State::Preparing | State::Releasing | State::Unknown
                            )
                        {
                            return Err(invalid(
                                "workspace_in_use: reconcile pending/unknown provider work and relinquish the previous attempt's writer before another attempt",
                            ));
                        }
                    }
                }
                drop(query);
                let id = call.op_id;
                let task_contract_digest = task_contract(&task)?;
                let r = Record {
                    workspace_id: id,
                    revision: 1,
                    state: State::Preparing,
                    swarm_id: actor.swarm_id,
                    task_ref: task.reference,
                    task_etag: task.etag,
                    task_contract_digest,
                    source,
                    provider: "git".into(),
                    profile: "git".into(),
                    attempt: args.attempt,
                    knowledge_binding: binding.clone(),
                    provider_revision,
                    cwd: self.parent.join(id.to_string()),
                    branch: format!("bboxy/{id}"),
                    resource_token: id.to_string(),
                    checkout_identity: None,
                    preserved_module_repositories: None,
                    holder: None,
                    operation_id: id,
                    actor: actor.clone(),
                    request: call.clone(),
                    error: None,
                };
                tx.execute(
                    "INSERT INTO kv(key,value) VALUES(?1,?2)",
                    params![key(id), serde_json::to_vec(&r)?],
                )?;
                tx.commit()?;
                Ok(Begun {
                    value: json!({"workspace":r.view()}),
                    work: Some(Work::Prepare(r)),
                })
            }
            "workspace_get" => {
                let args: Get = tools::parse(&call.args).map_err(|e| invalid(e.to_string()))?;
                let mut r = match args.workspace_id {
                    Some(id) => self.get(id)?,
                    None => self.own(actor)?.ok_or_else(|| {
                        invalid("workspace_not_found: seat has no managed implementation workspace")
                    })?,
                };
                Self::authorize(&r, actor)?;
                if r.state == State::InUse
                    && r.holder.as_ref().is_some_and(|h| {
                        h.pid.is_some_and(|pid| !git::process_exists(pid))
                            && registry.get(h.seat_id).is_ok_and(|seat| {
                                seat.lifecycle != Lifecycle::Open && seat.generation == h.generation
                            })
                    })
                {
                    r = self.update(r.workspace_id, |current| {
                        if current.revision == r.revision {
                            current.holder = None;
                            current.state = State::Ready;
                            current.revision += 1;
                        }
                        Ok(current.clone())
                    })?;
                }
                let work = if r.state == State::Unknown && Self::provider_idle(&r)? {
                    self.update(r.workspace_id, |current| {
                        if current.revision != r.revision || current.state != State::Unknown {
                            return Ok(None);
                        }
                        current.revision += 1;
                        current.state = if current
                            .error
                            .as_deref()
                            .is_some_and(|e| e.starts_with("release:"))
                        {
                            State::Releasing
                        } else {
                            State::Preparing
                        };
                        Ok(Some(Work::Inspect(current.clone())))
                    })?
                } else {
                    None
                };
                Ok(Begun {
                    value: json!({"workspace":r.view()}),
                    work,
                })
            }
            "workspace_list" => {
                let args: List = tools::parse(&call.args).map_err(|e| invalid(e.to_string()))?;
                let limit = tools::page_limit(args.limit).map_err(|e| invalid(e.to_string()))?;
                let mut rows = Vec::new();
                let mut bytes = 0;
                let mut more = false;
                let cursor = args.cursor.unwrap_or(Ulid::nil());
                let db = self.store.connection()?;
                let mut query = db.prepare(
                    "SELECT value FROM kv WHERE key>?1 AND key<'workspace0' ORDER BY key",
                )?;
                for row in query.query_map([key(cursor)], |row| row.get::<_, Vec<u8>>(0))? {
                    let r: Record = serde_json::from_slice(&row?)?;
                    if r.swarm_id == actor.swarm_id
                        && args.state.as_ref().is_none_or(|v| v == &r.state)
                        && args.task_id.as_ref().is_none_or(|id| id == &r.task_ref.id)
                    {
                        let size = serde_json::to_vec(&r.view())?.len();
                        if rows.len() == limit || bytes + size > super::MAX_RESULT / 2 {
                            if rows.is_empty() {
                                return Err(invalid(
                                    "workspace_result_too_large: individual record exceeds page bound",
                                ));
                            }
                            more = true;
                            break;
                        }
                        bytes += size;
                        rows.push(r);
                    }
                }
                Ok(Begun {
                    value: json!({"items":rows.iter().map(Record::view).collect::<Vec<_>>(),"next":if more{rows.last().map(|r|r.workspace_id)}else{None}}),
                    work: None,
                })
            }
            "workspace_release" => {
                let args: Release = tools::parse(&call.args).map_err(|e| invalid(e.to_string()))?;
                let r = self.update(args.workspace_id, |r| {
                    Self::authorize(r, actor)?;
                    if r.revision != args.expected_revision || args.expected_revision == 0 {
                        return Err(invalid(
                            "workspace_stale_revision: read workspace_get before release",
                        ));
                    }
                    if r.state == State::Released {
                        return Ok(r.clone());
                    }
                    if let Some(h) = &r.holder {
                        let seat = registry
                            .get(h.seat_id)
                            .map_err(|_| invalid("workspace_in_use: writer outcome is unknown"))?;
                        if seat.lifecycle == Lifecycle::Open
                            || seat.generation != h.generation
                            || h.pid.is_none()
                            || h.pid.is_some_and(git::process_exists)
                        {
                            return Err(invalid(
                                "workspace_in_use: close and detach the exact writer first",
                            ));
                        }
                    }
                    if matches!(
                        r.state,
                        State::Preparing | State::Releasing | State::Unknown
                    ) {
                        return Err(invalid(
                            "workspace_in_use: provider operation needs reconciliation",
                        ));
                    }
                    r.holder = None;
                    r.revision += 1;
                    r.state = State::Releasing;
                    r.operation_id = call.op_id;
                    r.request = call.clone();
                    r.actor = actor.clone();
                    r.error = Some("release: pending preservation check".into());
                    Ok(r.clone())
                })?;
                Ok(Begun {
                    value: json!({"workspace":r.view()}),
                    work: (r.state != State::Released).then_some(Work::Release(r)),
                })
            }
            _ => Err(invalid("unknown workspace tool")),
        }
    }
    fn provider_lease(record: &Record) -> Result<std::fs::File> {
        use std::os::unix::fs::OpenOptionsExt;
        let parent = record
            .cwd
            .parent()
            .and_then(Path::parent)
            .ok_or_else(|| invalid("workspace_lease_invalid"))?
            .join("provider-leases");
        match std::fs::create_dir(&parent) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
        git::identity(&parent)?;
        Ok(std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .open(parent.join(format!("{}.lock", record.workspace_id)))?)
    }
    fn provider_idle(record: &Record) -> Result<bool> {
        let file = Self::provider_lease(record)?;
        Ok(file.try_lock().is_ok())
    }
    pub fn run(&self, work: &Work) -> Result<Record> {
        let record = work.record();
        let lease = Self::provider_lease(record)?;
        lease.try_lock().map_err(|_|invalid("workspace_provider_active: original provider work still holds its inherited resource lease"))?;
        let current = self.get(record.workspace_id)?;
        if current.revision != record.revision || current.operation_id != record.operation_id {
            return Err(invalid(
                "workspace_stale_completion: queued provider work lost its resource generation before starting",
            ));
        }
        // The lease is inherited by every provider child, so a logic-process
        // death cannot make an old Git effect look absent/safe to replace.
        git::with_provider_lease(std::sync::Arc::new(lease), || match work {
            Work::Prepare(r) => {
                // Claim an empty checkout directory exclusively and persist its
                // inode BEFORE Git can mutate it. A matching branch/path alone
                // never establishes that our create operation made a resource.
                std::fs::create_dir(&r.cwd).map_err(|_|invalid("workspace_collision: reserved checkout path already exists or cannot be created"))?;
                let inode = git::identity(&r.cwd)?;
                let claimed = self.update(r.workspace_id, |current| {
                    if current.revision != r.revision
                        || current.operation_id != r.operation_id
                        || current.checkout_identity.is_some()
                    {
                        return Err(invalid(
                            "workspace_stale_completion: checkout claim lost its admission",
                        ));
                    }
                    current.checkout_identity = Some(inode);
                    Ok(current.clone())
                })?;
                git::prepare(&claimed)
            }
            Work::Release(r) => git::release(r),
            Work::Inspect(r) => git::inspect(r),
        })
    }
    pub fn finish(&self, before: &Record, result: Result<Record>, current: bool) -> Result<Record> {
        self.update(before.workspace_id,|r|{
            if r.revision!=before.revision || r.operation_id!=before.operation_id {return Err(invalid("workspace_stale_completion: newer state owns this workspace"));}
            match result {
                Ok(mut next)=>{next.revision=r.revision+1;if !current && next.state!=State::Released {next.state=State::Failed;next.error=Some("workspace_binding_changed: original resources retained; new authority must inspect before use".into());}*r=next;},
                Err(error)=>{
                    let message=error.to_string();let uncertain=message.contains("workspace_provider_timeout")||message.contains("workspace_provider_unknown")||message.contains("workspace_provider_active")||message.contains("workspace_release_unknown");
                    r.revision+=1;r.state=if uncertain{State::Unknown}else{State::Failed};
                    r.error=Some(if before.state==State::Releasing{format!("release: {message}")}else{message});
                }
            }
            Ok(r.clone())
        })
    }
    pub fn resolve(
        &self,
        reference: &Reference,
        actor: &SeatActor,
        binding: &RootBinding,
        provider_revision: u64,
    ) -> Result<Record> {
        let r = self.get(reference.id)?;
        Self::authorize(&r, actor)?;
        if r.revision != reference.expected_revision {
            return Err(invalid("workspace_stale_revision: read workspace_get"));
        }
        if r.state != State::Ready || r.holder.is_some() {
            return Err(invalid(
                "workspace_in_use: workspace must be ready and unbound",
            ));
        }
        if r.knowledge_binding != *binding || r.provider_revision != provider_revision {
            return Err(invalid(
                "workspace_binding_changed: task knowledge root/provider changed",
            ));
        }
        Ok(r)
    }
    /// Shared execution seam for admitted script/tool workers. Resolve the
    /// current writing lease, never a cwd supplied by a script. Callers retain
    /// this revision and recheck it before publishing an asynchronous result.
    pub fn execution_context(
        &self,
        actor: &SeatActor,
        binding: &RootBinding,
        provider_revision: u64,
        expected: Option<&Reference>,
    ) -> Result<Option<Record>> {
        let Some(r) = self.own(actor)? else {
            if expected.is_some() {
                return Err(invalid(
                    "workspace_not_found: caller has no managed workspace",
                ));
            }
            return Ok(None);
        };
        if r.state != State::InUse || r.holder.as_ref().is_none_or(|h| h.pid.is_none()) {
            return Err(invalid(
                "workspace_in_use: current writing lease is not a confirmed live launch",
            ));
        }
        if &r.knowledge_binding != binding || r.provider_revision != provider_revision {
            return Err(invalid(
                "workspace_binding_changed: source and canonical KB context differ",
            ));
        }
        if r.checkout_identity != Some(git::identity(&r.cwd)?)
            || git::identity(&r.source.common)? != r.source.common_identity
        {
            return Err(invalid(
                "workspace_identity_changed: execution source was replaced",
            ));
        }
        if expected.is_some_and(|reference| {
            reference.id != r.workspace_id || reference.expected_revision != r.revision
        }) {
            return Err(invalid(
                "workspace_stale_revision: script must use the current writer and workspace revision",
            ));
        }
        Ok(Some(r))
    }
    pub fn verify_ready(r: &Record, source: &dyn DataSource) -> Result<()> {
        git::verify(r)?;
        let task = source
            .resolve(d::ResolveRequest {
                locator: r.task_ref.id.clone(),
            })
            .and_then(|reference| source.fetch(d::FetchRequest { reference }))
            .map_err(|e| invalid(e.to_string()))?;
        if task.etag != r.task_etag && task_contract(&task)? != r.task_contract_digest {
            return Err(invalid(
                "workspace_task_changed: source brief, verify or allowed paths changed; read the amended task before another attempt",
            ));
        }
        Ok(())
    }
    pub fn reserve(&self, r: &Record, seat_id: Ulid, op: Ulid) -> Result<()> {
        self.update(r.workspace_id, |current| {
            if current.revision != r.revision
                || current.state != State::Ready
                || current.holder.is_some()
            {
                return Err(invalid(
                    "workspace_in_use: another launch reserved the writer",
                ));
            }
            current.revision += 1;
            current.state = State::InUse;
            current.holder = Some(Holder {
                seat_id,
                generation: 1,
                launch_operation: op,
                pid: None,
                launch_requested: false,
            });
            Ok(())
        })
    }
    /// All native launch doors preserve a managed seat's writing lease across
    /// a proven restart. An unknown/possibly live old process never transfers.
    pub fn prepare_launch(
        &self,
        seat: Ulid,
        generation: u64,
        swarm: Ulid,
        cwd: &Path,
        registry: &Registry,
    ) -> Result<Option<Ulid>> {
        let Some(bytes) = self.store.get(&format!("holder-seat/{seat}"))? else {
            if cwd.canonicalize()?.starts_with(&self.parent) {
                return Err(invalid(
                    "workspace_use_handle: managed checkouts require workspace {id, expected_revision}; raw cwd cannot bypass the writer lease",
                ));
            }
            return Ok(None);
        };
        let id = String::from_utf8(bytes)?.parse::<Ulid>()?;
        let record=self.update(id,|r| {
            let h=r.holder.as_ref().filter(|h|h.seat_id==seat).ok_or_else(||invalid("workspace_stale_holder: closed lease cannot be restored through an unmanaged launch"))?;
            let swarm_revision=registry.swarm(swarm)?.map_or(0,|s|s.revision);
            if r.swarm_id!=swarm || r.knowledge_binding.revision!=swarm_revision || r.state!=State::InUse || !cwd.canonicalize()?.starts_with(&r.cwd) || r.checkout_identity!=Some(git::identity(&r.cwd)?) {
                return Err(invalid("workspace_binding_changed: restart source/KB identity changed"));
            }
            if generation==h.generation && h.pid.is_none(){return Ok(r.workspace_id);}
            if generation<=h.generation || h.pid.is_none_or(git::process_exists) {
                return Err(invalid("workspace_in_use: previous writer must be proven dead before generation transfer"));
            }
            r.holder.as_mut().unwrap().generation=generation;r.holder.as_mut().unwrap().pid=None;r.holder.as_mut().unwrap().launch_requested=false;r.revision+=1;
            Ok(r.workspace_id)
        })?;
        Ok(Some(record))
    }
    pub fn launched(&self, id: Ulid, seat: Ulid, generation: u64, pid: u32) -> Result<()> {
        self.update(id, |r| {
            if r.state != State::InUse {
                return Err(invalid("workspace_stale_holder"));
            }
            let h = r
                .holder
                .as_mut()
                .filter(|h| h.seat_id == seat && h.generation == generation)
                .ok_or_else(|| invalid("workspace_stale_holder"))?;
            if h.pid.is_some_and(|old| old != pid) {
                return Err(invalid(
                    "workspace_stale_holder: confirmed process identity changed",
                ));
            }
            if h.pid.is_none() {
                h.pid = Some(pid);
                h.launch_requested = true;
                r.revision += 1;
            }
            Ok(())
        })
    }
    /// Persist the uncertainty boundary before sending the native spawn request.
    /// Earlier planning failures may relinquish a reservation; later failures
    /// retain it until the exact supervisor-owned seat is observed.
    pub fn request_launch(&self, id: Ulid, seat: Ulid, generation: u64) -> Result<()> {
        self.update(id, |r| {
            let h = r
                .holder
                .as_mut()
                .filter(|h| h.seat_id == seat && h.generation == generation)
                .ok_or_else(|| invalid("workspace_stale_holder"))?;
            if r.state != State::InUse || h.pid.is_some() || h.launch_requested {
                return Err(invalid(
                    "workspace_in_use: launch already requested; reconcile its original operation",
                ));
            }
            h.launch_requested = true;
            Ok(())
        })
    }
    pub fn cancel_unstarted_launch(&self, id: Ulid, seat: Ulid, op: Ulid) -> Result<bool> {
        self.update(id, |r| {
            if r.state != State::InUse
                || !r.holder.as_ref().is_some_and(|h| {
                    h.seat_id == seat
                        && h.launch_operation == op
                        && h.pid.is_none()
                        && !h.launch_requested
                })
            {
                return Ok(false);
            }
            r.holder = None;
            r.state = State::Ready;
            r.revision += 1;
            Ok(true)
        })
    }
    pub fn observe_launch(&self, seat: Ulid, generation: u64, pid: u32) -> Result<()> {
        if let Some(bytes) = self.store.get(&format!("holder-seat/{seat}"))? {
            self.launched(String::from_utf8(bytes)?.parse()?, seat, generation, pid)?;
        }
        Ok(())
    }
    pub fn delegation(
        &self,
        target: Ulid,
        generation: u64,
        task: &str,
        root: &RootBinding,
    ) -> Result<()> {
        let Some(bytes) = self.store.get(&format!("holder/{target}/{generation}"))? else {
            return Ok(());
        };
        let r = self.get(String::from_utf8(bytes)?.parse()?)?;
        if r.state != State::InUse
            || !r.holder.as_ref().is_some_and(|h| {
                h.seat_id == target && h.generation == generation && h.pid.is_some()
            })
        {
            return Err(invalid(
                "workspace_in_use: task delegation requires the exact confirmed managed writer; reconcile the original launch first",
            ));
        }
        if r.knowledge_binding != *root || r.task_ref.id != task {
            return Err(invalid(
                "workspace_task_mismatch: target is bound to a different task or knowledge root",
            ));
        }
        Ok(())
    }
}
fn key(id: Ulid) -> String {
    format!("workspace/{id}")
}
pub(crate) fn failure(op: Ulid, error: &(dyn std::error::Error + 'static)) -> Value {
    let text = error.to_string();
    let code = text
        .split_once(':')
        .map_or("workspace_unavailable", |(code, _)| code);
    super::error(op, code, &text)
}

fn task_contract(record: &d::Record) -> Result<String> {
    use sha2::{Digest, Sha256};
    let fields = json!({"body":record.body,"title":record.frontmatter["title"],"type":record.frontmatter["type"],"role":record.frontmatter["role"],"allowed_paths":record.frontmatter["allowed_paths"],"verify":record.frontmatter["verify"]});
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&fields)?)
    ))
}
