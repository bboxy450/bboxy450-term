//! Bounded newline JSON-RPC on stdio, forwarding to the private seat endpoint.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;

use serde_json::{Value, json};
use ulid::Ulid;

use super::Call;
use crate::seat_socket::{Action, Request};

pub const MAX_FRAME: usize = 1024 * 1024;
const VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26", "2024-11-05"];

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
impl Bridge {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            initialized: false,
        }
    }
    fn forward(&self, action: Action) -> io::Result<Value> {
        let mut stream = UnixStream::connect(&self.config.socket)?;
        stream.set_read_timeout(Some(Duration::from_secs(10)))?;
        stream.set_write_timeout(Some(Duration::from_secs(2)))?;
        let request = Request {
            version: 1,
            seat: self.config.seat,
            generation: self.config.generation,
            capability: self.config.capability,
            action,
        };
        let mut bytes = serde_json::to_vec(&request).map_err(io::Error::other)?;
        if bytes.len() + 1 > MAX_FRAME {
            return Err(io::Error::other("seat request exceeds bound"));
        }
        bytes.push(b'\n');
        stream.write_all(&bytes)?;
        let raw = read_frame(&mut BufReader::new(stream))?
            .ok_or_else(|| io::Error::other("seat response unavailable"))?;
        let value: Value =
            serde_json::from_slice(&raw).map_err(|_| io::Error::other("invalid seat response"))?;
        if value["kind"] == "Rejected" {
            return Err(io::Error::other("seat control request rejected"));
        }
        Ok(value)
    }

    pub fn handle(&mut self, bytes: &[u8]) -> Option<Value> {
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
                    "capabilities":{"tools":{"listChanged":false}},
                    "serverInfo":{"name":"bboxy450","version":env!("CARGO_PKG_VERSION")},
                    // Decision #260: a curious agent finding the binary in PATH is the common case.
                    "instructions":"These tools are your interface to bboxy450-term. `bboxy450-term ctl` is the operator's door, not yours: it refuses seats."}),
                )
            }
            "ping" => Ok(json!({})),
            "tools/list" | "tools/call" if !self.initialized => {
                return Some(rpc_error(id, -32002, "Initialize first"));
            }
            "tools/list" => {
                if params.get("cursor").is_some() {
                    return Some(rpc_error(id, -32602, "No catalogue cursor is available"));
                }
                self.forward(Action::Catalogue)
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
                    Err(_) => {
                        let mut response = rpc_error(
                            id,
                            -32001,
                            "Seat control transport unavailable; retain this operation ID before retry",
                        );
                        response["error"]["data"] = json!({"op_id":op_id,"status":"unknown"});
                        return Some(response);
                    }
                }
            }
            _ => return Some(rpc_error(id, -32601, "Method not found")),
        };
        Some(match result {
            Ok(result) => json!({"jsonrpc":"2.0","id":id,"result":result}),
            Err(_) => rpc_error(
                id,
                -32001,
                "Seat control transport unavailable; query the original operation before retry",
            ),
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
