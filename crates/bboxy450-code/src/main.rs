// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-code`: the code queries as a stdio MCP server.
//!
//! One JSON-RPC message per line. The workspace root is `BBOXY_CODE_ROOT`, or the
//! directory the server starts in. `bboxy450-code <tool> '<json>'` runs one query
//! and prints its result, for scripts and for looking at what a tool returns.
use bboxy450_code::{Root, call, serve};
use serde_json::Value;

fn main() -> std::process::ExitCode {
    let root = std::env::var_os("BBOXY_CODE_ROOT")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .and_then(|path| Root::new(path).ok());
    let Some(root) = root else {
        eprintln!("bboxy450-code: the workspace root is not a readable directory");
        return std::process::ExitCode::from(2);
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [tool, input] = args.as_slice() {
        let outcome = serde_json::from_str::<Value>(input)
            .map_err(|e| format!("invalid JSON: {e}"))
            .and_then(|input| call(&root, tool, &input));
        return match outcome {
            Ok(value) => {
                println!("{value}");
                std::process::ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("bboxy450-code: {error}");
                std::process::ExitCode::FAILURE
            }
        };
    }
    if !args.is_empty() {
        eprintln!(
            "usage: bboxy450-code                 serve MCP on stdio\n       bboxy450-code <tool> '<json>'   run one query"
        );
        return std::process::ExitCode::from(2);
    }
    serve(&root, std::io::stdin().lock(), std::io::stdout());
    std::process::ExitCode::SUCCESS
}
