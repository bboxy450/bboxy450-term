// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! `bboxy450-term-remote serve --state DIR`: run the remote supervisor relay on
//! stdin/stdout (the SSH provider runs it as `ssh HOST bboxy450-term-remote serve`).
use std::path::PathBuf;

fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let state = match args.as_slice() {
        [cmd, flag, dir] if cmd == "serve" && flag == "--state" => PathBuf::from(dir),
        _ => {
            eprintln!("usage: bboxy450-term-remote serve --state DIR");
            return std::process::ExitCode::from(2);
        }
    };
    let result = bboxy450_term_remote::serve::Server::open(&state)
        .and_then(|server| server.serve(std::io::stdin().lock(), std::io::stdout()));
    match result {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("bboxy450-term-remote: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}
