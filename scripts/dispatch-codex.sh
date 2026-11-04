#!/bin/sh
# Run one headless Codex Executor on a brief, in its own worktree off origin/main.
#   scripts/dispatch-codex.sh BATCH BRIEF.md     (or: make dispatch BATCH=... BRIEF=...)
# Worktree ../<repo>-cx-BATCH on branch cx/BATCH; the prompt is scripts/dispatch-codex.md
# followed by BRIEF. Logs and the final reply stay outside Git in $DISPATCH_DIR.
# The Executor lands its own work through `make land` (see LAND.md); its target/ is
# removed when it exits, the branch and worktree are kept for the Oracle to verify.
set -u
batch=${1:?usage: dispatch-codex.sh BATCH BRIEF.md}
brief=${2:?usage: dispatch-codex.sh BATCH BRIEF.md}
repo=$(cd "$(dirname "$0")/.." && pwd -P)
model=${CODEX_MODEL:-gpt-6.1-sol}
effort=${CODEX_EFFORT:-xhigh}
dir=${DISPATCH_DIR:-$HOME/.local/state/bboxy450-term/dispatch}
worktree=$(dirname "$repo")/$(basename "$repo")-cx-$batch
log=$dir/$batch.log
[ -r "$brief" ] || { echo "dispatch: no brief $brief" >&2; exit 2; }
mkdir -p "$dir"
git -C "$repo" fetch -q origin main || exit 1
[ -d "$worktree" ] || git -C "$repo" worktree add -q -b "cx/$batch" "$worktree" origin/main || exit 1
prompt=$(cat "$repo/scripts/dispatch-codex.md"; echo; cat "$brief")
echo "start $(date) $batch $worktree $model/$effort" >> "$log"
codex exec -m "$model" -c model_reasoning_effort="$effort" \
  --dangerously-bypass-approvals-and-sandbox -C "$worktree" \
  -o "$dir/$batch.final.md" "$prompt" </dev/null >> "$log" 2>&1
rc=$?
echo "codex exit $rc $(date)" >> "$log"
rm -rf "$worktree/target"
df -h / | tail -1 >> "$log"
exit $rc
