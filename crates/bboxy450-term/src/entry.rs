// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
//! CLI entry points shared by the node and the rendered app.
mod qos;

#[cfg(feature = "desktop")]
include!("main.rs");

#[cfg(not(feature = "desktop"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    qos::startup(args.first().map_or("", String::as_str));
    if args == ["--version"] || args == ["-V"] {
        println!("bboxy450-term {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if !args
        .first()
        .is_some_and(|arg| matches!(arg.as_str(), "supervisor" | "logic" | "desktop"))
        && let Ok(root) = diagnostic_root(&args)
    {
        bbt_rt::error_journal::configure(&root);
    }
    service_entry(&args).unwrap_or_else(|| {
        let usage = bboxy450_term::cockpit::node::USAGE;
        if args.is_empty() || args == ["--help"] || args == ["-h"] {
            println!("{usage}");
            Ok(())
        } else {
            Err(usage.into())
        }
    })
}

fn diagnostic_root(args: &[String]) -> Result<std::path::PathBuf, String> {
    // Validate private custody before choosing any fallback. In particular an
    // inherited seat run must never win over an explicit throwaway account.
    let account =
        bboxy450_term::cockpit::Config::test_account_from_args(args).map_err(|e| e.to_string())?;
    if let Some(path) = args.windows(2).find(|p| p[0] == "--run-dir").map(|p| &p[1]) {
        return Ok(path.into());
    }
    if let Some(account) = account {
        return Ok(account.join("run"));
    }
    if let Some(path) = std::env::var_os("BBOXY_RUN_DIR") {
        return Ok(path.into());
    }
    bbt_mux::transport::default_socket_path()
        .map_err(|e| e.to_string())?
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or("default diagnostics directory unavailable".into())
}
fn service_entry(args: &[String]) -> Option<Result<(), Box<dyn std::error::Error>>> {
    let role = args.first()?.as_str();
    if let Some(result) = bboxy450_term::cockpit::private_entry(args) {
        if let Err(error) = &result {
            bbt_rt::error_journal::report(bbt_rt::error_journal::Failure::new(
                role,
                "process_entry",
                error,
            ));
        }
        return Some(result.map_err(|error| error as Box<dyn std::error::Error>));
    }
    let rest = &args[1..];
    Some(match role {
        "node" => bboxy450_term::cockpit::node::run(rest)
            .map_err(|error| error as Box<dyn std::error::Error>),
        "harness-exec" => bbt_mux::adapters::launch::exec_when_ready(rest)
            .map_err(|error| error as Box<dyn std::error::Error>),
        "hook" => exit_on_error(bbt_mux::adapters::hook::run_cli(rest)),
        "plugin" => exit_on_error(bbt_mux::plugins::run_cli(rest)),
        "mcp" => bbt_mux::mcp::run_cli()
            .inspect_err(|error| {
                bbt_rt::error_journal::report(bbt_rt::error_journal::Failure::new(
                    "mcp",
                    "bridge_exit",
                    error,
                ));
            })
            .map_err(Into::into),
        "kb-convert" => std::process::exit(bbt_mux::knowledge::den::run_cli(rest)),
        // The code-query MCP server is rooted at the seat cwd.
        "code" => code(),
        "ctl" => ctl(rest),
        _ => return None,
    })
}

fn exit_on_error(code: i32) -> Result<(), Box<dyn std::error::Error>> {
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

fn code() -> Result<(), Box<dyn std::error::Error>> {
    let root = bboxy450_code::Root::new(std::env::current_dir()?)?;
    bboxy450_code::serve(&root, std::io::stdin().lock(), std::io::stdout());
    Ok(())
}

fn ctl(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    let endpoint = diagnostic_root(&[])?.join(bbt_mux::ctl_door::ENDPOINT);
    if let Err(reason) = bboxy450_term::ctl::main(args, &endpoint) {
        println!("{}", serde_json::json!({"kind": "error", "reason": reason}));
        std::process::exit(1);
    }
    Ok(())
}
