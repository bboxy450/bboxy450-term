#!/bin/sh
# Source after entering the worktree. Cache availability never gates compilation.
bboxy_cache_env() {
    export RUSTC_WRAPPER='' RUSTC_WORKSPACE_WRAPPER=''
    # Keep per-run paths and display settings out of the Rust cache key.
    if [ -n "${CARGO_TARGET_DIR:-}" ]; then
        BBOXY_CARGO_TARGET_DIR=$CARGO_TARGET_DIR
    fi
    if [ -n "${BBOXY_CARGO_TARGET_DIR:-}" ]; then
        case "$BBOXY_CARGO_TARGET_DIR" in
            /*) ;;
            *) BBOXY_CARGO_TARGET_DIR="$(pwd -P)/$BBOXY_CARGO_TARGET_DIR" ;;
        esac
        export BBOXY_CARGO_TARGET_DIR
    fi
    unset CARGO_TARGET_DIR CARGO_LOG CARGO_TERM_COLOR CARGO_TERM_PROGRESS_WHEN CARGO_TERM_PROGRESS_WIDTH
    export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
    if ! command -v sccache >/dev/null 2>&1; then
        printf 'cache: off (sccache missing)\n' >&2
        return 0
    fi
    if ! command -v python3 >/dev/null 2>&1; then
        printf 'cache: off (python3 missing; cannot bound server startup)\n' >&2
        return 0
    fi
    if ! bboxy_cache_root=$(git rev-parse --show-toplevel 2>/dev/null) || [ -z "${HOME:-}" ]; then
        printf 'cache: off (worktree or HOME unavailable)\n' >&2
        return 0
    fi
    bboxy_cache_root=$(CDPATH='' cd -- "$bboxy_cache_root" && pwd -P)
    SCCACHE_SERVER_PORT=$(printf '%s' "$bboxy_cache_root" | cksum | awk '{print 4300 + $1 % 1000}')
    SCCACHE_DIR="$HOME/Library/Caches/bboxy450-sccache"
    SCCACHE_CACHE_SIZE=50G
    SCCACHE_IGNORE_SERVER_IO_ERROR=1
    # An inherited Unix socket would override the worktree's TCP port.
    unset SCCACHE_SERVER_UDS
    export SCCACHE_SERVER_PORT SCCACHE_DIR SCCACHE_CACHE_SIZE SCCACHE_IGNORE_SERVER_IO_ERROR
    if bboxy_cache_reason=$(python3 - <<'PY'
import os
import socket
import subprocess
import sys
import time

deadline = time.monotonic() + 9.5
try:
    for action in ("--show-stats", "--start-server", "--show-stats"):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise subprocess.TimeoutExpired("sccache", 9.5)
        result = subprocess.run(
            ["sccache", action], capture_output=True, timeout=remaining,
        )
        if result.returncode == 0:
            # --show-stats can succeed with zero counters and no running server.
            try:
                with socket.create_connection(
                    ("127.0.0.1", int(os.environ["SCCACHE_SERVER_PORT"])),
                    timeout=max(0.001, min(0.25, deadline - time.monotonic())),
                ):
                    pass
            except OSError:
                continue
            sys.exit(0)
    print("server failed to start")
except subprocess.TimeoutExpired:
    print("server startup exceeded 10 s")
except OSError as error:
    print(f"server unavailable: {error.strerror}")
sys.exit(1)
PY
    ); then
        export RUSTC_WRAPPER=sccache
        (python3 -I "$bboxy_cache_root/scripts/resource-ledger.py" bind-cache "$bboxy_cache_root" "$SCCACHE_SERVER_PORT" </dev/null >/dev/null 2>&1 || :) &
        printf 'cache: on (port %s; shared local cache %s; 50G)\n' "$SCCACHE_SERVER_PORT" "$SCCACHE_DIR" >&2
    else
        printf 'cache: off (%s)\n' "${bboxy_cache_reason:-server check failed}" >&2
    fi
    return 0
}
bboxy_cache_env
unset -f bboxy_cache_env
unset bboxy_cache_root bboxy_cache_reason
