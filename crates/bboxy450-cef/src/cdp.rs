// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! A Chrome DevTools Protocol client with no I/O.
//!
//! [`Client::request`] numbers and encodes a call; whatever transport carries
//! the bytes hands every message back to [`Client::receive`], which matches a
//! reply to exactly one outstanding request. A reply nobody is waiting for
//! (duplicated, late after [`Client::cancel`], or never sent by us) comes back
//! as [`Incoming::Unmatched`] instead of being delivered twice.
//!
//! Protocol: https://chromedevtools.github.io/devtools-protocol/
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;

#[derive(Debug, Default)]
pub struct Client {
    next: u64,
    /// Outstanding request ids and their methods.
    pending: BTreeMap<u64, String>,
}

/// One encoded call, ready for the transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: u64,
    pub bytes: Vec<u8>,
}

/// A protocol error reply (`{"error": {"code", "message"}}`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// The one reply to request `id`.
    Reply { id: u64, method: String, outcome: Result<Value, Error> },
    /// A protocol event; `session` names the attached target, if any.
    Event { method: String, params: Value, session: Option<String> },
    /// A reply for no outstanding request.
    Unmatched { id: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    NotJson,
    /// Neither a reply (`id`) nor an event (`method`).
    Shape,
}

impl Client {
    pub fn new() -> Self {
        Self::default()
    }

    /// Encode `method(params)`, optionally on an attached target's session.
    pub fn request(&mut self, method: &str, params: Value, session: Option<&str>) -> Request {
        self.next += 1;
        let id = self.next;
        let mut message = json!({ "id": id, "method": method, "params": params });
        if let Some(session) = session {
            message["sessionId"] = Value::String(session.to_owned());
        }
        self.pending.insert(id, method.to_owned());
        Request { id, bytes: message.to_string().into_bytes() }
    }

    /// Stop waiting for `id`; its reply, if it ever arrives, is `Unmatched`.
    pub fn cancel(&mut self, id: u64) -> bool {
        self.pending.remove(&id).is_some()
    }

    /// Requests still waiting for a reply.
    pub fn pending(&self) -> usize {
        self.pending.len()
    }

    pub fn receive(&mut self, bytes: &[u8]) -> Result<Incoming, DecodeError> {
        let Value::Object(mut message) = serde_json::from_slice(bytes).map_err(|_| DecodeError::NotJson)? else {
            return Err(DecodeError::Shape);
        };
        if let Some(id) = message.get("id") {
            let id = id.as_u64().ok_or(DecodeError::Shape)?;
            let Some(method) = self.pending.remove(&id) else {
                return Ok(Incoming::Unmatched { id });
            };
            let outcome = match message.remove("error") {
                Some(error) => Err(Error {
                    code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                    message: error.get("message").and_then(Value::as_str).unwrap_or_default().to_owned(),
                }),
                None => Ok(message.remove("result").unwrap_or(Value::Object(Map::new()))),
            };
            return Ok(Incoming::Reply { id, method, outcome });
        }
        let Some(Value::String(method)) = message.remove("method") else {
            return Err(DecodeError::Shape);
        };
        let params = message.remove("params").unwrap_or(Value::Object(Map::new()));
        let session = match message.remove("sessionId") {
            Some(Value::String(s)) => Some(s),
            _ => None,
        };
        Ok(Incoming::Event { method, params, session })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reply(id: u64) -> Vec<u8> {
        json!({ "id": id, "result": { "n": id } }).to_string().into_bytes()
    }

    #[test]
    fn request_encodes_id_method_params_and_session() {
        let mut c = Client::new();
        let r = c.request("Runtime.evaluate", json!({ "expression": "1+1" }), Some("S1"));
        let v: Value = serde_json::from_slice(&r.bytes).unwrap();
        assert_eq!(v, json!({ "id": 1, "method": "Runtime.evaluate", "params": { "expression": "1+1" }, "sessionId": "S1" }));
        let r2 = c.request("Page.enable", json!({}), None);
        assert_eq!(r2.id, 2);
        assert!(serde_json::from_slice::<Value>(&r2.bytes).unwrap().get("sessionId").is_none());
        assert_eq!(c.pending(), 2);
    }

    #[test]
    fn replies_match_out_of_order_and_errors_carry_code() {
        let mut c = Client::new();
        let a = c.request("A.a", json!({}), None);
        let b = c.request("B.b", json!({}), None);
        let err = json!({ "id": b.id, "error": { "code": -32601, "message": "no such method" } });
        assert_eq!(
            c.receive(err.to_string().as_bytes()).unwrap(),
            Incoming::Reply { id: b.id, method: "B.b".into(), outcome: Err(Error { code: -32601, message: "no such method".into() }) }
        );
        assert_eq!(
            c.receive(&reply(a.id)).unwrap(),
            Incoming::Reply { id: a.id, method: "A.a".into(), outcome: Ok(json!({ "n": a.id })) }
        );
        assert_eq!(c.pending(), 0);
    }

    #[test]
    fn events_route_with_their_session() {
        let mut c = Client::new();
        let e = json!({ "method": "Page.loadEventFired", "params": { "timestamp": 1.5 }, "sessionId": "S9" });
        assert_eq!(
            c.receive(e.to_string().as_bytes()).unwrap(),
            Incoming::Event { method: "Page.loadEventFired".into(), params: json!({ "timestamp": 1.5 }), session: Some("S9".into()) }
        );
    }

    #[test]
    fn malformed_messages_are_refused() {
        let mut c = Client::new();
        assert_eq!(c.receive(b"not json"), Err(DecodeError::NotJson));
        assert_eq!(c.receive(b"[1]"), Err(DecodeError::Shape));
        assert_eq!(c.receive(br#"{"id":"x"}"#), Err(DecodeError::Shape));
        assert_eq!(c.receive(br#"{"params":{}}"#), Err(DecodeError::Shape));
    }

    /// Exhaustive small instance: three requests, one of them cancelled at any
    /// point, every reply delivered twice, in every order. Each request gets at
    /// most one Reply; each request not cancelled before its reply gets exactly
    /// one; everything else is Unmatched.
    #[test]
    fn every_order_of_duplicated_replies_delivers_each_reply_once() {
        // Deliveries: ids 1,2,3 each twice, plus a cancel marker 0 for id 2.
        let items = [1u64, 1, 2, 2, 3, 3, 0];
        let mut orders = Vec::new();
        permute(&mut items.to_vec(), 0, &mut orders);
        orders.sort();
        orders.dedup();
        for order in &orders {
            let mut c = Client::new();
            for m in ["A", "B", "C"] {
                c.request(m, json!({}), None);
            }
            let mut replies = [0u32; 4];
            let mut cancelled_first = false;
            for &step in order {
                if step == 0 {
                    cancelled_first = replies[2] == 0;
                    c.cancel(2);
                    continue;
                }
                match c.receive(&reply(step)).unwrap() {
                    Incoming::Reply { id, .. } => replies[id as usize] += 1,
                    Incoming::Unmatched { .. } => {}
                    other => panic!("unexpected {other:?}"),
                }
            }
            assert_eq!(replies[1], 1, "{order:?}");
            assert_eq!(replies[3], 1, "{order:?}");
            assert_eq!(replies[2], u32::from(!cancelled_first), "{order:?}");
            assert_eq!(c.pending(), 0, "{order:?}");
        }
        assert_eq!(orders.len(), 630); // 7! / (2!·2!·2!)
    }

    fn permute(items: &mut Vec<u64>, k: usize, out: &mut Vec<Vec<u64>>) {
        if k == items.len() {
            out.push(items.clone());
            return;
        }
        for i in k..items.len() {
            items.swap(k, i);
            permute(items, k + 1, out);
            items.swap(k, i);
        }
    }
}
