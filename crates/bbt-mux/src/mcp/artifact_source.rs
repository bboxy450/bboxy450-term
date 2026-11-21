// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
use crate::{
    Ulid,
    adapters::Binding,
    mcp::Call,
    store::{Result, digest, invalid},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

const SOURCE_MAX: usize = 1024 * 1024;
const RUNTIME_FRAME: usize = 8 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Offer {
    endpoint: PathBuf,
    execution_id: Ulid,
    nonce: Ulid,
}

pub(crate) struct Source {
    pub(crate) execution: Ulid,
    pub(crate) source: Box<str>,
}

fn canonical_digest(value: &Value) -> Result<String> {
    let mut canonical = value.clone();
    canonical.sort_all_objects();
    Ok(digest(&serde_json::to_vec(&canonical)?))
}

fn broker_endpoint(path: &Path) -> bool {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_prefix("bpy-"))
        .and_then(|id| id.parse::<Ulid>().ok())
        .is_some_and(|id| {
            id != Ulid::nil() && path == Path::new("/tmp").join(format!("bpy-{id}/m"))
        })
}

impl Offer {
    pub(crate) async fn resolve(
        self,
        binding: &Binding,
        call: &Call,
        context: &Value,
    ) -> Result<Source> {
        if !broker_endpoint(&self.endpoint)
            || self.execution_id == Ulid::nil()
            || self.nonce == Ulid::nil()
        {
            return Err(invalid(
                "execution_source_unavailable: invalid broker dispatch",
            ));
        }
        let exchange = async {
            // A session-owned endpoint is not identified by its name or same uid.
            // Check the kernel peer and process incarnation captured by the app.
            bboxy450_harness_wake_up::native::verify_process(binding)?;
            let mut stream = tokio::net::UnixStream::connect(&self.endpoint).await?;
            let peer = stream.peer_cred()?;
            if peer.pid().and_then(|pid| u32::try_from(pid).ok()) != Some(binding.pid)
                || peer.uid() != unsafe { libc::geteuid() }
            {
                return Err(invalid("broker is outside supervisor custody"));
            }
            let mut request = serde_json::to_vec(&json!({
                "jsonrpc":"2.0","id":1,"method":"python/artifact_source","params":{
                    "execution_id":self.execution_id,"nonce":self.nonce,"operation":call.op_id,
                    "tool":call.tool,"arguments_digest":canonical_digest(&call.args)?,
                    "context_digest":canonical_digest(context)?,
                }
            }))?;
            request.push(b'\n');
            stream.write_all(&request).await?;
            let mut frame = Vec::new();
            BufReader::new(stream)
                .take((RUNTIME_FRAME + 1) as u64)
                .read_until(b'\n', &mut frame)
                .await?;
            if !frame.ends_with(b"\n") || frame.len() > RUNTIME_FRAME {
                return Err(invalid("broker reply is incomplete or oversized"));
            }
            bboxy450_harness_wake_up::native::verify_process(binding)?;
            let mut reply: Value = serde_json::from_slice(&frame)?;
            if reply["id"] != 1 || reply["result"]["execution_id"] != json!(self.execution_id) {
                return Err(invalid("broker refused execution source"));
            }
            let Value::String(source) = reply["result"]["source"].take() else {
                return Err(invalid("broker source is absent"));
            };
            if !(1..=SOURCE_MAX).contains(&source.len()) {
                return Err(invalid("broker source exceeds its admitted length"));
            }
            Ok(Source {
                execution: self.execution_id,
                source: source.into_boxed_str(),
            })
        };
        tokio::time::timeout(Duration::from_secs(2), exchange)
            .await
            .map_err(|_| invalid("execution_source_unavailable: broker deadline exceeded"))?
            .map_err(|error| invalid(format!("execution_source_unavailable: {error}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_endpoint_accepts_only_the_bounded_private_wire_path() {
        let id = Ulid(9);
        assert!(broker_endpoint(&PathBuf::from(format!("/tmp/bpy-{id}/m"))));
        assert!(!broker_endpoint(&PathBuf::from(format!(
            "/private/tmp/bpy-{id}/m"
        ))));
        assert!(!broker_endpoint(&PathBuf::from(format!(
            "/tmp/bpy-{id}/../m"
        ))));
        assert!(!broker_endpoint(Path::new("/tmp/bpy-missing/m")));
    }
}
