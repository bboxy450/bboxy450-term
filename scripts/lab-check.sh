#!/bin/sh
# Lab checks (renderer.md §4.4b): every lab/web page × density keeps text at true size (micro floor) and draws no
# browser-default chrome. Self-tests on lab/web/checks/bad.html first. Optional args: page names to limit the run.
set -eu
cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)"
exec node lab/web/checks/run.mjs "$@"
