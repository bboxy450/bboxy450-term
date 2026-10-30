// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Host API for plugins (G7): JSON-RPC requests a plugin sends on its own
//! stdio, `host/<method>`, answered on that connection.
//!
//! - `host/records.search`, `host/records.fetch`: the data_source `search`
//!   and `fetch` request/result of the plugin's own knowledge root, read
//!   through that root's selected source.
//! - `host/events.publish` `{subject, payload?}`: published on the account
//!   stream as `plugin/<candidate>/<subject>`, source `plugin:<candidate>`.
//!   The subject is always a suffix under the plugin's own prefix.
//! - `host/messages.send` `{destination, text, id?, priority?}`: one plain
//!   text message to a destination in the record's `messages_to` grant,
//!   signed by the host as `plugin:<candidate>` (never a seat or operator
//!   key); returns `{id, destination}`.
//!
//! Every request rechecks the connection's authority (enabled, checked,
//! same activation generation and trust record) and requires the method in
//! both the trusted shape's `host_api` and the operator's `host_api` grant;
//! otherwise the JSON-RPC error carries `data.code` `provider_changed` or
//! `host_api_not_granted`. The reader serves requests on separate tasks
//! (client.rs), at most `MAX_HOST_CALLS` per connection within
//! `HOST_CALL_BUDGET`, so a request made while the plugin is serving a host
//! call cannot deadlock the connection. Modelled by `PluginHostApi`.
use super::*;
use crate::plugins::client::HostHandler;
use crate::store::events::Draft;
use bbt_plugin::{policy::validate_subject, shape::HOST_API};
use std::sync::Weak;

/// Reads the root's records: `(method, params)` with method
/// `records.search` or `records.fetch`, data_source request and result JSON.
/// Installed by the workspace mount; blocking IO.
pub type RecordReader = Arc<dyn Fn(&str, Value) -> d::Result<Value> + Send + Sync>;

/// Posts one host-signed plugin message (`messaging::Service::send_plugin`)
/// and returns `{id, destination}`. Installed by the logic; blocking IO.
pub type MessageSender = Arc<
    dyn Fn(
            crate::messaging::service::PluginSender,
            crate::messaging::wire::SendRequest,
            Instant,
        ) -> d::Result<Value>
        + Send
        + Sync,
>;

/// Largest published payload.
const MAX_PAYLOAD_BYTES: usize = 64 * 1024;

/// The handler for one connection, fixed to the authority it opened under.
pub(super) fn handler(host: Weak<Inner>, candidate: String, opened: Authority) -> HostHandler {
    Arc::new(move |method, params, deadline| {
        let inner = host
            .upgrade()
            .ok_or_else(|| Refusal::new("provider_closed", "this root's plugin host is closed"))?;
        PluginHost(inner)
            .serve_host_api(&candidate, &opened, method, params, deadline)
            .map_err(|reason| Refusal::new(reason.code, reason.message))
    })
}

impl PluginHost {
    /// Serve `records.search` and `records.fetch` from `reader`.
    pub fn attach_records(&self, reader: RecordReader) {
        if let Ok(mut held) = self.0.records.lock() {
            *held = Some(reader);
        }
    }

    /// Serve `messages.send` through `sender`.
    pub fn attach_messages(&self, sender: MessageSender) {
        if let Ok(mut held) = self.0.messages.lock() {
            *held = Some(sender);
        }
    }

    /// `messages.send {destination, text, id?, priority?}`: one plain text
    /// message to a destination in the record's `messages_to` grant, signed
    /// by the host as `plugin:<candidate>`.
    fn send_message(
        &self,
        candidate: &str,
        current: &Authority,
        params: Value,
        deadline: Instant,
    ) -> d::Result<Value> {
        let field = |name: &str| {
            params[name]
                .as_str()
                .ok_or_else(|| error("invalid_request", format!("{name} must be a string")))
        };
        let mut request =
            crate::messaging::wire::SendRequest::text(field("destination")?, field("text")?);
        if let Some(id) = params.get("id").filter(|id| !id.is_null()) {
            request.id = id
                .as_str()
                .and_then(|id| id.parse().ok())
                .ok_or_else(|| error("invalid_request", "id must be a ULID"))?;
        }
        if let Some(priority) = params.get("priority").filter(|p| !p.is_null()) {
            request.priority = priority
                .as_u64()
                .and_then(|p| u8::try_from(p).ok())
                .ok_or_else(|| error("invalid_request", "priority must be 0..=255"))?;
        }
        let sender = crate::messaging::service::PluginSender {
            candidate: candidate.to_owned(),
            name: current
                .record
                .shape
                .as_ref()
                .map_or_else(|| candidate.to_owned(), |shape| shape.name.clone()),
            generation: current.activation.generation,
            sha256: current.record.sha256.clone(),
            granted: current.record.grants.messages_to.clone(),
        };
        let send = self
            .0
            .messages
            .lock()
            .ok()
            .and_then(|sender| sender.clone())
            .ok_or_else(|| error("host_api_unavailable", "this root sends no messages"))?;
        send(sender, request, deadline)
    }

    fn serve_host_api(
        &self,
        candidate: &str,
        opened: &Authority,
        method: &str,
        params: Value,
        deadline: Instant,
    ) -> d::Result<Value> {
        let name = method
            .strip_prefix("host/")
            .filter(|name| HOST_API.iter().any(|(known, _)| known == name))
            .ok_or_else(|| {
                error(
                    "unknown_host_api",
                    format!("{method} is not a host API method"),
                )
            })?;
        let (current, _) = self.authorize(candidate)?;
        if current != *opened {
            return Err(error(
                "provider_changed",
                "plugin authority changed since this connection opened",
            ));
        }
        if !current.record.host_api_granted(name) {
            return Err(error(
                "host_api_not_granted",
                format!("{name} was not requested and granted at trust"),
            ));
        }
        if Instant::now() >= deadline {
            return Err(error("deadline_exceeded", "host request budget exhausted"));
        }
        match name {
            "events.publish" => self.publish(candidate, params, deadline),
            "messages.send" => self.send_message(candidate, &current, params, deadline),
            _ => self.read_records(name, params),
        }
    }

    fn read_records(&self, name: &str, params: Value) -> d::Result<Value> {
        let method = name.trim_start_matches("records.");
        let contract = Contract::DataSource;
        let schema = |result| {
            contract
                .method_schema(method, result)
                .ok_or_else(|| error("unknown_host_api", name))
        };
        validate(&schema(false)?, &params, "invalid_request")?;
        let reader = self
            .0
            .records
            .lock()
            .ok()
            .and_then(|reader| reader.clone())
            .ok_or_else(|| error("host_api_unavailable", "this root serves no records"))?;
        let value = reader(name, params)?;
        if serde_json::to_vec(&value).map_or(true, |bytes| bytes.len() > d::MAX_PAGE_BYTES) {
            return Err(error(
                "response_too_large",
                "records result exceeds the portable page limit",
            ));
        }
        validate(&schema(true)?, &value, "invalid_provider_result")?;
        Ok(value)
    }

    fn publish(&self, candidate: &str, params: Value, deadline: Instant) -> d::Result<Value> {
        let suffix = params["subject"]
            .as_str()
            .ok_or_else(|| error("invalid_request", "subject must be a string"))?;
        let payload = match params.get("payload") {
            None | Some(Value::Null) => json!({}),
            Some(payload @ Value::Object(_)) => payload.clone(),
            Some(_) => return Err(error("invalid_request", "payload must be an object")),
        };
        if serde_json::to_vec(&payload).map_or(true, |bytes| bytes.len() > MAX_PAYLOAD_BYTES) {
            return Err(error("payload_too_large", "payload exceeds 64 KiB"));
        }
        let subject = format!("plugin/{candidate}/{suffix}");
        for checked in [suffix, subject.as_str()] {
            validate_subject(checked, false).map_err(refused)?;
        }
        let bus = self
            .hooks_bus()
            .ok_or_else(|| error("host_api_unavailable", "this root has no event bus"))?;
        let receiver = bus
            .try_publish(
                "account",
                Draft::new(subject.clone(), format!("plugin:{candidate}"), payload),
            )
            .map_err(|e| error("host_busy", e.to_string()))?;
        let envelope = self
            .0
            .handle
            .block_on(async { tokio::time::timeout_at(deadline.into(), receiver).await })
            .map_err(|_| error("deadline_exceeded", "publication outcome unknown"))?
            .map_err(|_| error("host_busy", "publication outcome unknown"))?
            .map_err(|e| error("publish_refused", e.to_string()))?;
        Ok(json!({"id": envelope.event.id, "subject": subject}))
    }
}
