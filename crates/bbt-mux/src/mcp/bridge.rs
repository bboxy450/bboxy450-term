// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Bounded newline JSON-RPC on stdio, forwarding to the private seat endpoint.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use ulid::Ulid;

use super::Call;
use crate::seat_socket::{Action, Request};

pub const MAX_FRAME: usize = 1024 * 1024;
/// Supported MCP wire revisions, newest first, shared by seat and plugin peers.
pub const VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

/// Intentionally not Debug: the admission capability is process-tree authority.
pub struct Config {
    pub socket: PathBuf,
    pub seat: Ulid,
    pub generation: u64,
    pub capability: [u8; 32],
}
impl Config {
    pub fn from_env() -> io::Result<Self> {
        let bad = || io::Error::other("missing or invalid BBOXY MCP admission environment");
        let socket = std::env::var_os("BBOXY_SOCK")
            .filter(|s| !s.is_empty())
            .ok_or_else(bad)?;
        let seat = std::env::var("BBOXY_SEAT_ID")
            .map_err(|_| bad())?
            .parse::<Ulid>()
            .map_err(|_| bad())?;
        let generation = std::env::var("BBOXY_GENERATION")
            .map_err(|_| bad())?
            .parse::<u64>()
            .map_err(|_| bad())?;
        let cap = std::env::var("BBOXY_CAPABILITY").map_err(|_| bad())?;
        if seat == Ulid::nil()
            || generation == 0
            || cap.len() != 64
            || !cap.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(bad());
        }
        let mut capability = [0; 32];
        for (i, byte) in capability.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&cap[i * 2..i * 2 + 2], 16).map_err(|_| bad())?;
        }
        Ok(Self {
            socket: socket.into(),
            seat,
            generation,
            capability,
        })
    }
}

/// Read a bounded frame before allocating its payload. A partial final frame is
/// rejected; JSON-RPC batches and Content-Length framing are not stdio MCP.
pub fn read_frame(reader: &mut impl BufRead) -> io::Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    loop {
        let part = reader.fill_buf()?;
        if part.is_empty() {
            return if bytes.is_empty() {
                Ok(None)
            } else {
                Err(io::Error::other("truncated MCP frame"))
            };
        }
        let n = part
            .iter()
            .position(|b| *b == b'\n')
            .map_or(part.len(), |n| n + 1);
        if bytes.len() + n > MAX_FRAME {
            return Err(io::Error::other("MCP frame exceeds 1 MiB"));
        }
        let done = part[n - 1] == b'\n';
        bytes.extend_from_slice(&part[..n]);
        reader.consume(n);
        if done {
            bytes.pop();
            return Ok(Some(bytes));
        }
    }
}

fn rpc_error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

/// The bridge does no role or tool dispatch. Its sole authority is the socket's
/// admitted seat/generation; every request is checked again by logic.
pub struct Bridge {
    config: Config,
    initialized: bool,
}
/// Only fixed phase names and OS categories cross the diagnostic boundary;
/// paths, request bytes and the admission capability never do.
struct TransportFailure {
    phase: &'static str,
    kind: io::ErrorKind,
    os: Option<i32>,
    elapsed_ms: u128,
}
impl TransportFailure {
    fn at(phase: &'static str, started: Instant, error: io::Error) -> Self {
        Self {
            phase,
            kind: error.kind(),
            os: error.raw_os_error(),
            elapsed_ms: started.elapsed().as_millis(),
        }
    }
    fn diagnostic(&self) -> Value {
        json!({"phase":self.phase,"kind":format!("{:?}",self.kind),
            "os":self.os,"elapsed_ms":self.elapsed_ms})
    }
}
fn diagnostic_failure(
    seat: Ulid,
    request: &Value,
    response: &Value,
) -> bbt_rt::error_journal::Failure {
    let tool = if request["method"] == "tools/call" {
        request["params"]["name"].as_str().unwrap_or("invalid_tool")
    } else {
        request["method"].as_str().unwrap_or("invalid_request")
    };
    let data = &response["result"]["structuredContent"];
    let op = data["op_id"]
        .as_str()
        .or_else(|| response["error"]["data"]["op_id"].as_str())
        .and_then(|id| id.parse::<Ulid>().ok())
        .filter(|op| *op != Ulid::nil());
    // Whitespace keeps the validated ULID separate from the tool name during
    // token redaction. No request arguments are copied into this observation.
    let operation = op.map_or_else(|| tool.to_owned(), |op| format!("{tool} {op}"));
    let reason = response["error"]["message"]
        .as_str()
        .or(data["error"]["message"].as_str())
        .or(response["result"]["reason"].as_str())
        .unwrap_or("tool returned an error or refusal");
    let reason = data["error"]["code"]
        .as_str()
        .map_or_else(|| reason.to_owned(), |code| format!("{code}: {reason}"));
    bbt_rt::error_journal::Failure::new("mcp", &operation, reason).seat(seat)
}
impl Bridge {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            initialized: false,
        }
    }
    fn forward(&self, action: Action) -> Result<Value, TransportFailure> {
        let started = Instant::now();
        let failure = |phase, error| TransportFailure::at(phase, started, error);
        let mut stream =
            UnixStream::connect(&self.config.socket).map_err(|e| failure("connect", e))?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .map_err(|e| failure("configure_read", e))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| failure("configure_write", e))?;
        let request = Request {
            version: 1,
            seat: self.config.seat,
            generation: self.config.generation,
            capability: self.config.capability,
            action,
        };
        let mut bytes = serde_json::to_vec(&request)
            .map_err(|e| failure("encode_request", io::Error::other(e)))?;
        if bytes.len() + 1 > MAX_FRAME {
            return Err(failure(
                "request_bound",
                io::Error::other("seat request exceeds bound"),
            ));
        }
        bytes.push(b'\n');
        stream
            .write_all(&bytes)
            .map_err(|e| failure("write_request", e))?;
        let raw = read_frame(&mut BufReader::new(stream))
            .map_err(|e| failure("read_response", e))?
            .ok_or_else(|| {
                failure(
                    "response_eof",
                    io::Error::other("seat response unavailable"),
                )
            })?;
        let value: Value = serde_json::from_slice(&raw)
            .map_err(|_| failure("decode_response", io::Error::other("invalid seat response")))?;
        if value["kind"] == "Rejected" {
            return Err(failure(
                "seat_rejected",
                io::Error::other("seat control request rejected"),
            ));
        }
        Ok(value)
    }

    pub fn handle(&mut self, bytes: &[u8]) -> Option<Value> {
        let reply = self.handle_inner(bytes);
        if let Some(response) = &reply
            && (response.get("error").is_some()
                || response["result"]["isError"] == true
                || response["result"]["kind"] == "Refused"
                || response["result"]["kind"] == "Rejected")
        {
            let request = serde_json::from_slice::<Value>(bytes).unwrap_or(Value::Null);
            bbt_rt::error_journal::report(diagnostic_failure(self.config.seat, &request, response));
        }
        reply
    }
    fn handle_inner(&mut self, bytes: &[u8]) -> Option<Value> {
        let value: Value = match serde_json::from_slice(bytes) {
            Ok(v) => v,
            Err(_) => return Some(rpc_error(Value::Null, -32700, "Parse error")),
        };
        if !value.is_object() || value["jsonrpc"] != "2.0" || !value["method"].is_string() {
            return Some(rpc_error(Value::Null, -32600, "Invalid Request"));
        }
        let method = value["method"].as_str().unwrap();
        let id = value.get("id").cloned()?;
        if !(id.is_string() || id.is_i64() || id.is_u64())
            || id.as_str().is_some_and(|s| s.len() > 128)
        {
            return Some(rpc_error(Value::Null, -32600, "Invalid request ID"));
        }
        let params = value.get("params").cloned().unwrap_or_else(|| json!({}));
        if !params.is_object() {
            return Some(rpc_error(id, -32602, "Params must be an object"));
        }
        let result = match method {
            "initialize" => {
                if self.initialized {
                    return Some(rpc_error(id, -32600, "Already initialized"));
                }
                let Some(version) = params["protocolVersion"].as_str() else {
                    return Some(rpc_error(id, -32602, "protocolVersion required"));
                };
                if !params["capabilities"].is_object() || !params["clientInfo"].is_object() {
                    return Some(rpc_error(
                        id,
                        -32602,
                        "Client capabilities and info required",
                    ));
                }
                self.initialized = true;
                Ok(
                    json!({"protocolVersion":if VERSIONS.contains(&version){version}else{VERSIONS[0]},
                    "capabilities":{"tools":{"listChanged":false},"resources":{"listChanged":false,"subscribe":false}},
                    "serverInfo":{"name":"bboxy450","version":env!("CARGO_PKG_VERSION")},
                    // Decision #260: a curious agent finding the binary in PATH is the common case.
                    "instructions":"These tools are your interface to bboxy450-term. Call orient first and use your role-filtered tools. Discover canonical workflow and current provider guidance with skill_list and skill_read, or MCP resources. Reading guidance grants no authority. Oracle orchestration uses seat_spawn, seat_split, seat_focus or the MCP ctl tool. The shell command `bboxy450-term ctl` is the operator's door and refuses seats."}),
                )
            }
            "ping" => Ok(json!({})),
            "tools/list" | "tools/call" | "resources/list" | "resources/read"
                if !self.initialized =>
            {
                return Some(rpc_error(id, -32002, "Initialize first"));
            }
            "tools/list" => {
                if params.get("cursor").is_some() {
                    return Some(rpc_error(id, -32602, "No catalogue cursor is available"));
                }
                self.forward(Action::Catalogue)
            }
            "resources/list" | "resources/read" => {
                if params.get("cursor").is_some() {
                    return Some(rpc_error(id, -32602, "No resource cursor is available"));
                }
                let (tool, args) = if method == "resources/read" {
                    let Some(uri) = params["uri"]
                        .as_str()
                        .filter(|uri| !uri.is_empty() && uri.len() <= super::skills::MAX_URI_BYTES)
                    else {
                        return Some(rpc_error(id, -32602, "A bounded skill URI is required"));
                    };
                    ("skill_read", json!({"uri":uri}))
                } else {
                    ("skill_list", json!({}))
                };
                // Use the very same admitted, root-scoped read as legacy tools.
                // No paths, provider processes or independent resource authority.
                match self.forward(Action::Call {
                    call: Call {
                        op_id: Ulid::new(),
                        tool: tool.into(),
                        args,
                    },
                }) {
                    Ok(reply) if reply["isError"] == true => {
                        let mut error = rpc_error(
                            id,
                            -32002,
                            "Skill resource is unavailable for the current seat and workspace",
                        );
                        error["error"]["data"] = reply["structuredContent"].clone();
                        return Some(error);
                    }
                    Ok(reply) => {
                        let data = &reply["structuredContent"];
                        if method == "resources/list" {
                            let resources = data["skills"].as_array().into_iter().flatten().map(|skill| json!({
                                "uri":skill["uri"],"name":skill["name"],"description":skill["description"],"mimeType":"text/markdown",
                                "_meta":{"bboxy450/skill":skill}
                            })).collect::<Vec<_>>();
                            Ok(json!({"resources":resources}))
                        } else {
                            let skill = &data["skill"];
                            Ok(
                                json!({"contents":[{"uri":skill["uri"],"mimeType":"text/markdown","text":skill["content"]}]}),
                            )
                        }
                    }
                    Err(error) => Err(error),
                }
            }
            "tools/call" => {
                let Some(tool) = params["name"]
                    .as_str()
                    .filter(|s| !s.is_empty() && s.len() <= 64)
                else {
                    return Some(rpc_error(id, -32602, "Invalid tool name"));
                };
                let mut args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                if !args.is_object() {
                    return Some(rpc_error(id, -32602, "Invalid or oversized arguments"));
                }
                let meta_id = match params.get("_meta").and_then(|m| m.get("bboxy450/op_id")) {
                    Some(value) => match value
                        .as_str()
                        .and_then(|s| s.parse::<Ulid>().ok())
                        .filter(|id| *id != Ulid::nil())
                    {
                        Some(id) => Some(id),
                        None => return Some(rpc_error(id, -32602, "Invalid bboxy450/op_id")),
                    },
                    None => None,
                };
                // The model can set tool arguments; native clients need not
                // expose custom request metadata to support an explicit retry.
                let argument_id = match args.as_object_mut().unwrap().remove("op_id") {
                    Some(value) => match value
                        .as_str()
                        .and_then(|s| s.parse::<Ulid>().ok())
                        .filter(|id| *id != Ulid::nil())
                    {
                        Some(id) => Some(id),
                        None => return Some(rpc_error(id, -32602, "Invalid op_id argument")),
                    },
                    None => None,
                };
                let op_id = match (argument_id, meta_id) {
                    (Some(a), Some(b)) if a != b => {
                        return Some(rpc_error(id, -32602, "op_id argument/_meta mismatch"));
                    }
                    (Some(value), _) | (_, Some(value)) => value,
                    (None, None) => Ulid::new(),
                };
                // The operation ID is retry metadata, not a tool argument and
                // must not make an otherwise bounded retry exceed MAX_ARGS.
                if !serde_json::to_vec(&args).is_ok_and(|b| b.len() <= super::MAX_ARGS) {
                    return Some(rpc_error(id, -32602, "Invalid or oversized arguments"));
                }
                let call = Call {
                    op_id,
                    tool: tool.into(),
                    args,
                };
                if call.validate().is_err() {
                    return Some(rpc_error(id, -32602, "Invalid control call"));
                }
                match self.forward(Action::Call { call }) {
                    Ok(result) => Ok(result),
                    Err(error) => {
                        let mut response = rpc_error(
                            id,
                            -32001,
                            "Seat control transport unavailable; retain this operation ID before retry",
                        );
                        response["error"]["data"] = json!({"op_id":op_id,"status":"unknown",
                            "transport":error.diagnostic()});
                        return Some(response);
                    }
                }
            }
            _ => return Some(rpc_error(id, -32601, "Method not found")),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(error) => {
                let mut response = rpc_error(
                    id,
                    -32001,
                    "Seat control transport unavailable; query the original operation before retry",
                );
                response["error"]["data"] = json!({"transport":error.diagnostic()});
                response
            }
        })
    }

    pub fn run(&mut self, reader: &mut impl BufRead, writer: &mut impl Write) -> io::Result<()> {
        while let Some(bytes) = read_frame(reader)? {
            if let Some(response) = self.handle(&bytes) {
                let mut bytes = serde_json::to_vec(&response).map_err(io::Error::other)?;
                if bytes.len() + 1 > MAX_FRAME {
                    return Err(io::Error::other("MCP response exceeds bound"));
                }
                bytes.push(b'\n');
                writer.write_all(&bytes)?;
                writer.flush()?;
            }
        }
        Ok(())
    }
}

pub fn run_cli() -> io::Result<()> {
    let config = Config::from_env()?;
    Bridge::new(config).run(&mut io::stdin().lock(), &mut io::stdout().lock())
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;
    #[test]
    fn diagnostic_producer_preserves_original_operation_and_class_without_arguments() {
        let op = Ulid::new();
        let request = json!({"method":"tools/call","params":{"name":"knowledge_fetch","arguments":{"locator":"raw-secret-sentinel"}}});
        let response = json!({"result":super::super::error(op,"number_conflict","knowledge lookup refused: number_conflict")});
        let failure = diagnostic_failure(Ulid::from(7_u128), &request, &response);
        assert_eq!(failure.operation, format!("knowledge_fetch {op}"));
        assert!(failure.message.starts_with("number_conflict:"));
        assert!(!failure.message.contains("raw-secret-sentinel"));
        assert_eq!(
            bbt_rt::error_journal::scrub(&failure.operation, 128),
            failure.operation
        );
        assert_eq!(failure.seat, Some(Ulid::from(7_u128).to_string()));
    }
}
