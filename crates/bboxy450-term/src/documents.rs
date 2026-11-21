// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Document reads use the existing bounded Knowledge door. Parsing belongs to
//! IO; the window owns only navigation, retained layout and scroll.
use bbt_mux::{Ulid, knowledge::data_source as d};
use bbt_ui::modules::document_modal::{
    Document,
    controller::{Controller, RecordLinks},
};
use serde_json::Value;
pub mod observation;
pub mod images;
use std::{
    collections::{BTreeSet, VecDeque},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub enum Read {
    Resolve(String),
    Fetch(d::RecordRef),
    Artifact {
        id: Ulid,
        revision: Option<u64>,
        preview: bool,
    },
}
impl Read {
    fn target(target: String, preview: bool) -> Self {
        if let Some((id, revision)) = target
            .strip_prefix("artifact:")
            .and_then(bboxy450_canvas::artifact_address)
        {
            Self::Artifact {
                id,
                revision,
                preview,
            }
        } else {
            Self::Resolve(target)
        }
    }
    pub fn action(&self, id: Ulid) -> bbt_proto::GuiAction {
        self.action_scoped(id, None)
    }
    pub fn action_scoped(
        &self,
        id: Ulid,
        scope: Option<bbt_proto::gui::SwarmScope>,
    ) -> bbt_proto::GuiAction {
        let (method, args) = match self {
            Self::Resolve(locator) => ("resolve", serde_json::json!({"locator":locator})),
            Self::Fetch(reference) => ("fetch", serde_json::json!({"ref":reference})),
            Self::Artifact { id, revision, .. } => (
                "artifact_image",
                serde_json::json!({"id":id,"revision":revision}),
            ),
        };
        bbt_proto::GuiAction::Knowledge {
            id,
            scope,
            method: method.into(),
            args,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Prepared {
    pub reference: d::RecordRef,
    pub etag: Option<String>,
    pub markdown: bbt_doc::Markdown,
    pub document: Document,
    pub targets: Vec<String>,
    pub follows_artifact: bool,
    pub images: Vec<Arc<bbt_render::chrome::icons::Image>>,
}
/// Flat router evidence is retained before portable provider DTO parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Source {
    pub scope: bbt_proto::gui::SwarmScope,
    pub root: String,
    pub provider_revision: u64,
}
impl Source {
    pub(crate) fn decode(value: &Value, scope: bbt_proto::gui::SwarmScope) -> Result<Self, String> {
        let root = value["knowledge_root_id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or("document_source_missing: knowledge root")?;
        let swarm = value["swarm_workspace_id"]
            .as_str()
            .and_then(|s| s.parse::<Ulid>().ok());
        let revision = value["swarm_workspace_revision"].as_u64();
        if swarm != Some(scope.swarm) || revision != Some(scope.revision) {
            return Err(
                "document_scope_mismatch: provider result belongs to another workspace".into(),
            );
        }
        let provider_revision = value["knowledge_root_revision"]
            .as_u64()
            .ok_or("document_source_missing: provider revision")?;
        Ok(Self {
            scope,
            root: root.into(),
            provider_revision,
        })
    }
}
#[derive(Debug, Clone)]
pub enum Response {
    Resolved(d::RecordRef),
    Fetched(Arc<Prepared>),
    Failed(String),
    Retry {
        after_ms: u64,
    },
    Bound {
        source: Source,
        response: Box<Response>,
    },
}
impl Response {
    pub fn status(&self) -> &'static str {
        match self {
            Self::Resolved(_) => "resolved",
            Self::Fetched(_) => "loaded",
            Self::Failed(_) => "refused",
            Self::Retry { .. } => "pending",
            Self::Bound { response, .. } => response.status(),
        }
    }
}
#[derive(Debug, Clone)]
pub struct Reply {
    pub id: Ulid,
    pub response: Response,
}
fn decode(read: &Read, value: d::Result<Value>, host: Option<&ImageHost>) -> Response {
    let result = (|| {
        let value = value?;
        if serde_json::to_vec(&d::Result::Ok(&value))
            .map_err(|e| d::CommandError::new("invalid_provider_result", e.to_string()))?
            .len()
            > d::MAX_PAGE_BYTES
        {
            return Err(d::CommandError::new(
                "response_too_large",
                "document result exceeds 256 KiB",
            ));
        }
        match read {
            Read::Resolve(_) => serde_json::from_value(value)
                .map(Response::Resolved)
                .map_err(|e| d::CommandError::new("invalid_provider_result", e.to_string())),
            Read::Artifact {
                id,
                revision,
                preview,
            } => artifact_preview(value, *id, *revision, *preview, host),
            Read::Fetch(expected) => {
                let etag = value.get("etag").and_then(Value::as_str).map(str::to_owned);
                let record: d::Record = serde_json::from_value(value)
                    .map_err(|e| d::CommandError::new("invalid_provider_result", e.to_string()))?;
                if record.reference != *expected {
                    return Err(d::CommandError::new(
                        "stale_reference",
                        "fetched record changed; open it again",
                    ));
                }
                let mut markdown = bbt_doc::parse(&record.body);
                markdown
                    .set_metadata(&record.frontmatter)
                    .map_err(|e| d::CommandError::new("invalid_frontmatter", e.to_string()))?;
                markdown.frontmatter.error = record.parse_error;
                let (images, follows_artifact) =
                    host.map_or_else(Default::default, |host| host.load(&mut markdown));
                let mut document = markdown.document(&record.reference.path);
                document.prov.digest = record.reference.digest.clone();
                if let Some(label) = record.frontmatter["authorship"]["issued_by"]
                    .as_str()
                    .filter(|value| value.starts_with("test-account:"))
                {
                    let updater = document.prov.updated_by.get_or_insert_with(String::new);
                    if !updater.is_empty() {
                        updater.push_str(" · ");
                    }
                    updater.push_str(label);
                }
                // Metadata alone never proves cryptographic verification.
                document.prov.signed = None;
                let targets = markdown.link_targets();
                Ok(Response::Fetched(Arc::new(Prepared {
                    reference: record.reference,
                    etag,
                    markdown,
                    document,
                    targets,
                    follows_artifact,
                    images,
                })))
            }
        }
    })();
    result.unwrap_or_else(|error| {
        if error.code == "workspace_pending" {
            Response::Retry {
                after_ms: error.retry_after_ms.unwrap_or(100).clamp(25, 1000),
            }
        } else {
            Response::Failed(error.to_string())
        }
    })
}

fn artifact_preview(
    value: Value,
    id: Ulid,
    revision: Option<u64>,
    preview: bool,
    host: Option<&ImageHost>,
) -> d::Result<Response> {
    use bbt_mux::artifact_images::{Images, Resolved};
    let invalid = |message: &str| d::CommandError::new("invalid_provider_result", message);
    let found: Resolved =
        serde_json::from_value(value).map_err(|error| invalid(&error.to_string()))?;
    if found.id != id || revision.is_some_and(|expected| found.revision != expected) {
        return Err(invalid(
            "artifact address disagrees with the resolved revision",
        ));
    }
    let path = found.reference();
    let reference = d::RecordRef {
        id: id.to_string(),
        number: None,
        slug: path.clone(),
        path: path.clone(),
        digest: found.digest.clone(),
        external_id: None,
        updated_at: None,
    };
    if !preview {
        return Ok(Response::Resolved(reference));
    }
    let (port, _) = host
        .and_then(|host| host.artifacts.as_ref())
        .ok_or_else(|| invalid("artifact preview requires a bound workspace"))?;
    let mut body = format!("# {path}\n\n{} · {} bytes\n", found.media_type, found.bytes);
    let mut images = Images::default();
    let image = matches!(found.media_type.as_str(), "image/png" | "image/svg+xml");
    if image {
        body.push_str(&format!("\n![[{path}]]\n"));
    }
    let mut markdown = bbt_doc::parse(&body);
    if image {
        images::load_targets(&mut markdown, |target| {
            images.prepare(target, |remaining| port.prepare(&found, remaining))
        });
    } else if found.bytes <= d::MAX_PAGE_BYTES
        && (found.media_type.starts_with("text/") || found.media_type == "application/json")
    {
        let bytes = port
            .bytes(&found)
            .map_err(|error| d::CommandError::new("artifact_unavailable", error.to_string()))?;
        if let Ok(code) = String::from_utf8(bytes) {
            let end = markdown.source.len();
            markdown.blocks.push(bbt_doc::Block {
                source: end..end,
                kind: bbt_doc::BlockKind::Code {
                    info: String::new(),
                    text: code,
                },
            });
        }
    }
    let mut document = markdown.document(&path);
    document.prov.digest = found.digest;
    Ok(Response::Fetched(Arc::new(Prepared {
        reference,
        etag: None,
        markdown,
        document,
        targets: Vec::new(),
        follows_artifact: revision.is_none(),
        images: images.into_rasters(),
    })))
}

#[derive(Clone)]
struct ImageHost {
    directory: std::path::PathBuf,
    artifacts: Option<(Arc<images::Port>, bbt_proto::gui::SwarmScope)>,
}

impl ImageHost {
    fn load(
        &self,
        markdown: &mut bbt_doc::Markdown,
    ) -> (Vec<Arc<bbt_render::chrome::icons::Image>>, bool) {
        images::load_for_host(
            markdown,
            &self.directory,
            self.artifacts
                .as_ref()
                .map(|(port, scope)| (port.as_ref(), *scope)),
        )
    }
}

/// One result is retained until consumed, even if Latest replaces a frame.
#[derive(Default)]
pub struct Reader {
    pending: Option<(
        Ulid,
        Read,
        Option<bbt_proto::gui::SwarmScope>,
        Option<ImageHost>,
    )>,
    outstanding: BTreeSet<Ulid>,
    image_host: Option<ImageHost>,
}
pub struct Work {
    id: Ulid,
    read: Read,
    value: d::Result<Value>,
    image_host: Option<ImageHost>,
    scope: Option<bbt_proto::gui::SwarmScope>,
}
impl Work {
    pub fn prepare(self) -> Arc<Reply> {
        bbt_rt::assert_not_ui_thread();
        let source = self.scope.and_then(|scope| {
            self.value
                .as_ref()
                .ok()
                .map(|value| Source::decode(value, scope))
        });
        let response = match source {
            Some(Ok(source)) => Response::Bound {
                source,
                response: Box::new(decode(&self.read, self.value, self.image_host.as_ref())),
            },
            Some(Err(reason)) => Response::Failed(reason),
            None => decode(&self.read, self.value, self.image_host.as_ref()),
        };
        Arc::new(Reply {
            id: self.id,
            response,
        })
    }
}
impl Reader {
    pub fn image_root(&mut self, directory: std::path::PathBuf) {
        self.image_host(directory, None);
    }

    pub fn image_host(
        &mut self,
        directory: std::path::PathBuf,
        artifacts: Option<(Arc<images::Port>, bbt_proto::gui::SwarmScope)>,
    ) {
        self.image_host = Some(ImageHost {
            directory,
            artifacts,
        });
    }
    pub fn admit(&mut self, id: Ulid, read: Read) -> Option<bbt_proto::GuiAction> {
        self.admit_scoped(id, read, None)
    }
    pub fn admit_scoped(
        &mut self,
        id: Ulid,
        read: Read,
        scope: Option<bbt_proto::gui::SwarmScope>,
    ) -> Option<bbt_proto::GuiAction> {
        if self.pending.is_some() {
            return None;
        }
        self.admit_latest_scoped(id, read, scope).ok()
    }
    /// Read-only navigation may abandon a held provider without closing the shared
    /// GUI door. Retired replies only release bounded accounting, never repaint.
    pub fn admit_latest_scoped(
        &mut self,
        id: Ulid,
        read: Read,
        scope: Option<bbt_proto::gui::SwarmScope>,
    ) -> Result<bbt_proto::GuiAction, &'static str> {
        self.pending = None;
        if self.outstanding.len() >= 8 {
            return Err(
                "document_read_limit: eight previous reads are still outstanding; reopen after they finish",
            );
        }
        let action = read.action_scoped(id, scope);
        self.outstanding.insert(id);
        self.pending = Some((id, read, scope, self.image_host.take()));
        Ok(action)
    }
    pub fn response(&mut self, id: Ulid, value: d::Result<Value>) -> Option<Work> {
        self.outstanding.remove(&id);
        if self.pending.as_ref()?.0 != id {
            return None;
        }
        let (_, read, scope, image_host) = self.pending.take()?;
        Some(Work {
            id,
            read,
            value,
            image_host,
            scope,
        })
    }
    pub fn disconnected(&mut self, reason: &str) -> Option<Arc<Reply>> {
        self.outstanding.clear();
        let (id, _, _, _) = self.pending.take()?;
        Some(Arc::new(Reply {
            id,
            response: Response::Failed(format!("document_read_disconnected: {reason}")),
        }))
    }
}

#[derive(Debug, Clone)]
enum Purpose {
    Open(u64),
    Fetch(u64),
    Probe(u64, String),
}
/// Window navigation is single-flight. Reopening or closing while reading
/// invalidates the result, but still drains it; stale reads never change focus.
#[derive(Default)]
pub struct Host {
    pub open: bool,
    pub target: String,
    pub prepared: Option<Arc<Prepared>>,
    pub error: Option<String>,
    pub notice: Option<String>,
    pub links: RecordLinks,
    pub scroll: Controller,
    epoch: u64,
    queued: VecDeque<(Read, Purpose)>,
    pending: Option<(Ulid, Read, Purpose)>,
    scope: Option<bbt_proto::gui::SwarmScope>,
    source: Option<Source>,
    retry_blocked: bool,
    retry_delay: Option<std::time::Duration>,
    retry_started: Option<std::time::Instant>,
    retries: u8,
}
impl Host {
    /// Failed GPU custody leaves readable source placeholders, never an app-fatal draw.
    pub fn image_failed(&mut self, reason: String) {
        if let Some(prepared)=&mut self.prepared {
            let prepared=Arc::make_mut(prepared);
            images::clear(&mut prepared.markdown);
            prepared.images.clear();
        }
        self.notice=Some(format!("Image unavailable: {reason}"));
    }
    /// Deliberate startup-root path for legacy CLI/test callers.
    pub fn show(&mut self, target: String) {
        self.show_scoped(target, None)
    }
    pub fn show_scoped(&mut self, target: String, scope: Option<bbt_proto::gui::SwarmScope>) {
        self.scope = scope;
        self.pending = None;
        self.source = None;
        self.retry_blocked = false;
        self.retry_delay = None;
        self.retry_started = None;
        self.retries = 0;
        self.epoch = self.epoch.wrapping_add(1);
        self.open = true;
        self.target = target.clone();
        self.prepared = None;
        self.error = None;
        self.notice = None;
        self.links.clear();
        self.scroll = Controller::default();
        self.queued.clear();
        self.queued
            .push_back((Read::target(target, true), Purpose::Open(self.epoch)));
    }
    pub fn artifact_changed(&mut self) -> bool {
        if !self.open
            || self.scope.is_none()
            || self.error.is_some()
            || self
                .prepared
                .as_ref()
                .is_some_and(|prepared| !prepared.follows_artifact)
        {
            return false;
        }
        let scroll = std::mem::take(&mut self.scroll);
        self.show_scoped(self.target.clone(), self.scope);
        self.scroll = scroll;
        true
    }
    pub fn follow(&mut self, target: String) {
        let source = self.source.clone();
        let scope = self.scope;
        self.show_scoped(target, scope);
        self.source = source;
    }
    pub fn scope(&self) -> Option<bbt_proto::gui::SwarmScope> {
        self.scope
    }
    pub fn source(&self) -> Option<&Source> {
        self.source.as_ref()
    }
    /// A ready, current provider result, never a reference inferred from display.
    pub fn observation_source(&self) -> Option<bbt_proto::gui::UiDocumentSource> {
        if !self.open
            || self.error.is_some()
            || self.notice.is_some()
            || self.pending.is_some()
            || !self.queued.is_empty()
        {
            return None;
        }
        let source = self.source.as_ref()?;
        if self.scope != Some(source.scope) {
            return None;
        }
        let prepared = self.prepared.as_ref()?;
        let etag = prepared.etag.as_deref()?;
        for (value, bound) in [
            (prepared.reference.id.as_str(), 128),
            (prepared.reference.path.as_str(), 1024),
            (prepared.reference.digest.as_str(), 128),
            (etag, 128),
            (source.root.as_str(), 128),
        ] {
            if value.is_empty() || value.len() > bound || value.contains('\0') {
                return None;
            }
        }
        Some(bbt_proto::gui::UiDocumentSource {
            record_id: prepared.reference.id.clone(),
            number: prepared.reference.number,
            path: prepared.reference.path.clone(),
            digest: prepared.reference.digest.clone(),
            etag: etag.to_owned(),
            workspace: source.scope,
            root: source.root.clone(),
            provider_revision: source.provider_revision,
        })
    }
    pub fn take_retry(&mut self) -> Option<(u64, std::time::Duration)> {
        self.retry_delay.take().map(|delay| (self.epoch, delay))
    }
    pub fn retry_ready(&mut self, epoch: u64) -> bool {
        if epoch == self.epoch && self.open && self.retry_blocked {
            self.retry_blocked = false;
            true
        } else {
            false
        }
    }
    pub fn close(&mut self) {
        self.pending = None;
        self.retry_blocked = false;
        self.retry_delay = None;
        self.epoch = self.epoch.wrapping_add(1);
        self.open = false;
        self.queued.clear();
    }
    pub fn next_read(&mut self) -> Option<(Ulid, Read)> {
        if self.pending.is_some() || self.retry_blocked {
            return None;
        }
        let (read, purpose) = self.queued.pop_front()?;
        let id = Ulid::new();
        self.pending = Some((id, read.clone(), purpose));
        Some((id, read))
    }
    pub fn apply(&mut self, reply: &Reply) -> bool {
        if self.pending.as_ref().map(|p| p.0) != Some(reply.id) {
            return false;
        }
        let (_, read, purpose) = self.pending.take().expect("matching pending read");
        let epoch = match &purpose {
            Purpose::Open(e) | Purpose::Fetch(e) | Purpose::Probe(e, _) => *e,
        };
        if epoch != self.epoch || !self.open {
            return true;
        }
        let (response, source) = match &reply.response {
            Response::Bound { source, response } => (response.as_ref(), Some(source)),
            response => (response, None),
        };
        if let Response::Retry { after_ms } = response {
            let now = std::time::Instant::now();
            let began = self.retry_started.get_or_insert(now);
            if self.retries >= 50
                || now.saturating_duration_since(*began) >= std::time::Duration::from_secs(10)
            {
                self.unavailable(
                    "workspace_pending: preparing the workspace exceeded its retry bound",
                );
            } else {
                self.retries += 1;
                self.retry_blocked = true;
                self.retry_delay = Some(std::time::Duration::from_millis(*after_ms));
                self.queued.push_front((read, purpose));
                self.notice = Some("Preparing workspace…".into());
            }
            return true;
        }
        if let Some(scope) = self.scope
            && matches!(response, Response::Resolved(_) | Response::Fetched(_))
        {
            let valid = source.is_some_and(|source| {
                source.scope == scope && self.source.as_ref().is_none_or(|old| old == source)
            });
            if !valid {
                self.unavailable("document_source_changed: record replies no longer belong to the captured workspace/provider");
                return true;
            }
            if self.source.is_none() {
                self.source = source.cloned();
            }
        }
        match (purpose, response) {
            (Purpose::Open(e), Response::Resolved(reference)) => self
                .queued
                .push_front((Read::Fetch(reference.clone()), Purpose::Fetch(e))),
            (Purpose::Fetch(e) | Purpose::Open(e), Response::Fetched(prepared)) => {
                let targets = &prepared.targets;
                if targets.len() > 128 {
                    self.notice=Some("link_resolution_limit: first 128 links resolved; remaining links resolve when opened".into());
                }
                for target in targets.iter().take(128) {
                    self.queued.push_back((
                        Read::target(target.clone(), false),
                        Purpose::Probe(e, target.clone()),
                    ));
                }
                self.prepared = Some(prepared.clone());
            }
            (Purpose::Probe(_, target), Response::Resolved(reference)) => {
                self.links.insert(target, reference.number);
            }
            (Purpose::Probe(_, target), Response::Failed(reason)) => {
                if reason.starts_with("not_found:") || reason.starts_with("missing:") {
                    self.links.insert(target, None);
                } else {
                    self.notice = Some(format!("link resolution: {reason}"));
                }
            }
            (_, Response::Failed(reason)) => self.error = Some(reason.clone()),
            _ => self.error = Some("invalid_provider_result: unexpected document reply".into()),
        }
        true
    }
    pub fn unavailable(&mut self, reason: &str) -> bool {
        self.pending = None;
        let changed = self.error.as_deref() != Some(reason);
        if changed {
            self.epoch = self.epoch.wrapping_add(1);
        }
        self.queued.clear();
        self.prepared = None;
        self.links.clear();
        self.retry_blocked = false;
        self.retry_delay = None;
        self.error = Some(reason.into());
        changed
    }
    pub fn settled(&self) -> bool {
        self.error.is_some() || (self.pending.is_none() && self.queued.is_empty())
    }
    pub fn view(&self) -> Document {
        self.prepared
            .as_ref()
            .map(|p| p.document.clone())
            .unwrap_or_else(|| Document {
                path: self.target.clone(),
                title: self.target.clone(),
                body: if self.error.is_some() {
                    "Open the record again to retry.".into()
                } else {
                    "Loading record…".into()
                },
                ..Default::default()
            })
    }
}
