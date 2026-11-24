#!/bin/sh
# Source after entering the worktree. Cache availability never gates compilation.
# bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; retain accurate source credit on reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
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
    export CARGO_BUILD_JOBS="${BBOXY_CARGO_JOBS:-2}" CARGO_INCREMENTAL=0
    # A machine-wide build.build-dir mixes worktrees' same-named crates
    # (rust-lang/cargo#12516) and serializes every build on one lock. Keep
    # intermediates with this worktree's target; kache shares compiled outputs.
    if [ -z "${CARGO_BUILD_BUILD_DIR:-}" ]; then
        CARGO_BUILD_BUILD_DIR=${BBOXY_CARGO_TARGET_DIR:-$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)/target}
        export CARGO_BUILD_BUILD_DIR
    fi
    # A full capacity-gated CI run cannot grow the optional shared cache. Do not
    # reconfigure or stop a live daemon: nested checks inherit this disposition.
    case "${BBOXY_CACHE_MODE:-auto}" in
        off)
            printf 'cache: off (full-gate disk headroom; no shared cache mutation)\n' >&2
            return 0 ;;
        auto) ;;
        *) printf 'cache: configuration refused (unknown BBOXY_CACHE_MODE)\n' >&2; return 2 ;;
    esac
    if ! command -v kache >/dev/null 2>&1; then
        printf 'cache: off (kache missing; run make tools)\n' >&2
        return 0
    fi
    if [ -z "${HOME:-}" ]; then
        printf 'cache: off (HOME unavailable)\n' >&2
        return 0
    fi
    # kache (https://github.com/kunobi-ninja/kache) wraps rustc: it caches
    # libraries, build scripts and linked executables, normalises worktree paths
    # so another worktree of the same source hits, and restores outputs as APFS
    # clones, so worktrees share disk. Enabled by environment only: `kache init`
    # would rewrite the user's global Cargo config. Local store only; no remote,
    # no account.
    case "$(uname -s)" in
        Darwin) KACHE_CACHE_DIR=${KACHE_CACHE_DIR:-$HOME/Library/Caches/bboxy450-kache} ;;
        *) KACHE_CACHE_DIR=${KACHE_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/bboxy450-kache} ;;
    esac
    KACHE_MAX_SIZE=${KACHE_MAX_SIZE:-20GiB}
    KACHE_LOCAL_ONLY=1
    # Cargo's job count (CARGO_BUILD_JOBS above) is the one concurrency limit.
    # kache's own scheduler sizes permits from the CPU count and halves them
    # under memory pressure: on this workspace it made a cold build 12.4 min
    # against 8.8 min without it, and a second worktree 5.5 against 4.2 min.
    KACHE_SCHEDULER=${KACHE_SCHEDULER:-0}
    # Compile in the wrapper, not the daemon: under load (2026-11-17, load 57-79)
    # daemon compiles dropped with "Broken pipe" and kache refused to rerun them
    # locally, failing build scripts and cc crates. Cache lookups still use it.
    KACHE_TRUST_DAEMON_EXECUTION=${KACHE_TRUST_DAEMON_EXECUTION:-0}
    export KACHE_CACHE_DIR KACHE_MAX_SIZE KACHE_LOCAL_ONLY KACHE_SCHEDULER KACHE_TRUST_DAEMON_EXECUTION
    # The first rustc call otherwise spawns the daemon with its own cwd, and a
    # daemon living in a managed checkout blocks workspace_release forever.
    # Start it from / first; when it already runs this returns in ~10 ms.
    (cd / && kache daemon start >/dev/null 2>&1) || true
    export RUSTC_WRAPPER=kache
    printf 'cache: on (kache; local store %s; %s)\n' "$KACHE_CACHE_DIR" "$KACHE_MAX_SIZE" >&2
    return 0
}
bboxy_cache_env || return $?
# Keep the shell itself unclamped: an explicitly timed process needs default
# QoS. Cargo/test trees and compiler/linker trees are classified independently.
bboxy_qos_root=$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)
# The real Cargo is never a shim that resolves `cargo` through PATH again: with
# kache's shim first on PATH it found target/qos-bin/cargo, which called the
# shim back until kache refused ("the chain is now 8 levels deep").
if [ -z "${BBOXY_REAL_CARGO:-}" ]; then
    for bboxy_cargo in "$HOME/.cargo/bin/cargo" $(which -a cargo 2>/dev/null); do
        case "$(readlink "$bboxy_cargo" 2>/dev/null)" in *kache*) continue ;; esac
        case "$bboxy_cargo" in */kache/*|*/qos-bin/*) continue ;; esac
        [ -x "$bboxy_cargo" ] && { BBOXY_REAL_CARGO=$bboxy_cargo; break; }
    done
    unset bboxy_cargo
fi
if [ -n "${BBOXY_REAL_CARGO:-}" ]; then
    KACHE_REAL_CARGO=${KACHE_REAL_CARGO:-$BBOXY_REAL_CARGO}
    export BBOXY_REAL_CARGO KACHE_REAL_CARGO
fi
# BBOXY_QOS=off runs Cargo and the compilers at normal priority: for a build
# the operator is waiting on (an install before a restart), not a background
# gate. `BBOXY_QOS=off make install` (Makefile:8 skips qos.sh too).
if [ "${BBOXY_QOS:-on}" != off ] && [ -f "$bboxy_qos_root/scripts/qos.sh" ]; then
    BBOXY_QOS_ROOT=$bboxy_qos_root
    export BBOXY_QOS_ROOT
    bboxy_qos_bin=${BBOXY_CARGO_TARGET_DIR:-$bboxy_qos_root/target}/qos-bin
    mkdir -p "$bboxy_qos_bin"
    ln -sf "$bboxy_qos_root/scripts/qos.sh" "$bboxy_qos_bin/cargo"
    ln -sf "$bboxy_qos_root/scripts/qos.sh" "$bboxy_qos_bin/rustc-qos"
    ln -sf "$bboxy_qos_root/scripts/qos.sh" "$bboxy_qos_bin/rustdoc-qos"
    ln -sf "$bboxy_qos_root/scripts/qos-policy.py" "$bboxy_qos_bin/bboxy-qos-test"
    BBOXY_REAL_RUSTDOC=${BBOXY_REAL_RUSTDOC:-${RUSTDOC:-$(command -v rustdoc)}}
    RUSTDOC=$bboxy_qos_bin/rustdoc-qos
    BBOXY_INNER_RUSTC_WRAPPER=$RUSTC_WRAPPER
    RUSTC_WRAPPER=$bboxy_qos_bin/rustc-qos
    export BBOXY_REAL_CARGO BBOXY_INNER_RUSTC_WRAPPER RUSTC_WRAPPER BBOXY_REAL_RUSTDOC RUSTDOC
    case ":$PATH:" in *":$bboxy_qos_bin:"*) ;; *) PATH=$bboxy_qos_bin:$PATH ;; esac
    export PATH
fi
unset -f bboxy_cache_env
unset bboxy_qos_root bboxy_qos_bin
