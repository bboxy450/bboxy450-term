#!/bin/sh
# Copyright 2026 Boxy (bboxy450) and contributors.
# SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
# License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
# Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
# Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
# LicenseRef-Respectful-Slop-1.0-Draft; credit copied/adapted/translated reuse.
# Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
# Executed both directly and through the worktree-private Cargo/compiler shims.
set -eu
case "$(basename "$0")" in
    cargo) set -- cargo "$@" ;;
    rustc-qos) set -- compiler "$@" ;;
    rustdoc-qos) set -- rustdoc "$@" ;;
esac
role=$1; shift
case "$role" in
    make-*)
        target=${role#make-}
        role=default
        case "$target" in
            build*|compile|tools|lock|lock-online|bundle*|package-crate) role=background ;;
            test-*|check-*|smoke*) role=utility ;;
        esac
        # These gates measure real latency/CPU/frame budgets. Build children
        # still pass through the compiler shim. Keep the list in LAND.md.
        case "$target" in
            test-dream-c1|test-dream-c1-integration|test-perf-p1|test-perf-p2|bench-budgets|profile-ingestion|profile-input-ui|profile-render-input|test-input-latency|test-input-backend|test-terminal-streaming|test-codex-streaming|test-native-strip|test-seat-ingestion|test-formal-contracts|test-formal|test-qos|test-qos-app|test-landing|test-landing-extended|test-parity)
                role=default ;;
        esac
        # These explicit parents invoke the timed formal_contracts executable
        # or recurse into bench-budgets. Their ordinary Cargo leaves still use
        # utility; a utility ancestor would irreversibly clamp timed siblings.
        case "$target" in
            test-harness-native|test-terminal-replies|test-task-pages|test-task-panel-formal|test-jsonl-recovery|test-capture-storage-model|test-watcher-models|test-toolkit-models|test-knowledge-orientation-formal|test-provenance-background-formal|test-provenance-mcp-formal)
                role=default ;;
        esac
        set -- /bin/sh "$@" ;;
    cargo)
        exec python3 -I "${BBOXY_QOS_ROOT:?}/scripts/landing.py" qos-cargo-exec "$@" ;;
    compiler)
        role=background
        if [ -n "${BBOXY_INNER_RUSTC_WRAPPER:-}" ]; then
            set -- "$BBOXY_INNER_RUSTC_WRAPPER" "$@"
        fi ;;
    rustdoc)
        role=background
        for argument do
            if [ "$argument" = --test ]; then
                set -- "$@" --test-runtool "$BBOXY_QOS_ROOT/scripts/qos.sh" --test-runtool-arg utility
                break
            fi
        done
        set -- "${BBOXY_REAL_RUSTDOC:?}" "$@" ;;
    background|utility|default) ;;
    *) echo "qos: unknown role $role" >&2; exit 2 ;;
esac
if [ "$(uname -s)" = Darwin ]; then
    case "$role" in
        background) exec /usr/sbin/taskpolicy -b "$@" ;;
        utility|default)
            # Clear inherited Darwin background state in the calling process.
            # An external `taskpolicy -B -p` does not clear inherited state on
            # this macOS host; the native self call is process-witnessed below.
            # PRIO_DARWIN_PROCESS = 4 (Darwin sys/resource.h). Keep utility
            # clamps off parents of the explicitly named default-QoS gates.
            exec python3 -I -c '
import ctypes, os, sys
libc = ctypes.CDLL(None, use_errno=True)
if libc.setpriority(4, os.getpid(), 0) != 0:
    raise OSError(ctypes.get_errno(), "cannot clear inherited Darwin background policy")
role, *argv = sys.argv[1:]
if role == "utility":
    argv = ["/usr/sbin/taskpolicy", "-c", "utility", *argv]
os.execvp(argv[0], argv)
' "$role" "$@" ;;
    esac
fi
case "$role" in
    background|utility) exec nice -n 10 "$@" ;;
    default) exec "$@" ;;
esac
