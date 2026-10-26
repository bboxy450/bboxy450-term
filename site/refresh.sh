#!/bin/sh
# Regenerate site/real.js from this machine's sessions (Oracle transcript + one Codex rollout per Executor), then build.
set -eu
cd "$(dirname "$0")/.."
C=$HOME/.codex/sessions/2026/10
python3 -I site/extract.py --claude "$HOME/.claude/projects/-Users-feral-mydev-bboxy450-term/3544d975-706b-4ce4-ba8c-d9734c0db856.jsonl" \
  --codex Nyx=$C/21/rollout-2026-10-21T10-58-24-01a15f9d-4223-7d02-a39e-759d2f662562.jsonl \
  --codex Rook=$C/21/rollout-2026-10-21T10-57-55-01a15f9c-d3e3-7980-acf0-af93c2269774.jsonl \
  --codex Sable=$C/22/rollout-2026-10-22T08-28-41-01a1643a-8d2d-7371-8e07-8b6a0b4fae8c.jsonl \
  --codex Aster=$C/22/rollout-2026-10-22T11-28-47-01a164df-7182-7a30-8dfa-e5f382c57f62.jsonl \
  --codex Kestrel=$C/22/rollout-2026-10-22T13-29-44-01a1654e-2c85-7f81-893e-175107b34597.jsonl \
  --codex Velvet=$C/22/rollout-2026-10-22T14-13-10-01a16575-f139-7f41-87aa-bf9accfe2094.jsonl
sh site/build.sh
