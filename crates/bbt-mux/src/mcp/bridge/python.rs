// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! Session runtime proxy. Native admission brackets every forwarded result.
use super::*;

pub(super) fn handles(name: &str) -> bool {
    matches!(
        name,
        "python_execute"
            | "python_execution_get"
            | "python_execution_cancel"
            | "python_execute_orient"
    )
}
impl Bridge {
    pub(super) fn python_orient(&self, mut result: Value) -> Value {
        let Ok(runtime) = self.python_catalogue() else {
            return result;
        };
        let before = result["structuredContent"].clone();
        let Some(tools) = result["structuredContent"]["tools"].as_array_mut() else {
            return result;
        };
        for definition in runtime {
            if !tools.iter().any(|tool| tool["name"] == definition["name"]) {
                // Names only, as the host's own tools are listed.
                tools.push(json!({"name": definition["name"]}));
            }
        }
        let after = result["structuredContent"].clone();
        if let Some(content) = result["content"].as_array_mut() {
            for block in content {
                if block["text"]
                    .as_str()
                    .and_then(|s| serde_json::from_str::<Value>(s).ok())
                    .as_ref()
                    == Some(&before)
                {
                    block["text"] = json!(after.to_string());
                }
            }
        }
        result
    }
    fn python_request(&self, method: &str, params: Value) -> Result<Value, TransportFailure> {
        let started = Instant::now();
        let failure = |phase, error| TransportFailure::at(phase, started, error);
        let path = self.python_socket.as_ref().ok_or_else(|| {
            failure(
                "python_config",
                io::Error::new(io::ErrorKind::NotFound, "runtime unavailable"),
            )
        })?;
        let mut stream = UnixStream::connect(path).map_err(|e| failure("python_connect", e))?;
        // The runtime holds a call for up to 25 s while a run finishes
        // (python_execute wait_ms); reading must outlast that.
        stream
            .set_read_timeout(Some(Duration::from_secs(30)))
            .map_err(|e| failure("python_read_config", e))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(|e| failure("python_write_config", e))?;
        let mut bytes =
            serde_json::to_vec(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
                .map_err(|e| failure("python_encode", io::Error::other(e)))?;
        bytes.push(b'\n');
        stream
            .write_all(&bytes)
            .map_err(|e| failure("python_write", e))?;
        let bytes = read_frame(&mut BufReader::new(stream))
            .map_err(|e| failure("python_read", e))?
            .ok_or_else(|| failure("python_read", io::Error::from(io::ErrorKind::UnexpectedEof)))?;
        let reply: Value = serde_json::from_slice(&bytes)
            .map_err(|e| failure("python_decode", io::Error::other(e)))?;
        if reply.get("error").is_some() {
            return Err(failure(
                "python_result",
                io::Error::other("runtime protocol error"),
            ));
        }
        Ok(reply["result"].clone())
    }
    pub(super) fn python_catalogue(&self) -> Result<Vec<Value>, TransportFailure> {
        let value = self.python_request("tools/list", json!({}))?;
        Ok(value["tools"].as_array().cloned().unwrap_or_default())
    }
    fn python_context(&self) -> Result<Value, TransportFailure> {
        self.forward(Action::PythonContext)
    }
    pub(super) fn python_call(
        &self,
        call: &Call,
        expected: Option<&Value>,
    ) -> Result<Value, TransportFailure> {
        // Cancellation only retires this private runtime's exact owned child.
        // A revoked/missing provider must not prevent that cleanup obligation.
        if call.tool == "python_execution_cancel" {
            let mut arguments = call.args.clone();
            arguments["op_id"] = json!(call.op_id);
            return self.python_request(
                "python/proxy",
                json!({"name":call.tool,"arguments":arguments,"context":null}),
            );
        }
        let admitted = self.python_context()?;
        if admitted["isError"] == true {
            return Ok(admitted);
        }
        let context = admitted["structuredContent"]["context"].clone();
        if !context.is_object() || context["guard_version"] != 1 {
            return Ok(super::super::error(
                call.op_id,
                "native_context_unavailable",
                "Backend lacks the native execution guard; upgrade/restart the backend and reconnect this harness",
            ));
        }
        if expected
            .is_some_and(|expected| !super::super::python_context_matches(expected, &context))
        {
            return Ok(super::super::error(
                call.op_id,
                "execution_context_changed",
                "Captured Python context no longer matches native admission; no new execution was sent",
            ));
        }
        let mut arguments = call.args.clone();
        if matches!(
            call.tool.as_str(),
            "python_execute" | "python_execution_cancel"
        ) {
            arguments["op_id"] = json!(call.op_id);
        }
        // The runtime must not callback through this synchronous stdio stream
        // while it waits for this reply. We supply the native-admitted context.
        let result = self.python_request(
            "python/proxy",
            json!({"name":call.tool,"arguments":arguments,"context":context}),
        )?;
        let current = self.python_context()?;
        if current["isError"] == true || current["structuredContent"]["context"] != context {
            let mut refused = super::super::error(
                call.op_id,
                "execution_context_changed",
                "Context changed while Python replied; keep the execution ID and reconcile prior effects",
            );
            refused["structuredContent"]["execution_id"] =
                result["structuredContent"]["execution_id"].clone();
            refused["content"][0]["text"] = json!(refused["structuredContent"].to_string());
            return Ok(refused);
        }
        Ok(result)
    }
}
