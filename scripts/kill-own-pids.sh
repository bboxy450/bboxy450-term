#!/bin/sh
# Nothing in the product or its tests kills a process by name or pattern; a cleanup
# kills only PIDs it recorded as its own children (invariant kill-only-own-pids).
set -eu
cd "$(dirname "$0")/.."
hits=$(grep -rn -E '\b(pkill|killall)\b|pgrep[^|;]*\|[^|;]*xargs[^|;]*kill|kill[^;]*\$\(pgrep' crates scripts lab/web 2>/dev/null | grep -v -E '^scripts/kill-own-pids\.sh:' || true)
if [ -n "$hits" ]; then
    echo "kill-own-pids: a kill by name or pattern:" >&2
    printf '%s\n' "$hits" >&2
    exit 1
fi
echo "kill-own-pids: held"
