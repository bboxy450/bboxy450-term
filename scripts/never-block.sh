#!/bin/sh
# Never block the swarm (.bboxy/invariants/never-block-swarm.md). Operator, 2026-10-24:
# "test and e2e tests and smoke tests should not require MY INPUT, fully autonomous! NEVER BLOCK SWARM".
# Checkable half: no open task or brief is parked on a person. A task whose status is not
# closed and whose frontmatter or body says it waits on, is blocked on, needs or requires
# the operator, the Architect, the Oracle or a ruling fails this check. The test half is
# e2e-runs-unattended (#283), which runs the night with stdin closed and no operator.
set -eu
cd "$(dirname "$0")/.."
bad=0
for f in .bboxy/tasks/*.md; do
    case "$f" in */README.md) continue ;; esac   # the index is derived from the task records; the records are the truth
    status=$(awk '/^---$/{c++; next} c==1 && /^status:/{sub(/^status:[ ]*/,""); print; exit}' "$f" | tr -d '"')
    case "$status" in accepted|rejected|done|closed|retired|superseded) continue ;; esac
    if grep -qiE '(blocked on|blocked by|waits? (for|on)|waiting (for|on)|needs|requires) (the )?(operator|architect|oracle|a ruling|ruling)' "$f"; then
        echo "never-block: $f parks work on a person: $(grep -oiE '(blocked on|blocked by|waits? (for|on)|waiting (for|on)|needs|requires) (the )?(operator|architect|oracle|a ruling|ruling)' "$f" | head -1)"
        bad=1
    fi
done
echo "never-block: $([ "$bad" -eq 0 ] && echo held || echo VIOLATED)"
exit "$bad"
