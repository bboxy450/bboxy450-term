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
use std::{collections::VecDeque, sync::Arc};

#[derive(Debug, Clone)]
pub enum Read {
    Resolve(String),
    Fetch(d::RecordRef),
}
impl Read {
    pub fn action(&self, id: Ulid) -> bbt_proto::GuiAction {
        let (method, args) = match self {
            Self::Resolve(locator) => ("resolve", serde_json::json!({"locator":locator})),
            Self::Fetch(reference) => ("fetch", serde_json::json!({"ref":reference})),
        };
        bbt_proto::GuiAction::Knowledge {
            id,
            method: method.into(),
            args,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Prepared {
    pub reference: d::RecordRef,
    pub markdown: bbt_doc::Markdown,
    pub document: Document,
    pub targets: Vec<String>,
}
#[derive(Debug, Clone)]
pub enum Response {
    Resolved(d::RecordRef),
    Fetched(Arc<Prepared>),
    Failed(String),
}
#[derive(Debug, Clone)]
pub struct Reply {
    pub id: Ulid,
    pub response: Response,
}
fn decode(read: &Read, value: d::Result<Value>) -> Response {
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
            Read::Fetch(expected) => {
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
                    markdown,
                    document,
                    targets,
                })))
            }
        }
    })();
    result.unwrap_or_else(|error| Response::Failed(error.to_string()))
}

/// One result is retained until consumed, even if Latest replaces a frame.
#[derive(Default)]
pub struct Reader {
    pending: Option<(Ulid, Read)>,
}
pub struct Work {
    id: Ulid,
    read: Read,
    value: d::Result<Value>,
}
impl Work {
    pub fn prepare(self) -> Arc<Reply> {
        bbt_rt::assert_not_ui_thread();
        Arc::new(Reply {
            id: self.id,
            response: decode(&self.read, self.value),
        })
    }
}
impl Reader {
    pub fn admit(&mut self, id: Ulid, read: Read) -> Option<bbt_proto::GuiAction> {
        if self.pending.is_some() {
            return None;
        }
        let action = read.action(id);
        self.pending = Some((id, read));
        Some(action)
    }
    pub fn response(&mut self, id: Ulid, value: d::Result<Value>) -> Option<Work> {
        if self.pending.as_ref()?.0 != id {
            return None;
        }
        let (_, read) = self.pending.take()?;
        Some(Work { id, read, value })
    }
    pub fn disconnected(&mut self, reason: &str) -> Option<Arc<Reply>> {
        let (id, _) = self.pending.take()?;
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
    pending: Option<(Ulid, Purpose)>,
}
impl Host {
    pub fn show(&mut self, target: String) {
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
            .push_back((Read::Resolve(target), Purpose::Open(self.epoch)));
    }
    pub fn close(&mut self) {
        self.epoch = self.epoch.wrapping_add(1);
        self.open = false;
        self.queued.clear();
    }
    pub fn next_read(&mut self) -> Option<(Ulid, Read)> {
        if self.pending.is_some() {
            return None;
        }
        let (read, purpose) = self.queued.pop_front()?;
        let id = Ulid::new();
        self.pending = Some((id, purpose));
        Some((id, read))
    }
    pub fn apply(&mut self, reply: &Reply) -> bool {
        if self.pending.as_ref().map(|p| p.0) != Some(reply.id) {
            return false;
        }
        let (_, purpose) = self.pending.take().expect("matching pending read");
        let epoch = match &purpose {
            Purpose::Open(e) | Purpose::Fetch(e) | Purpose::Probe(e, _) => *e,
        };
        if epoch != self.epoch || !self.open {
            return true;
        }
        match (purpose, &reply.response) {
            (Purpose::Open(e), Response::Resolved(reference)) => self
                .queued
                .push_front((Read::Fetch(reference.clone()), Purpose::Fetch(e))),
            (Purpose::Fetch(e), Response::Fetched(prepared)) => {
                let targets = &prepared.targets;
                if targets.len() > 128 {
                    self.notice=Some("link_resolution_limit: first 128 links resolved; remaining links resolve when opened".into());
                }
                for target in targets.iter().take(128) {
                    self.queued.push_back((
                        Read::Resolve(target.clone()),
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
        let changed = self.error.as_deref() != Some(reason);
        if changed {
            self.epoch = self.epoch.wrapping_add(1);
        }
        self.queued.clear();
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
